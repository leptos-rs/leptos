#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Provides functions to easily integrate Leptos with Actix.
//!
//! For more details on how to use the integrations, see the
//! [`examples`](https://github.com/leptos-rs/leptos/tree/main/examples)
//! directory in the Leptos repository.

use actix_files::NamedFile;
use actix_http::header::{HeaderName, HeaderValue, ACCEPT, LOCATION, REFERER};
use actix_web::{
    dev::{ServiceFactory, ServiceRequest},
    http::header,
    test,
    web::{Data, Payload, ServiceConfig},
    *,
};
use dashmap::DashMap;
use futures::{stream::once, Stream, StreamExt};
use http::StatusCode;
use hydration_context::SsrSharedContext;
use leptos::{
    config::LeptosOptions,
    context::{provide_context, use_context},
    prelude::expect_context,
    reactive::{computed::ScopedFuture, owner::Owner},
    IntoView,
};
use leptos_integration_utils::{
    BoxedFnOnce, ExtendResponse, PinnedFuture, PinnedStream,
};
use leptos_meta::ServerMetaContext;
use leptos_router::{
    components::provide_server_redirect,
    location::RequestUrl,
    static_routes::{RegenerationFn, ResolvedStaticPath},
    ExpandOptionals, Method, PathSegment, RouteList, RouteListing, SsrMode,
};
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use send_wrapper::SendWrapper;
use server_fn::{
    redirect::REDIRECT_HEADER, request::actix::ActixRequest, ServerFnError,
};
use std::{
    collections::HashSet,
    fmt::{Debug, Display},
    future::Future,
    ops::{Deref, DerefMut},
    path::Path,
    sync::Arc,
};

/// This struct lets you define headers and override the status of the Response from an Element or a Server Function
/// Typically contained inside of a ResponseOptions. Setting this is useful for cookies and custom responses.
#[derive(Debug, Clone, Default)]
pub struct ResponseParts {
    /// If provided, this will overwrite any other status code for this response.
    pub status: Option<StatusCode>,
    /// The map of headers that should be added to the response.
    pub headers: header::HeaderMap,
}

impl ResponseParts {
    /// Insert a header, overwriting any previous value with the same key
    pub fn insert_header(
        &mut self,
        key: header::HeaderName,
        value: header::HeaderValue,
    ) {
        self.headers.insert(key, value);
    }

    /// Append a header, leaving any header with the same key intact
    pub fn append_header(
        &mut self,
        key: header::HeaderName,
        value: header::HeaderValue,
    ) {
        self.headers.append(key, value);
    }
}

/// A wrapper for an Actix [`HttpRequest`] that allows it to be used in an
/// `Send`/`Sync` setting like Leptos's Context API.
#[derive(Debug, Clone)]
pub struct Request(SendWrapper<HttpRequest>);

impl Request {
    /// Wraps an existing Actix request.
    pub fn new(req: &HttpRequest) -> Self {
        Self(SendWrapper::new(req.clone()))
    }

    /// Consumes the wrapper and returns the inner Actix request.
    pub fn into_inner(self) -> HttpRequest {
        self.0.take()
    }
}

impl Deref for Request {
    type Target = HttpRequest;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Request {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Allows you to override details of the HTTP response like the status code and add Headers/Cookies.
#[derive(Debug, Clone, Default)]
pub struct ResponseOptions(pub Arc<RwLock<ResponseParts>>);

impl ResponseOptions {
    /// A simpler way to overwrite the contents of `ResponseOptions` with a new `ResponseParts`.
    pub fn overwrite(&self, parts: ResponseParts) {
        let mut writable = self.0.write();
        *writable = parts
    }
    /// Set the status of the returned Response.
    pub fn set_status(&self, status: StatusCode) {
        let mut writeable = self.0.write();
        let res_parts = &mut *writeable;
        res_parts.status = Some(status);
    }
    /// Insert a header, overwriting any previous value with the same key.
    pub fn insert_header(
        &self,
        key: header::HeaderName,
        value: header::HeaderValue,
    ) {
        let mut writeable = self.0.write();
        let res_parts = &mut *writeable;
        res_parts.headers.insert(key, value);
    }
    /// Append a header, leaving any header with the same key intact.
    pub fn append_header(
        &self,
        key: header::HeaderName,
        value: header::HeaderValue,
    ) {
        let mut writeable = self.0.write();
        let res_parts = &mut *writeable;
        res_parts.headers.append(key, value);
    }
}

struct ActixResponse(HttpResponse);

impl ExtendResponse for ActixResponse {
    type ResponseOptions = ResponseOptions;

    fn from_stream(
        stream: impl Stream<Item = String> + Send + 'static,
    ) -> Self {
        ActixResponse(
            HttpResponse::Ok()
                .content_type("text/html")
                .streaming(stream.map(|chunk| {
                    Ok(web::Bytes::from(chunk)) as Result<web::Bytes>
                })),
        )
    }

    fn extend_response(&mut self, res_options: &Self::ResponseOptions) {
        let mut res_options = res_options.0.write();

        let headers = self.0.headers_mut();
        for (key, value) in std::mem::take(&mut res_options.headers) {
            headers.append(key, value);
        }

        // Set status to what is returned in the function
        if let Some(status) = res_options.status {
            *self.0.status_mut() = status;
        }
    }

    fn set_default_content_type(&mut self, content_type: &str) {
        let headers = self.0.headers_mut();
        if !headers.contains_key(header::CONTENT_TYPE) {
            // Set the Content Type headers on all responses. This makes Firefox show the page source
            // without complaining
            headers.insert(
                header::CONTENT_TYPE,
                HeaderValue::from_str(content_type).unwrap(),
            );
        }
    }
}

/// Provides an easy way to redirect the user from within a server function.
///
/// Calling `redirect` in a server function will redirect the browser in three
/// situations:
/// 1. A server function that is calling in a [blocking
///    resource](leptos::server::Resource::new_blocking).
/// 2. A server function that is called from WASM running in the client (e.g., a dispatched action
///    or a spawned `Future`).
/// 3. A `<form>` submitted to the server function endpoint using default browser APIs (often due
///    to using [`ActionForm`](leptos::form::ActionForm) without JS/WASM present.)
///
/// Using it with a non-blocking [`Resource`](leptos::server::Resource) will not work if you are using streaming rendering,
/// as the response's headers will already have been sent by the time the server function calls `redirect()`.
///
/// ### Implementation
///
/// This sets the `Location` header to the URL given.
///
/// If the route or server function in which this is called is being accessed
/// by an ordinary `GET` request or an HTML `<form>` without any enhancement, it also sets a
/// status code of `302` for a temporary redirect. (This is determined by whether the `Accept`
/// header contains `text/html` as it does for an ordinary navigation.)
///
/// Otherwise, it sets a custom header that indicates to the client that it should redirect,
/// without actually setting the status code. This means that the client will not follow the
/// redirect, and can therefore return the value of the server function and then handle
/// the redirect with client-side routing.
#[cfg_attr(
    feature = "tracing",
    tracing::instrument(level = "trace", fields(error), skip_all)
)]
pub fn redirect(path: &str) {
    if let (Some(req), Some(res)) =
        (use_context::<Request>(), use_context::<ResponseOptions>())
    {
        // insert the Location header in any case
        res.insert_header(
            header::LOCATION,
            header::HeaderValue::from_str(path)
                .expect("Failed to create HeaderValue"),
        );

        let accepts_html = req
            .headers()
            .get(ACCEPT)
            .and_then(|v| v.to_str().ok())
            .map(|v| v.contains("text/html"))
            .unwrap_or(false);
        if accepts_html {
            // if the request accepts text/html, it's a plain form request and needs
            // to have the 302 code set
            res.set_status(StatusCode::FOUND);
        } else {
            // otherwise, we sent it from the server fn client and actually don't want
            // to set a real redirect, as this will break the ability to return data
            // instead, set the REDIRECT_HEADER to indicate that the client should redirect
            res.insert_header(
                HeaderName::from_static(REDIRECT_HEADER),
                HeaderValue::from_str("").unwrap(),
            );
        }
    } else {
        let msg = "Couldn't retrieve either Parts or ResponseOptions while \
                   trying to redirect().";

        #[cfg(feature = "tracing")]
        tracing::warn!("{}", &msg);

        #[cfg(not(feature = "tracing"))]
        eprintln!("{}", &msg);
    }
}

