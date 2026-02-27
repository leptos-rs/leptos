use crate::{
    extend_response, generate_request_and_parts, handle_response_inner,
    PinnedStream, ResponseOptions,
};
use axum::{
    body::Body,
    http::{Request, Response, StatusCode},
};
use futures::{stream::once, Future, StreamExt};
use leptos::{
    context::provide_context,
    reactive::{computed::ScopedFuture, owner::Owner},
    IntoView,
};
use std::{
    convert::Infallible,
    pin::Pin,
    task::{Context, Poll},
};
use tower::{Layer, Service};

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
/// # let leptos_options = conf.leptos_options;
/// # let routes = generate_route_list(App);
/// fn shell(options: LeptosOptions) -> impl IntoView {
///     view! {
///         <html>
///             <head>
///                 <HydrationScripts options/>
///             </head>
///             <body>
///                 <App/>
///             </body>
///         </html>
///     }
/// }
///
/// # #[cfg(feature = "default")]
/// let app = Router::new()
///     .leptos_routes(&leptos_options, routes, {
///         let leptos_options = leptos_options.clone();
///         move || shell(leptos_options.clone())
///     })
///     // the following `fallback_service(...)` call approximately replicates
///     // .fallback(leptos_axum::file_and_error_handler(shell))
///     .fallback_service(
///         // please do take note that both `file_and_error_handler` and
///         // `site_pkg_dir_service` require `feature = "default"`
///         leptos_axum::site_pkg_dir_service(&leptos_options).fallback(
///             leptos_axum::ErrorHandler::new(shell, leptos_options),
///         ),
///     );
/// ```
#[derive(Clone, Debug)]
pub struct ErrorHandler<CX, SH, S> {
    additional_context: Option<CX>,
    shell: SH,
    state: S,
}

impl<SH, S> ErrorHandler<fn(), SH, S> {
    /// Create a new handler with the provided shell and state.
    pub fn new(shell: SH, state: S) -> Self {
        Self {
            additional_context: None,
            shell,
            state,
        }
    }
}

impl<CX, SH, S> ErrorHandler<CX, SH, S> {
    /// Create a new handler with an additional context along with the provided shell and state.
    pub fn new_with_context(
        additional_context: CX,
        shell: SH,
        state: S,
    ) -> Self {
        Self {
            additional_context: Some(additional_context),
            shell,
            state,
        }
    }
}

impl<CX, SH, S, IV> Service<Request<Body>> for ErrorHandler<CX, SH, S>
where
    CX: Fn() + 'static + Clone + Send,
    SH: Fn(S) -> IV + 'static + Clone + Send,
    S: Clone + Send + Sync + 'static,
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
        let state = self.state.clone();
        let shell = self.shell.clone();
        let additional_context = self.additional_context.clone();
        render_error_handler(additional_context, shell, state, req)
    }
}

fn render_error_handler<IV, S>(
    additional_context: Option<impl Fn() + 'static + Clone + Send>,
    shell: impl Fn(S) -> IV + 'static + Clone + Send,
    state: S,
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
    S: Send + Sync + Clone + 'static,
{
    Box::pin(async move {
        let mut res = handle_response_inner(
            {
                let state = state.clone();
                let additional_context = additional_context.clone();
                move || {
                    provide_context(state.clone());
                    if let Some(additional_context) = &additional_context {
                        additional_context();
                    }
                }
            },
            {
                let state = state.clone();
                let shell = shell.clone();
                move || shell(state)
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

/// Layer for providing the `LeptosContext` middleware.
#[derive(Clone, Debug)]
pub struct LeptosContextLayer<CX> {
    additional_context: Option<CX>,
}

impl Default for LeptosContextLayer<fn()> {
    fn default() -> Self {
        Self::new()
    }
}

impl LeptosContextLayer<fn()> {
    /// Create a new layer with the additional context to be provided.
    pub fn new() -> Self {
        Self {
            additional_context: None,
        }
    }
}

impl<CX> LeptosContextLayer<CX> {
    /// Create a new layer with the additional context to be provided.
    pub fn new_with_context(additional_context: CX) -> Self
    where
        CX: Fn() + 'static + Clone + Send,
    {
        Self {
            additional_context: Some(additional_context),
        }
    }
}

impl<S, CX> Layer<S> for LeptosContextLayer<CX>
where
    CX: Clone,
{
    type Service = LeptosContext<S, CX>;

    fn layer(&self, service: S) -> Self::Service {
        LeptosContext::new(service, self.additional_context.clone())
    }
}

/// Middleware for applying additional contexts to other tower/axum services.
#[derive(Clone, Debug)]
pub struct LeptosContext<S, CX> {
    inner: S,
    owner: Owner,
    additional_context: Option<CX>,
}

impl<S, CX> LeptosContext<S, CX> {
    /// Create a new handler with an additional context along with the provided shell and state.
    pub fn new(inner: S, additional_context: Option<CX>) -> Self {
        let owner = Owner::new();
        Self {
            inner,
            owner,
            additional_context,
        }
    }
}

impl<ReqBody, ResBody, S, CX> Service<Request<ReqBody>> for LeptosContext<S, CX>
where
    CX: Fn() + 'static + Clone + Send,
    S: Service<Request<ReqBody>, Response = Response<ResBody>>
        + Clone
        + Send
        + 'static,
    S::Future: Send,
    ReqBody: Send + 'static,
    ResBody: Default + Send,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<
        Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    #[inline]
    fn poll_ready(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        // Because the inner service can panic until ready, we need to ensure we only
        // use the ready service.
        //
        // See: https://docs.rs/tower/latest/tower/trait.Service.html#be-careful-when-cloning-inner-services
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);
        let (req, parts) = generate_request_and_parts(req);
        let additional_context = self.additional_context.clone();

        self.owner.with(|| {
            Box::pin(ScopedFuture::new(async move {
                provide_context(parts);
                let res_options = ResponseOptions::default();
                provide_context(res_options.clone());
                if let Some(additional_context) = additional_context {
                    additional_context();
                }
                let mut res = inner.call(req).await?;
                extend_response(&mut res, &res_options);
                Ok(res)
            }))
        })
    }
}
