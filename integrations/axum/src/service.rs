use crate::{handle_response_inner, PinnedStream};
use axum::{
    body::Body,
    http::{Request, Response, StatusCode},
};
use futures::{stream::once, Future, StreamExt};
use leptos::{config::LeptosOptions, context::provide_context, IntoView};
use std::{
    convert::Infallible,
    pin::Pin,
    task::{Context, Poll},
};
use tower::Service;

/// Service for serving error pages generated with the provided application shell.
///
/// This error handler is typically set up as a fallback service on some other services, such as the
/// Axum's Router set up with a Leptos app, and is provided as a tower [`Service`] to enable composition
/// with other tower services.
///
/// The behavior of [`file_and_error_handler`] can be approximately replicated with the following by
/// composing with the [`ServeDir`] service returned by [`site_pkg_dir_service`].
///
/// [`file_and_error_handler`]: crate::file_and_error_handler
/// [`site_pkg_dir_service`]: crate::site_pkg_dir_service
/// [`Service`]: tower::Service
/// [`ServeDir`]: tower_http::services::ServeDir
///
/// ```
/// # use axum::Router;
/// # use leptos::prelude::*;
/// # use leptos_axum::{LeptosRoutes, generate_route_list};
/// # #[component]
/// # fn App() -> impl IntoView {
/// #     view! { <main>"Hello, world!"</main> }
/// # }
/// # let conf = get_configuration(None).unwrap();
/// # let addr = conf.leptos_options.site_addr;
/// # let leptos_options = conf.leptos_options;
/// # let routes = generate_route_list(App);
/// fn shell(options: LeptosOptions) -> impl IntoView {
///     view! { <App/> }
/// }
///
/// let app = Router::new()
///     .leptos_routes(&leptos_options, routes, {
///         let leptos_options = leptos_options.clone();
///         move || shell(leptos_options.clone())
///     })
///     // the following `fallback_service(...)` call approximately replicates
///     // .fallback(leptos_axum::file_and_error_handler(shell))
///     .fallback_service(
///         leptos_axum::site_pkg_dir_service(&leptos_options).fallback(
///             leptos_axum::ErrorHandler::new(shell, leptos_options),
///         ),
///     );
/// ```
#[derive(Clone, Debug)]
pub struct ErrorHandler<CX, SH> {
    additional_context: CX,
    shell: SH,
    options: LeptosOptions,
}

impl<SH> ErrorHandler<(), SH> {
    /// Create a new handler with the provided shell and options.
    pub fn new(shell: SH, options: LeptosOptions) -> Self {
        Self {
            additional_context: (),
            shell,
            options,
        }
    }
}

impl<CX, SH> ErrorHandler<CX, SH> {
    /// Create a new handler with an additional context along with the provided shell and options.
    pub fn new_with_context(
        additional_context: CX,
        shell: SH,
        options: LeptosOptions,
    ) -> Self {
        Self {
            additional_context,
            shell,
            options,
        }
    }
}

impl<SH, IV> Service<Request<Body>> for ErrorHandler<(), SH>
where
    SH: Fn(LeptosOptions) -> IV + 'static + Clone + Send,
    IV: IntoView + 'static,
{
    type Response = Response<Body>;
    type Error = Infallible;
    type Future = Pin<
        Box<
            dyn Future<Output = Result<Response<Body>, Infallible>>
                + Send
                + 'static,
        >,
    >;

    #[inline]
    fn poll_ready(
        &mut self,
        _cx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let options = self.options.clone();
        let shell = self.shell.clone();
        render_error_handler(|| {}, shell, options, req)
    }
}

impl<CX, SH, IV> Service<Request<Body>> for ErrorHandler<CX, SH>
where
    CX: Fn() + 'static + Clone + Send,
    SH: Fn(LeptosOptions) -> IV + 'static + Clone + Send,
    IV: IntoView + 'static,
{
    type Response = Response<Body>;
    type Error = Infallible;
    type Future = Pin<
        Box<
            dyn Future<Output = Result<Response<Body>, Infallible>>
                + Send
                + 'static,
        >,
    >;

    #[inline]
    fn poll_ready(
        &mut self,
        _cx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let options = self.options.clone();
        let shell = self.shell.clone();
        let additional_context = self.additional_context.clone();
        render_error_handler(additional_context, shell, options, req)
    }
}

fn render_error_handler<IV>(
    additional_context: impl Fn() + 'static + Clone + Send,
    shell: impl Fn(LeptosOptions) -> IV + 'static + Clone + Send,
    options: LeptosOptions,
    req: Request<Body>,
) -> Pin<
    Box<
        dyn Future<Output = Result<Response<Body>, Infallible>>
            + Send
            + 'static,
    >,
>
where
    IV: IntoView + 'static,
{
    Box::pin(async move {
        let mut res = handle_response_inner(
            {
                let options = options.clone();
                let additional_context = additional_context.clone();
                move || {
                    provide_context(options.clone());
                    additional_context();
                }
            },
            {
                let options = options.clone();
                let shell = shell.clone();
                move || shell(options)
            },
            req,
            |app, chunks, _supports_ooo| {
                Box::pin(async move {
                    let app = if cfg!(feature = "islands-router") {
                        app.to_html_stream_in_order_branching()
                    } else {
                        app.to_html_stream_in_order()
                    };
                    let app = app.collect::<String>().await;
                    let chunks = chunks();
                    Box::pin(once(async move { app }).chain(chunks))
                        as PinnedStream<String>
                })
            },
        )
        .await;

        // set the status to 404
        // but if the status was already set (for example, to a 302 redirect) don't
        // overwrite it
        let status = res.status_mut();
        if *status == StatusCode::OK {
            *res.status_mut() = StatusCode::NOT_FOUND;
        }

        Ok(res)
    })
}