/// An Actix [struct@Route](actix_web::Route) that listens for a `POST` request with
/// Leptos server function arguments in the body, runs the server function if found,
/// and returns the resulting [HttpResponse].
///
/// This can then be set up at an appropriate route in your application:
///
/// ```no_run
/// use actix_web::*;
///
/// fn register_server_functions() {
///   // call ServerFn::register() for each of the server functions you've defined
/// }
///
/// #[actix_web::main]
/// async fn main() -> std::io::Result<()> {
///     // make sure you actually register your server functions
///     register_server_functions();
///
///     HttpServer::new(|| {
///         App::new()
///             // "/api" should match the prefix, if any, declared when defining server functions
///             // {tail:.*} passes the remainder of the URL as the server function name
///             .route("/api/{tail:.*}", leptos_actix::handle_server_fns())
///     })
///     .bind(("127.0.0.1", 8080))?
///     .run()
///     .await
/// }
/// ```
///
/// ## Provided Context Types
/// This function always provides context values including the following types:
/// - [ResponseOptions]
/// - [Request]
#[cfg_attr(
    feature = "tracing",
    tracing::instrument(level = "trace", fields(error), skip_all)
)]
pub fn handle_server_fns() -> Route {
    handle_server_fns_with_context(|| {})
}

/// An Actix [struct@Route](actix_web::Route) that listens for `GET` or `POST` requests with
/// Leptos server function arguments in the URL (`GET`) or body (`POST`),
/// runs the server function if found, and returns the resulting [HttpResponse].
///
/// This can then be set up at an appropriate route in your application:
///
/// This version allows you to pass in a closure that adds additional route data to the
/// context, allowing you to pass in info about the route or user from Actix, or other info.
///
/// **NOTE**: If your server functions expect a context, make sure to provide it both in
/// [`handle_server_fns_with_context`] **and** in [`LeptosRoutes::leptos_routes_with_context`] (or whatever
/// rendering method you are using). During SSR, server functions are called by the rendering
/// method, while subsequent calls from the client are handled by the server function handler.
/// The same context needs to be provided to both handlers.
///
/// ## Provided Context Types
/// This function always provides context values including the following types:
/// - [ResponseOptions]
/// - [Request]
#[cfg_attr(
    feature = "tracing",
    tracing::instrument(level = "trace", fields(error), skip_all)
)]
pub fn handle_server_fns_with_context(
    additional_context: impl Fn() + 'static + Clone + Send,
) -> Route {
    web::to(move |req: HttpRequest, payload: Payload| {
        let additional_context = additional_context.clone();
        async move {
            let additional_context = additional_context.clone();

            let path = req.path();
            let method = req.method();
            if let Some(mut service) =
                server_fn::actix::get_server_fn_service(path, method)
            {
                let owner = Owner::new();
                owner
                    .with(|| {
                        ScopedFuture::new(async move {
                            additional_context();
                            provide_context(Request::new(&req));
                            let res_options = ResponseOptions::default();
                            provide_context(res_options.clone());

                            // store Accepts and Referer in case we need them for redirect (below)
                            let accepts_html = req
                                .headers()
                                .get(ACCEPT)
                                .and_then(|v| v.to_str().ok())
                                .map(|v| v.contains("text/html"))
                                .unwrap_or(false);
                            let referrer = req.headers().get(REFERER).cloned();

                            // actually run the server fn
                            let mut res = ActixResponse(
                                service
                                    .0
                                    .run(ActixRequest::from((req, payload)))
                                    .await
                                    .take(),
                            );

                            // if it accepts text/html (i.e., is a plain form post) and doesn't already have a
                            // Location set, then redirect to the Referer
                            if accepts_html {
                                if let Some(referrer) = referrer {
                                    let has_location =
                                        res.0.headers().get(LOCATION).is_some();
                                    if !has_location {
                                        *res.0.status_mut() = StatusCode::FOUND;
                                        res.0
                                            .headers_mut()
                                            .insert(LOCATION, referrer);
                                    }
                                }
                            }

                            // the Location header may have been set to Referer, so any redirection by the
                            // user must overwrite it
                            {
                                let mut res_options = res_options.0.write();
                                let headers = res.0.headers_mut();

                                for location in
                                    res_options.headers.remove(header::LOCATION)
                                {
                                    headers.insert(header::LOCATION, location);
                                }
                            }

                            // apply status code and headers if user changed them
                            res.extend_response(&res_options);
                            res.0
                        })
                    })
                    .await
            } else {
                HttpResponse::BadRequest().body(format!(
                    "Could not find a server function at the route {:?}. \
                     \n\nIt's likely that either
                         1. The API prefix you specify in the `#[server]` \
                     macro doesn't match the prefix at which your server \
                     function handler is mounted, or \n2. You are on a \
                     platform that doesn't support automatic server function \
                     registration and you need to call \
                     ServerFn::register_explicit() on the server function \
                     type, somewhere in your `main` function.",
                    req.path()
                ))
            }
        }
    })
}

/// Returns an Actix [struct@Route](actix_web::Route) that listens for a `GET` request and tries
/// to route it using [leptos_router], serving an HTML stream of your application. The stream
/// will include fallback content for any `<Suspense/>` nodes, and be immediately interactive,
/// but requires some client-side JavaScript.
///
/// This can then be set up at an appropriate route in your application:
/// ```no_run
/// use actix_web::{App, HttpServer};
/// use leptos::prelude::*;
/// use leptos_router::Method;
/// use std::{env, net::SocketAddr};
///
/// #[component]
/// fn MyApp() -> impl IntoView {
///     view! { <main>"Hello, world!"</main> }
/// }
///
/// #[actix_web::main]
/// async fn main() -> std::io::Result<()> {
///     let conf = get_configuration(Some("Cargo.toml")).unwrap();
///     let addr = conf.leptos_options.site_addr.clone();
///     HttpServer::new(move || {
///         let leptos_options = &conf.leptos_options;
///
///         App::new()
///             // {tail:.*} passes the remainder of the URL as the route
///             // the actual routing will be handled by `leptos_router`
///             .route(
///                 "/{tail:.*}",
///                 leptos_actix::render_app_to_stream(MyApp, Method::Get),
///             )
///     })
///     .bind(&addr)?
///     .run()
///     .await
/// }
/// ```
///
/// ## Provided Context Types
/// This function always provides context values including the following types:
/// - [ResponseOptions]
/// - [Request]
/// - [MetaContext](leptos_meta::MetaContext)
#[cfg_attr(
    feature = "tracing",
    tracing::instrument(level = "trace", fields(error), skip_all)
)]
pub fn render_app_to_stream<IV>(
    app_fn: impl Fn() -> IV + Clone + Send + 'static,
    method: Method,
) -> Route
where
    IV: IntoView + 'static,
{
    render_app_to_stream_with_context(|| {}, app_fn, method)
}

/// Returns an Actix [struct@Route](actix_web::Route) that listens for a `GET` request and tries
/// to route it using [leptos_router], serving an in-order HTML stream of your application.
/// This stream will pause at each `<Suspense/>` node and wait for it to resolve before
/// sending down its HTML. The app will become interactive once it has fully loaded.
///
/// This can then be set up at an appropriate route in your application:
/// ```no_run
/// use actix_web::{App, HttpServer};
/// use leptos::prelude::*;
/// use leptos_router::Method;
/// use std::{env, net::SocketAddr};
///
/// #[component]
/// fn MyApp() -> impl IntoView {
///     view! { <main>"Hello, world!"</main> }
/// }
///
/// #[actix_web::main]
/// async fn main() -> std::io::Result<()> {
///     let conf = get_configuration(Some("Cargo.toml")).unwrap();
///     let addr = conf.leptos_options.site_addr.clone();
///     HttpServer::new(move || {
///         let leptos_options = &conf.leptos_options;
///
///         App::new()
///             // {tail:.*} passes the remainder of the URL as the route
///             // the actual routing will be handled by `leptos_router`
///             .route(
///                 "/{tail:.*}",
///                 leptos_actix::render_app_to_stream_in_order(
///                     MyApp,
///                     Method::Get,
///                 ),
///             )
///     })
///     .bind(&addr)?
///     .run()
///     .await
/// }
/// ```
///
/// ## Provided Context Types
/// This function always provides context values including the following types:
/// - [ResponseOptions]
/// - [Request]
#[cfg_attr(
    feature = "tracing",
    tracing::instrument(level = "trace", fields(error), skip_all)
)]
pub fn render_app_to_stream_in_order<IV>(
    app_fn: impl Fn() -> IV + Clone + Send + 'static,
    method: Method,
) -> Route
where
    IV: IntoView + 'static,
{
    render_app_to_stream_in_order_with_context(|| {}, app_fn, method)
}

/// Returns an Actix [struct@Route](actix_web::Route) that listens for a `GET` request and tries
/// to route it using [leptos_router], asynchronously rendering an HTML page after all
/// `async` resources have loaded.
///
/// This can then be set up at an appropriate route in your application:
/// ```no_run
/// use actix_web::{App, HttpServer};
/// use leptos::prelude::*;
/// use leptos_router::Method;
/// use std::{env, net::SocketAddr};
///
/// #[component]
/// fn MyApp() -> impl IntoView {
///     view! { <main>"Hello, world!"</main> }
/// }
///
/// #[actix_web::main]
/// async fn main() -> std::io::Result<()> {
///     let conf = get_configuration(Some("Cargo.toml")).unwrap();
///     let addr = conf.leptos_options.site_addr.clone();
///     HttpServer::new(move || {
///         let leptos_options = &conf.leptos_options;
///
///         App::new()
///             // {tail:.*} passes the remainder of the URL as the route
///             // the actual routing will be handled by `leptos_router`
///             .route(
///                 "/{tail:.*}",
///                 leptos_actix::render_app_async(MyApp, Method::Get),
///             )
///     })
///     .bind(&addr)?
///     .run()
///     .await
/// }
/// ```
///
/// ## Provided Context Types
/// This function always provides context values including the following types:
/// - [ResponseOptions]
/// - [Request]
#[cfg_attr(
    feature = "tracing",
    tracing::instrument(level = "trace", fields(error), skip_all)
)]
pub fn render_app_async<IV>(
    app_fn: impl Fn() -> IV + Clone + Send + 'static,
    method: Method,
) -> Route
where
    IV: IntoView + 'static,
{
    render_app_async_with_context(|| {}, app_fn, method)
}

/// Returns an Actix [struct@Route] that listens for a `GET` request and tries
/// to route it using [leptos_router], serving an HTML stream of your application.
///
/// This function allows you to provide additional information to Leptos for your route.
/// It could be used to pass in Path Info, Connection Info, or anything your heart desires.
///
/// ## Provided Context Types
/// This function always provides context values including the following types:
/// - [ResponseOptions]
/// - [Request]
#[cfg_attr(
    feature = "tracing",
    tracing::instrument(level = "trace", fields(error), skip_all)
)]
pub fn render_app_to_stream_with_context<IV>(
    additional_context: impl Fn() + 'static + Clone + Send,
    app_fn: impl Fn() -> IV + Clone + Send + 'static,
    method: Method,
) -> Route
where
    IV: IntoView + 'static,
{
    render_app_to_stream_with_context_and_replace_blocks(
        additional_context,
        app_fn,
        method,
        false,
    )
}

/// Returns an Actix [struct@Route](actix_web::Route) that listens for a `GET` request and tries
/// to route it using [leptos_router], serving an HTML stream of your application.
///
/// This function allows you to provide additional information to Leptos for your route.
/// It could be used to pass in Path Info, Connection Info, or anything your heart desires.
///
/// `replace_blocks` additionally lets you specify whether `<Suspense/>` fragments that read
/// from blocking resources should be retrojected into the HTML that's initially served, rather
/// than dynamically inserting them with JavaScript on the client. This means you will have
/// better support if JavaScript is not enabled, in exchange for a marginally slower response time.
///
/// ## Provided Context Types
/// This function always provides context values including the following types:
/// - [ResponseOptions]
/// - [Request]
#[cfg_attr(
    feature = "tracing",
    tracing::instrument(level = "trace", fields(error), skip_all)
)]
pub fn render_app_to_stream_with_context_and_replace_blocks<IV>(
    additional_context: impl Fn() + 'static + Clone + Send,
    app_fn: impl Fn() -> IV + Clone + Send + 'static,
    method: Method,
    replace_blocks: bool,
) -> Route
where
    IV: IntoView + 'static,
{
    _ = replace_blocks; // TODO
    handle_response(method, additional_context, app_fn, |app, chunks| {
        Box::pin(async move {
            Box::pin(app.to_html_stream_out_of_order().chain(chunks()))
                as PinnedStream<String>
        })
    })
}

/// Returns an Actix [struct@Route](actix_web::Route) that listens for a `GET` request and tries
/// to route it using [leptos_router], serving an in-order HTML stream of your application.
///
/// This function allows you to provide additional information to Leptos for your route.
/// It could be used to pass in Path Info, Connection Info, or anything your heart desires.
///
/// ## Provided Context Types
/// This function always provides context values including the following types:
/// - [ResponseOptions]
/// - [Request]
/// - [MetaContext](leptos_meta::MetaContext)
#[cfg_attr(
    feature = "tracing",
    tracing::instrument(level = "trace", fields(error), skip_all)
)]
pub fn render_app_to_stream_in_order_with_context<IV>(
    additional_context: impl Fn() + 'static + Clone + Send,
    app_fn: impl Fn() -> IV + Clone + Send + 'static,
    method: Method,
) -> Route
where
    IV: IntoView + 'static,
{
    handle_response(method, additional_context, app_fn, |app, chunks| {
        Box::pin(async move {
            Box::pin(app.to_html_stream_in_order().chain(chunks()))
                as PinnedStream<String>
        })
    })
}

/// Returns an Actix [struct@Route](actix_web::Route) that listens for a `GET` request and tries
/// to route it using [leptos_router], asynchronously serving the page once all `async`
/// resources have loaded.
///
/// This function allows you to provide additional information to Leptos for your route.
/// It could be used to pass in Path Info, Connection Info, or anything your heart desires.
///
/// ## Provided Context Types
/// This function always provides context values including the following types:
/// - [ResponseOptions]
/// - [Request]
#[cfg_attr(
    feature = "tracing",
    tracing::instrument(level = "trace", fields(error), skip_all)
)]
pub fn render_app_async_with_context<IV>(
    additional_context: impl Fn() + 'static + Clone + Send,
    app_fn: impl Fn() -> IV + Clone + Send + 'static,
    method: Method,
) -> Route
where
    IV: IntoView + 'static,
{
    handle_response(method, additional_context, app_fn, async_stream_builder)
}

fn async_stream_builder<IV>(
    app: IV,
    chunks: BoxedFnOnce<PinnedStream<String>>,
) -> PinnedFuture<PinnedStream<String>>
where
    IV: IntoView + 'static,
{
    Box::pin(async move {
        let app = if cfg!(feature = "dont-use-islands-router") {
            app.to_html_stream_in_order_branching()
        } else {
            app.to_html_stream_in_order()
        };
        let app = app.collect::<String>().await;
        let chunks = chunks();
        Box::pin(once(async move { app }).chain(chunks)) as PinnedStream<String>
    })
}

#[cfg_attr(
    feature = "tracing",
    tracing::instrument(level = "trace", fields(error), skip_all)
)]
fn provide_contexts(
    req: Request,
    meta_context: &ServerMetaContext,
    res_options: &ResponseOptions,
) {
    let path = leptos_corrected_path(&req);

    provide_context(RequestUrl::new(&path));
    provide_context(meta_context.clone());
    provide_context(res_options.clone());
    provide_context(req);
    provide_server_redirect(redirect);
    leptos::nonce::provide_nonce();
}

fn leptos_corrected_path(req: &HttpRequest) -> String {
    let path = req.path();
    let query = req.query_string();
    if query.is_empty() {
        "http://leptos".to_string() + path
    } else {
        "http://leptos".to_string() + path + "?" + query
    }
}

fn handle_response<IV>(
    method: Method,
    additional_context: impl Fn() + 'static + Clone + Send,
    app_fn: impl Fn() -> IV + Clone + Send + 'static,
    stream_builder: fn(
        IV,
        BoxedFnOnce<PinnedStream<String>>,
    ) -> PinnedFuture<PinnedStream<String>>,
) -> Route
where
    IV: IntoView + 'static,
{
    let handler = move |req: HttpRequest| {
        let app_fn = app_fn.clone();
        let add_context = additional_context.clone();

        async move {
            let res_options = ResponseOptions::default();
            let (meta_context, meta_output) = ServerMetaContext::new();

            let additional_context = {
                let meta_context = meta_context.clone();
                let res_options = res_options.clone();
                let req = Request::new(&req);
                move || {
                    provide_contexts(req, &meta_context, &res_options);
                    add_context();
                }
            };

            let res = ActixResponse::from_app(
                app_fn,
                meta_output,
                additional_context,
                res_options,
                stream_builder,
            )
            .await;

            res.0
        }
    };
    match method {
        Method::Get => web::get().to(handler),
        Method::Post => web::post().to(handler),
        Method::Put => web::put().to(handler),
        Method::Delete => web::delete().to(handler),
        Method::Patch => web::patch().to(handler),
    }
}

/// Generates a list of all routes defined in Leptos's Router in your app. We can then use this to automatically
/// create routes in Actix's App without having to use wildcard matching or fallbacks. Takes in your root app Element
/// as an argument so it can walk you app tree. This version is tailored to generated Actix compatible paths.
pub fn generate_route_list<IV>(
    app_fn: impl Fn() -> IV + 'static + Send + Clone,
) -> Vec<ActixRouteListing>
where
    IV: IntoView + 'static,
{
    generate_route_list_with_exclusions_and_ssg(app_fn, None).0
}

/// Generates a list of all routes defined in Leptos's Router in your app. We can then use this to automatically
/// create routes in Actix's App without having to use wildcard matching or fallbacks. Takes in your root app Element
/// as an argument so it can walk you app tree. This version is tailored to generated Actix compatible paths.
pub fn generate_route_list_with_ssg<IV>(
    app_fn: impl Fn() -> IV + 'static + Send + Clone,
) -> (Vec<ActixRouteListing>, StaticRouteGenerator)
where
    IV: IntoView + 'static,
{
    generate_route_list_with_exclusions_and_ssg(app_fn, None)
}

/// Generates a list of all routes defined in Leptos's Router in your app. We can then use this to automatically
/// create routes in Actix's App without having to use wildcard matching or fallbacks. Takes in your root app Element
/// as an argument so it can walk you app tree. This version is tailored to generated Actix compatible paths. Adding excluded_routes
/// to this function will stop `.leptos_routes()` from generating a route for it, allowing a custom handler. These need to be in Actix path format
pub fn generate_route_list_with_exclusions<IV>(
    app_fn: impl Fn() -> IV + 'static + Send + Clone,
    excluded_routes: Option<Vec<String>>,
) -> Vec<ActixRouteListing>
where
    IV: IntoView + 'static,
{
    generate_route_list_with_exclusions_and_ssg(app_fn, excluded_routes).0
}

/// Generates a list of all routes defined in Leptos's Router in your app. We can then use this to automatically
/// create routes in Actix's App without having to use wildcard matching or fallbacks. Takes in your root app Element
/// as an argument so it can walk you app tree. This version is tailored to generated Actix compatible paths. Adding excluded_routes
/// to this function will stop `.leptos_routes()` from generating a route for it, allowing a custom handler. These need to be in Actix path format
pub fn generate_route_list_with_exclusions_and_ssg<IV>(
    app_fn: impl Fn() -> IV + 'static + Send + Clone,
    excluded_routes: Option<Vec<String>>,
) -> (Vec<ActixRouteListing>, StaticRouteGenerator)
where
    IV: IntoView + 'static,
{
    generate_route_list_with_exclusions_and_ssg_and_context(
        app_fn,
        excluded_routes,
        || {},
    )
}

trait ActixPath {
    fn to_actix_path(&self) -> String;
}

impl ActixPath for Vec<PathSegment> {
    fn to_actix_path(&self) -> String {
        let mut path = String::new();
        for segment in self.iter() {
            // TODO trailing slash handling
            let raw = segment.as_raw_str();
            if !raw.is_empty() && !raw.starts_with('/') {
                path.push('/');
            }
            match segment {
                PathSegment::Static(s) => path.push_str(s),
                PathSegment::Param(s) => {
                    path.push('{');
                    path.push_str(s);
                    path.push('}');
                }
                PathSegment::Splat(s) => {
                    path.push('{');
                    path.push_str(s);
                    path.push_str(":.*}");
                }
                PathSegment::Unit => {}
                PathSegment::OptionalParam(_) => {
                    #[cfg(feature = "tracing")]
                    tracing::error!(
                        "to_axum_path should only be called on expanded \
                         paths, which do not have OptionalParam any longer"
                    );
                    Default::default()
                }
            }
        }
        path
    }
}

#[derive(Clone, Debug, Default)]
/// A route that this application can serve.
pub struct ActixRouteListing {
    path: String,
    mode: SsrMode,
    methods: Vec<leptos_router::Method>,
    regenerate: Vec<RegenerationFn>,
    exclude: bool,
}

trait IntoRouteListing: Sized {
    fn into_route_listing(self) -> Vec<ActixRouteListing>;
}

impl IntoRouteListing for RouteListing {
    fn into_route_listing(self) -> Vec<ActixRouteListing> {
        self.path()
            .to_vec()
            .expand_optionals()
            .into_iter()
            .map(|path| {
                let path = path.to_actix_path();
                let path = if path.is_empty() {
                    "/".to_string()
                } else {
                    path
                };
                let mode = self.mode();
                let methods = self.methods().collect();
                let regenerate = self.regenerate().into();
                ActixRouteListing {
                    path,
                    mode: mode.clone(),
                    methods,
                    regenerate,
                    exclude: false,
                }
            })
            .collect()
    }
}

impl ActixRouteListing {
    /// Create a route listing from its parts.
    pub fn new(
        path: String,
        mode: SsrMode,
        methods: impl IntoIterator<Item = leptos_router::Method>,
        regenerate: impl Into<Vec<RegenerationFn>>,
    ) -> Self {
        Self {
            path,
            mode,
            methods: methods.into_iter().collect(),
            regenerate: regenerate.into(),
            exclude: false,
        }
    }

    /// The path this route handles.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// The rendering mode for this path.
    pub fn mode(&self) -> SsrMode {
        self.mode.clone()
    }

    /// The HTTP request methods this path can handle.
    pub fn methods(&self) -> impl Iterator<Item = leptos_router::Method> + '_ {
        self.methods.iter().copied()
    }
}

/// Generates a list of all routes defined in Leptos's Router in your app. We can then use this to automatically
/// create routes in Actix's App without having to use wildcard matching or fallbacks. Takes in your root app Element
/// as an argument so it can walk you app tree. This version is tailored to generated Actix compatible paths. Adding excluded_routes
/// to this function will stop `.leptos_routes()` from generating a route for it, allowing a custom handler. These need to be in Actix path format.
/// Additional context will be provided to the app Element.
pub fn generate_route_list_with_exclusions_and_ssg_and_context<IV>(
    app_fn: impl Fn() -> IV + 'static + Send + Clone,
    excluded_routes: Option<Vec<String>>,
    additional_context: impl Fn() + 'static + Send + Clone,
) -> (Vec<ActixRouteListing>, StaticRouteGenerator)
where
    IV: IntoView + 'static,
{
    let _ = any_spawner::Executor::init_tokio();

    let owner = Owner::new_root(Some(Arc::new(SsrSharedContext::new())));
    let (mock_meta, _) = ServerMetaContext::new();
    let routes = owner
        .with(|| {
            // stub out a path for now
            provide_context(RequestUrl::new(""));
            provide_context(ResponseOptions::default());
            provide_context(mock_meta);
            additional_context();
            RouteList::generate(&app_fn)
        })
        .unwrap_or_default();

    let generator = StaticRouteGenerator::new(
        &routes,
        app_fn.clone(),
        additional_context.clone(),
    );

    // Axum's Router defines Root routes as "/" not ""
    let mut routes = routes
        .into_inner()
        .into_iter()
        .flat_map(IntoRouteListing::into_route_listing)
        .collect::<Vec<_>>();

    let routes = if routes.is_empty() {
        vec![ActixRouteListing::new(
            "/".to_string(),
            Default::default(),
            [leptos_router::Method::Get],
            vec![],
        )]
    } else {
        // Routes to exclude from auto generation
        if let Some(excluded_routes) = &excluded_routes {
            routes.retain(|p| !excluded_routes.iter().any(|e| e == p.path()))
        }
        routes
    };

    let excluded =
        excluded_routes
            .into_iter()
            .flatten()
            .map(|path| ActixRouteListing {
                path,
                mode: Default::default(),
                methods: Vec::new(),
                regenerate: Vec::new(),
                exclude: true,
            });

    (routes.into_iter().chain(excluded).collect(), generator)
}

/// Allows generating any prerendered routes.
#[allow(clippy::type_complexity)]
pub struct StaticRouteGenerator(
    Box<dyn FnOnce(&LeptosOptions) -> PinnedFuture<()> + Send>,
);

impl StaticRouteGenerator {
    fn render_route<IV: IntoView + 'static>(
        path: String,
        app_fn: impl Fn() -> IV + Clone + Send + 'static,
        additional_context: impl Fn() + Clone + Send + 'static,
    ) -> impl Future<Output = (Owner, String)> {
        let (meta_context, meta_output) = ServerMetaContext::new();
        let additional_context = {
            let add_context = additional_context.clone();
            move || {
                let mock_req = test::TestRequest::with_uri(&path)
                    .insert_header(("Accept", "text/html"))
                    .to_http_request();
                let res_options = ResponseOptions::default();
                provide_contexts(
                    Request::new(&mock_req),
                    &meta_context,
                    &res_options,
                );
                add_context();
            }
        };

        let (owner, stream) = leptos_integration_utils::build_response(
            app_fn.clone(),
            additional_context,
            async_stream_builder,
        );

        let sc = owner.shared_context().unwrap();

        async move {
            let stream = stream.await;
            while let Some(pending) = sc.await_deferred() {
                pending.await;
            }

            let html = meta_output
                .inject_meta_context(stream)
                .await
                .collect::<String>()
                .await;
            (owner, html)
        }
    }

    /// Creates a new static route generator from the given list of route definitions.
    pub fn new<IV>(
        routes: &RouteList,
        app_fn: impl Fn() -> IV + Clone + Send + 'static,
        additional_context: impl Fn() + Clone + Send + 'static,
    ) -> Self
    where
        IV: IntoView + 'static,
    {
        Self({
            let routes = routes.clone();
            Box::new(move |options| {
                let options = options.clone();
                let app_fn = app_fn.clone();
                let additional_context = additional_context.clone();

                Box::pin(routes.generate_static_files(
                    move |path: &ResolvedStaticPath| {
                        Self::render_route(
                            path.to_string(),
                            app_fn.clone(),
                            additional_context.clone(),
                        )
                    },
                    move |path: &ResolvedStaticPath,
                          owner: &Owner,
                          html: String| {
                        let options = options.clone();
                        let path = path.to_owned();
                        let response_options = owner.with(use_context);
                        async move {
                            write_static_route(
                                &options,
                                response_options,
                                path.as_ref(),
                                &html,
                            )
                            .await
                        }
                    },
                    was_404,
                ))
            })
        })
    }

    /// Generates the routes.
    pub async fn generate(self, options: &LeptosOptions) {
        (self.0)(options).await
    }
}

static STATIC_HEADERS: Lazy<DashMap<String, ResponseOptions>> =
    Lazy::new(DashMap::new);

fn was_404(owner: &Owner) -> bool {
    let resp = owner.with(|| expect_context::<ResponseOptions>());
    let status = resp.0.read().status;

    if let Some(status) = status {
        return status == StatusCode::NOT_FOUND;
    }

    false
}

fn static_path(options: &LeptosOptions, path: &str) -> String {
    use leptos_integration_utils::static_file_path;

    // If the path ends with a trailing slash, we generate the path
    // as a directory with a index.html file inside.
    if path != "/" && path.ends_with("/") {
        static_file_path(options, &format!("{}index", path))
    } else {
        static_file_path(options, path)
    }
}

async fn write_static_route(
    options: &LeptosOptions,
    response_options: Option<ResponseOptions>,
    path: &str,
    html: &str,
) -> Result<(), std::io::Error> {
    if let Some(options) = response_options {
        STATIC_HEADERS.insert(path.to_string(), options);
    }

    let path = static_path(options, path);
    let path = Path::new(&path);
    if let Some(path) = path.parent() {
        tokio::fs::create_dir_all(path).await?;
    }
    tokio::fs::write(path, &html).await?;

    Ok(())
}

fn handle_static_route<IV>(
    additional_context: impl Fn() + 'static + Clone + Send,
    app_fn: impl Fn() -> IV + Clone + Send + 'static,
    regenerate: Vec<RegenerationFn>,
) -> Route
where
    IV: IntoView + 'static,
{
    let handler = move |req: HttpRequest, data: Data<LeptosOptions>| {
        Box::pin({
            let app_fn = app_fn.clone();
            let additional_context = additional_context.clone();
            let regenerate = regenerate.clone();
            async move {
                let options = data.into_inner();
                let orig_path = req.uri().path();
                let path = static_path(&options, orig_path);
                let path = Path::new(&path);
                let exists = tokio::fs::try_exists(path).await.unwrap_or(false);

                let (response_options, html) = if !exists {
                    let path = ResolvedStaticPath::new(orig_path);

                    let (owner, html) = path
                        .build(
                            move |path: &ResolvedStaticPath| {
                                StaticRouteGenerator::render_route(
                                    path.to_string(),
                                    app_fn.clone(),
                                    additional_context.clone(),
                                )
                            },
                            move |path: &ResolvedStaticPath,
                                  owner: &Owner,
                                  html: String| {
                                let options = options.clone();
                                let path = path.to_owned();
                                let response_options = owner.with(use_context);
                                async move {
                                    write_static_route(
                                        &options,
                                        response_options,
                                        path.as_ref(),
                                        &html,
                                    )
                                    .await
                                }
                            },
                            was_404,
                            regenerate,
                        )
                        .await;
                    (owner.with(use_context::<ResponseOptions>), html)
                } else {
                    let headers =
                        STATIC_HEADERS.get(orig_path).map(|v| v.clone());
                    (headers, None)
                };

                // if html is Some(_), it means that `was_error_response` is true and we're not
                // actually going to cache this route, just return it as HTML
                //
                // this if for thing like 404s, where we do not want to cache an endless series of
                // typos (or malicious requests)
                let mut res = ActixResponse(match html {
                    Some(html) => {
                        HttpResponse::Ok().content_type("text/html").body(html)
                    }
                    None => match NamedFile::open(path) {
                        Ok(res) => res.into_response(&req),
                        Err(err) => HttpResponse::InternalServerError()
                            .body(err.to_string()),
                    },
                });

                if let Some(options) = response_options {
                    res.extend_response(&options);
                }

                res.0
            }
        })
    };
    web::get().to(handler)
}

/// This trait allows one to pass a list of routes and a render function to Actix's router, letting us avoid
/// having to use wildcards or manually define all routes in multiple places.
pub trait LeptosRoutes {
    /// Adds routes to the Axum router that have either
    /// 1) been generated by `leptos_router`, or
    /// 2) handle a server function.
    fn leptos_routes<IV>(
        self,
        paths: Vec<ActixRouteListing>,
        app_fn: impl Fn() -> IV + Clone + Send + 'static,
    ) -> Self
    where
        IV: IntoView + 'static;

    /// Adds routes to the Axum router that have either
    /// 1) been generated by `leptos_router`, or
    /// 2) handle a server function.
    ///
    /// Runs `additional_context` to provide additional data to the reactive system via context,
    /// when handling a route.
    fn leptos_routes_with_context<IV>(
        self,
        paths: Vec<ActixRouteListing>,
        additional_context: impl Fn() + 'static + Clone + Send,
        app_fn: impl Fn() -> IV + Clone + Send + 'static,
    ) -> Self
    where
        IV: IntoView + 'static;
}

/// The default implementation of `LeptosRoutes` which takes in a list of paths, and dispatches GET requests
/// to those paths to Leptos's renderer.
impl<T> LeptosRoutes for actix_web::App<T>
where
    T: ServiceFactory<
        ServiceRequest,
        Config = (),
        Error = Error,
        InitError = (),
    >,
{
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "trace", fields(error), skip_all)
    )]
    fn leptos_routes<IV>(
        self,
        paths: Vec<ActixRouteListing>,
        app_fn: impl Fn() -> IV + Clone + Send + 'static,
    ) -> Self
    where
        IV: IntoView + 'static,
    {
        self.leptos_routes_with_context(paths, || {}, app_fn)
    }

    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "trace", fields(error), skip_all)
    )]
    fn leptos_routes_with_context<IV>(
        self,
        paths: Vec<ActixRouteListing>,
        additional_context: impl Fn() + 'static + Clone + Send,
        app_fn: impl Fn() -> IV + Clone + Send + 'static,
    ) -> Self
    where
        IV: IntoView + 'static,
    {
        let mut router = self;

        let excluded = paths
            .iter()
            .filter(|&p| p.exclude)
            .map(|p| p.path.as_str())
            .collect::<HashSet<_>>();

        // register server functions first to allow for wildcard route in Leptos's Router
        for (path, _) in server_fn::actix::server_fn_paths() {
            if !excluded.contains(path) {
                let additional_context = additional_context.clone();
                let handler =
                    handle_server_fns_with_context(additional_context);
                router = router.route(path, handler);
            }
        }

        // register routes defined in Leptos's Router
        for listing in paths.iter().filter(|p| !p.exclude) {
            let path = listing.path();
            let mode = listing.mode();

            for method in listing.methods() {
                let additional_context = additional_context.clone();
                let additional_context_and_method = move || {
                    provide_context(method);
                    additional_context();
                };
                router = if matches!(listing.mode(), SsrMode::Static(_)) {
                    router.route(
                        path,
                        handle_static_route(
                            additional_context_and_method.clone(),
                            app_fn.clone(),
                            listing.regenerate.clone(),
                        ),
                    )
                } else {
                    router
                        .route(path, web::head().to(HttpResponse::Ok))
                        .route(
                            path,
                            match mode {
                                SsrMode::OutOfOrder => {
                                    render_app_to_stream_with_context(
                                        additional_context_and_method.clone(),
                                        app_fn.clone(),
                                        method,
                                    )
                                }
                                SsrMode::PartiallyBlocked => {
                                    render_app_to_stream_with_context_and_replace_blocks(
                                        additional_context_and_method.clone(),
                                        app_fn.clone(),
                                        method,
                                        true,
                                    )
                                }
                                SsrMode::InOrder => {
                                    render_app_to_stream_in_order_with_context(
                                        additional_context_and_method.clone(),
                                        app_fn.clone(),
                                        method,
                                    )
                                }
                                SsrMode::Async => render_app_async_with_context(
                                    additional_context_and_method.clone(),
                                    app_fn.clone(),
                                    method,
                                ),
                                _ => unreachable!()
                            },
                        )
                };
            }
        }

        router
    }
}

/// The default implementation of `LeptosRoutes` which takes in a list of paths, and dispatches GET requests
/// to those paths to Leptos's renderer.
impl LeptosRoutes for &mut ServiceConfig {
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "trace", fields(error), skip_all)
    )]
    fn leptos_routes<IV>(
        self,
        paths: Vec<ActixRouteListing>,
        app_fn: impl Fn() -> IV + Clone + Send + 'static,
    ) -> Self
    where
        IV: IntoView + 'static,
    {
        self.leptos_routes_with_context(paths, || {}, app_fn)
    }

    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "trace", fields(error), skip_all)
    )]
    fn leptos_routes_with_context<IV>(
        self,
        paths: Vec<ActixRouteListing>,
        additional_context: impl Fn() + 'static + Clone + Send,
        app_fn: impl Fn() -> IV + Clone + Send + 'static,
    ) -> Self
    where
        IV: IntoView + 'static,
    {
        let mut router = self;

        let excluded = paths
            .iter()
            .filter(|&p| p.exclude)
            .map(|p| p.path.as_str())
            .collect::<HashSet<_>>();

        // register server functions first to allow for wildcard route in Leptos's Router
        for (path, _) in server_fn::actix::server_fn_paths() {
            if !excluded.contains(path) {
                let additional_context = additional_context.clone();
                let handler =
                    handle_server_fns_with_context(additional_context);
                router = router.route(path, handler);
            }
        }

        // register routes defined in Leptos's Router
        for listing in paths.iter().filter(|p| !p.exclude) {
            let path = listing.path();
            let mode = listing.mode();

            for method in listing.methods() {
                if matches!(listing.mode(), SsrMode::Static(_)) {
                    router = router.route(
                        path,
                        handle_static_route(
                            additional_context.clone(),
                            app_fn.clone(),
                            listing.regenerate.clone(),
                        ),
                    )
                } else {
                    router = router.route(
                            path,
                            match mode {
                                SsrMode::OutOfOrder => {
                                    render_app_to_stream_with_context(
                                        additional_context.clone(),
                                        app_fn.clone(),
                                        method,
                                    )
                                }
                                SsrMode::PartiallyBlocked => {
                                    render_app_to_stream_with_context_and_replace_blocks(
                                        additional_context.clone(),
                                        app_fn.clone(),
                                        method,
                                        true,
                                    )
                                }
                                SsrMode::InOrder => {
                                    render_app_to_stream_in_order_with_context(
                                        additional_context.clone(),
                                        app_fn.clone(),
                                        method,
                                    )
                                }
                                SsrMode::Async => render_app_async_with_context(
                                    additional_context.clone(),
                                    app_fn.clone(),
                                    method,
                                ),
                                _ => unreachable!()
                            },
                        );
                }
            }
        }

        router
    }
}

/// A helper to make it easier to use Actix extractors in server functions.
///
/// It is generic over some type `T` that implements [`FromRequest`] and can
/// therefore be used in an extractor. The compiler can often infer this type.
///
/// Any error that occurs during extraction is converted to a [`ServerFnError`].
///
/// ```rust
/// use leptos::prelude::*;
///
/// #[server]
/// pub async fn extract_connection_info() -> Result<String, ServerFnError> {
///     use actix_web::dev::ConnectionInfo;
///     use leptos_actix::*;
///
///     // this can be any type you can use an Actix extractor with, as long as
///     // it works on the head, not the body of the request
///     let info: ConnectionInfo = extract().await?;
///
///     // do something with the data
///
///     Ok(format!("{info:?}"))
/// }
/// ```
pub async fn extract<T>() -> Result<T, ServerFnError>
where
    T: actix_web::FromRequest,
    <T as FromRequest>::Error: Display,
{
    let req = use_context::<Request>().ok_or_else(|| {
        ServerFnError::new("HttpRequest should have been provided via context")
    })?;

    SendWrapper::new(async move {
        T::extract(&req)
            .await
            .map_err(|e| ServerFnError::ServerError(e.to_string()))
    })
    .await
}
