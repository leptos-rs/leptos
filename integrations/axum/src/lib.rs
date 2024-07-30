#![forbid(unsafe_code)]
//! Provides functions to easily integrate Leptos with Axum.
//!
//! ## JS Fetch Integration
//! The `leptos_axum` integration supports running in JavaScript-hosted WebAssembly
//! runtimes, e.g., running inside Deno, Cloudflare Workers, or other JS environments.
//! To run in this environment, you need to disable the default feature set and enable
//! the `wasm` feature on `leptos_axum` in your `Cargo.toml`.
//! ```toml
//! leptos_axum = { version = "0.6.0", default-features = false, features = ["wasm"] }
//! ```
//!
//! ## Features
//! - `default`: supports running in a typical native Tokio/Axum environment
//! - `wasm`: with `default-features = false`, supports running in a JS Fetch-based
//!   environment
//! - `experimental-islands`: activates Leptos [islands mode](https://leptos-rs.github.io/leptos/islands.html)
//!
//! ### Important Note
//! Prior to 0.5, using `default-features = false` on `leptos_axum` simply did nothing. Now, it actively
//! disables features necessary to support the normal native/Tokio runtime environment we create. This can
//! generate errors like the following, which don’t point to an obvious culprit:
//! `
//! `spawn_local` called from outside of a `task::LocalSet`
//! `
//! If you are not using the `wasm` feature, do not set `default-features = false` on this package.
//!
//!
//! ## More information
//!
//! For more details on how to use the integrations, see the
//! [`examples`](https://github.com/leptos-rs/leptos/tree/main/examples)
//! directory in the Leptos repository.

use axum::{
    body::{Body, Bytes},
    extract::{FromRef, FromRequestParts, MatchedPath},
    http::{
        header::{self, HeaderName, HeaderValue, ACCEPT, LOCATION, REFERER},
        request::Parts,
        HeaderMap, Method, Request, Response, StatusCode,
    },
    response::{Html, IntoResponse},
    routing::{delete, get, patch, post, put},
};
use futures::{
    channel::mpsc::{Receiver, Sender},
    stream::once,
    Future, FutureExt, SinkExt, Stream, StreamExt,
};
use hydration_context::SsrSharedContext;
use leptos::{
    config::LeptosOptions,
    context::{provide_context, use_context},
    reactive_graph::{computed::ScopedFuture, owner::Owner},
    tachys::ssr::StreamBuilder,
    IntoView,
};
use leptos_meta::{MetaContext, ServerMetaContext};
use leptos_router::{
    location::RequestUrl, PathSegment, RouteList, RouteListing, SsrMode,
    StaticDataMap, StaticMode,
};
use once_cell::sync::OnceCell;
use parking_lot::RwLock;
use reactive_graph::owner::Sandboxed;
use server_fn::{
    
    error::{NoCustomError, ServerFnErrorSerde},
    redirect::REDIRECT_HEADER, ServerFnError,
,
};
use std::{
    collections::HashSet,
    fmt::{Debug, Write},
    io,
    pin::Pin,
    sync::Arc,
    thread::available_parallelism,
};
use tracing::Instrument;

/// This struct lets you define headers and override the status of the Response from an Element or a Server Function
/// Typically contained inside of a ResponseOptions. Setting this is useful for cookies and custom responses.
#[derive(Debug, Clone, Default)]
pub struct ResponseParts {
    pub status: Option<StatusCode>,
    pub headers: HeaderMap,
}

impl ResponseParts {
    /// Insert a header, overwriting any previous value with the same key
    pub fn insert_header(&mut self, key: HeaderName, value: HeaderValue) {
        self.headers.insert(key, value);
    }
    /// Append a header, leaving any header with the same key intact
    pub fn append_header(&mut self, key: HeaderName, value: HeaderValue) {
        self.headers.append(key, value);
    }
}

/// Allows you to override details of the HTTP response like the status code and add Headers/Cookies.
///
/// `ResponseOptions` is provided via context when you use most of the handlers provided in this
/// crate, including [`.leptos_routes`](LeptosRoutes::leptos_routes),
/// [`.leptos_routes_with_context`](LeptosRoutes::leptos_routes_with_context), [`handle_server_fns`], etc.
/// You can find the full set of provided context types in each handler function.
///
/// If you provide your own handler, you will need to provide `ResponseOptions` via context
/// yourself if you want to access it via context.
/// ```rust,ignore
/// #[server]
/// pub async fn get_opts() -> Result<(), ServerFnError> {
///     let opts = expect_context::<leptos_axum::ResponseOptions>();
///     Ok(())
/// }
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
    pub fn insert_header(&self, key: HeaderName, value: HeaderValue) {
        let mut writeable = self.0.write();
        let res_parts = &mut *writeable;
        res_parts.headers.insert(key, value);
    }
    /// Append a header, leaving any header with the same key intact.
    pub fn append_header(&self, key: HeaderName, value: HeaderValue) {
        let mut writeable = self.0.write();
        let res_parts = &mut *writeable;
        res_parts.headers.append(key, value);
    }
}

/// Provides an easy way to redirect the user from within a server function. Mimicking the Remix `redirect()`,
/// it sets a StatusCode of 302 and a LOCATION header with the provided value.
/// If looking to redirect from the client, `leptos_router::use_navigate()` should be used instead
pub fn redirect(path: &str) {
    if let (Some(req), Some(res)) =
        (use_context::<Parts>(), use_context::<ResponseOptions>())
    {
        // insert the Location header in any case
        res.insert_header(
            header::LOCATION,
            header::HeaderValue::from_str(path)
                .expect("Failed to create HeaderValue"),
        );

        let accepts_html = req
            .headers
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
        tracing::warn!(
            "Couldn't retrieve either Parts or ResponseOptions while trying \
             to redirect()."
        );
    }
}

/// Decomposes an HTTP request into its parts, allowing you to read its headers
/// and other data without consuming the body. Creates a new Request from the
/// original parts for further processing
pub fn generate_request_and_parts(
    req: Request<Body>,
) -> (Request<Body>, Parts) {
    let (parts, body) = req.into_parts();
    let parts2 = parts.clone();
    (Request::from_parts(parts, body), parts2)
}

/// An Axum handlers to listens for a request with Leptos server function arguments in the body,
/// run the server function if found, and return the resulting [`Response`].
///
/// This can then be set up at an appropriate route in your application:
///
/// ```
/// use axum::{handler::Handler, routing::post, Router};
/// use leptos::*;
/// use std::net::SocketAddr;
///
/// # if false { // don't actually try to run a server in a doctest...
/// #[cfg(feature = "default")]
/// #[tokio::main]
/// async fn main() {
///     let addr = SocketAddr::from(([127, 0, 0, 1], 8082));
///
///     // build our application with a route
///     let app = Router::new()
///         .route("/api/*fn_name", post(leptos_axum::handle_server_fns));
///
///     // run our app with hyper
///     let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
///     axum::serve(listener, app.into_make_service())
///         .await
///         .unwrap();
/// }
/// # }
/// ```
/// Leptos provides a generic implementation of `handle_server_fns`. If access to more specific parts of the Request is desired,
/// you can specify your own server fn handler based on this one and give it it's own route in the server macro.
///
/// ## Provided Context Types
/// This function always provides context values including the following types:
/// - [`Parts`]
/// - [`ResponseOptions`]
#[tracing::instrument(level = "trace", fields(error), skip_all)]
pub async fn handle_server_fns(req: Request<Body>) -> impl IntoResponse {
    handle_server_fns_inner(|| {}, req).await
}

fn init_executor() {
    #[cfg(feature = "wasm")]
    let _ = leptos::Executor::init_wasm_bindgen();
    #[cfg(all(not(feature = "wasm"), feature = "default"))]
    let _ = leptos::Executor::init_tokio();
    #[cfg(all(not(feature = "wasm"), not(feature = "default")))]
    {
        eprintln!(
            "It appears you have set 'default-features = false' on \
             'leptos_axum', but are not using the 'wasm' feature. Either \
             remove 'default-features = false' or, if you are running in a \
             JS-hosted WASM server environment, add the 'wasm' feature."
        );
    }
}

/// Leptos pool causes wasm to panic and leptos_reactive::spawn::spawn_local causes native
/// to panic so we define a macro to conditionally compile the correct code.
macro_rules! spawn_task {
    ($block:expr) => {
        #[cfg(feature = "wasm")]
        spawn_local($block);
        #[cfg(all(not(feature = "wasm"), feature = "default"))]
        spawn($block);
        #[cfg(all(not(feature = "wasm"), not(feature = "default")))]
        {
            eprintln!(
                "It appears you have set 'default-features = false' on \
                 'leptos_axum', but are not using the 'wasm' feature. Either \
                 remove 'default-features = false' or, if you are running in \
                 a JS-hosted WASM server environment, add the 'wasm' feature."
            );
            spawn_local($block);
        }
    };
}

/// An Axum handlers to listens for a request with Leptos server function arguments in the body,
/// run the server function if found, and return the resulting [`Response`].
///
/// This can then be set up at an appropriate route in your application:
///
/// This version allows you to pass in a closure to capture additional data from the layers above leptos
/// and store it in context. To use it, you'll need to define your own route, and a handler function
/// that takes in the data you'd like. See the [render_app_to_stream_with_context] docs for an example
/// of one that should work much like this one.
///
/// **NOTE**: If your server functions expect a context, make sure to provide it both in
/// [`handle_server_fns_with_context`] **and** in
/// [`leptos_routes_with_context`](LeptosRoutes::leptos_routes_with_context) (or whatever
/// rendering method you are using). During SSR, server functions are called by the rendering
/// method, while subsequent calls from the client are handled by the server function handler.
/// The same context needs to be provided to both handlers.
///
/// ## Provided Context Types
/// This function always provides context values including the following types:
/// - [`Parts`]
/// - [`ResponseOptions`]
#[tracing::instrument(level = "trace", fields(error), skip_all)]
pub async fn handle_server_fns_with_context(
    additional_context: impl Fn() + 'static + Clone + Send,
    req: Request<Body>,
) -> impl IntoResponse {
    handle_server_fns_inner(additional_context, req).await
}

async fn handle_server_fns_inner(
    additional_context: impl Fn() + 'static + Clone + Send,
    req: Request<Body>,
) -> impl IntoResponse {
    use server_fn::middleware::Service;

    let path = req.uri().path().to_string();
    let (req, parts) = generate_request_and_parts(req);

    let res = if let Some(mut service) =
        server_fn::axum::get_server_fn_service(&path)
    {
        let owner = Owner::new();
        owner
            .with(|| {
                ScopedFuture::new(async move {
                    additional_context();
                    provide_context(parts);
                    provide_context(ResponseOptions::default());

                    // store Accepts and Referer in case we need them for redirect (below)
                    let accepts_html = req
                        .headers()
                        .get(ACCEPT)
                        .and_then(|v| v.to_str().ok())
                        .map(|v| v.contains("text/html"))
                        .unwrap_or(false);
                    let referrer = req.headers().get(REFERER).cloned();

                    // actually run the server fn
                    let mut res = service.run(req).await;

                    // update response as needed
                    let res_options = use_context::<ResponseOptions>()
                        .expect("ResponseOptions not found")
                        .0;
                    let res_options_inner = res_options.read();
                    let (status, mut res_headers) = (
                        res_options_inner.status,
                        res_options_inner.headers.clone(),
                    );

                    // it it accepts text/html (i.e., is a plain form post) and doesn't already have a
                    // Location set, then redirect to to Referer
                    if accepts_html {
                        if let Some(referrer) = referrer {
                            let has_location =
                                res.headers().get(LOCATION).is_some();
                            if !has_location {
                                *res.status_mut() = StatusCode::FOUND;
                                res.headers_mut().insert(LOCATION, referrer);
                            }
                        }
                    }

                    // apply status code and headers if used changed them
                    if let Some(status) = status {
                        *res.status_mut() = status;
                    }
                    res.headers_mut().extend(res_headers.drain());
                    Ok(res)
                })
            })
            .await
    } else {
        Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(format!(
                "Could not find a server function at the route {path}. \
                 \n\nIt's likely that either
                         1. The API prefix you specify in the `#[server]` \
                 macro doesn't match the prefix at which your server function \
                 handler is mounted, or \n2. You are on a platform that \
                 doesn't support automatic server function registration and \
                 you need to call ServerFn::register_explicit() on the server \
                 function type, somewhere in your `main` function.",
            )))
    }
    .expect("could not build Response");

    res

    /*rx.await.unwrap_or_else(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            ServerFnErrorSerde::ser(
                &ServerFnError::<NoCustomError>::ServerError(e.to_string()),
            )
            .unwrap_or_default(),
        )
            .into_response()
    })*/
}

pub type PinnedHtmlStream =
    Pin<Box<dyn Stream<Item = io::Result<Bytes>> + Send>>;
type PinnedStream<T> = Pin<Box<dyn Stream<Item = T> + Send>>;

/// Returns an Axum [Handler](axum::handler::Handler) that listens for a `GET` request and tries
/// to route it using [leptos_router], serving an HTML stream of your application.
///
/// The provides a [MetaContext] and a [RouterIntegrationContext] to app’s context before
/// rendering it, and includes any meta tags injected using [leptos_meta].
///
/// The HTML stream is rendered using [render_to_stream](leptos::ssr::render_to_stream), and
/// includes everything described in the documentation for that function.
///
/// This can then be set up at an appropriate route in your application:
/// ```
/// use axum::{handler::Handler, Router};
/// use leptos::*;
/// use leptos_config::get_configuration;
/// use std::{env, net::SocketAddr};
///
/// #[component]
/// fn MyApp() -> impl IntoView {
///     view! { <main>"Hello, world!"</main> }
/// }
///
/// # if false { // don't actually try to run a server in a doctest...
/// #[cfg(feature = "default")]
/// #[tokio::main]
/// async fn main() {
///     let conf = get_configuration(Some("Cargo.toml")).await.unwrap();
///     let leptos_options = conf.leptos_options;
///     let addr = leptos_options.site_addr.clone();
///
///     // build our application with a route
///     let app = Router::new().fallback(leptos_axum::render_app_to_stream(
///         leptos_options,
///         || view! { <MyApp/> },
///     ));
///
///     // run our app with hyper
///     let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
///     axum::serve(listener, app.into_make_service())
///         .await
///         .unwrap();
/// }
/// # }
/// ```
///
/// ## Provided Context Types
/// This function always provides context values including the following types:
/// - [`Parts`]
/// - [`ResponseOptions`]
/// - [`MetaContext`](leptos_meta::MetaContext)
/// - [`RouterIntegrationContext`](leptos_router::RouterIntegrationContext)
#[tracing::instrument(level = "trace", fields(error), skip_all)]
pub fn render_app_to_stream<IV>(
    options: LeptosOptions,
    app_fn: impl Fn() -> IV + Clone + Send + 'static,
) -> impl Fn(
    Request<Body>,
) -> Pin<Box<dyn Future<Output = Response<Body>> + Send + 'static>>
       + Clone
       + Send
       + 'static
where
    IV: IntoView + 'static,
{
    render_app_to_stream_with_context(options, || {}, app_fn)
}

/// Returns an Axum [Handler](axum::handler::Handler) that listens for a `GET` request and tries
/// to route it using [leptos_router], serving an HTML stream of your application.
/// The difference between calling this and `render_app_to_stream_with_context()` is that this
/// one respects the `SsrMode` on each Route and thus requires `Vec<AxumRouteListing>` for route checking.
/// This is useful if you are using `.leptos_routes_with_handler()`
#[tracing::instrument(level = "trace", fields(error), skip_all)]
pub fn render_route<IV>(
    options: LeptosOptions,
    paths: Vec<AxumRouteListing>,
    app_fn: impl Fn() -> IV + Clone + Send + 'static,
) -> impl Fn(
    Request<Body>,
) -> Pin<Box<dyn Future<Output = Response<Body>> + Send + 'static>>
       + Clone
       + Send
       + 'static
where
    IV: IntoView + 'static,
{
    render_route_with_context(options, paths, || {}, app_fn)
}

/// Returns an Axum [Handler](axum::handler::Handler) that listens for a `GET` request and tries
/// to route it using [leptos_router], serving an in-order HTML stream of your application.
/// This stream will pause at each `<Suspense/>` node and wait for it to resolve before
/// sending down its HTML. The app will become interactive once it has fully loaded.
///
/// The provides a [MetaContext] and a [RouterIntegrationContext] to app’s context before
/// rendering it, and includes any meta tags injected using [leptos_meta].
///
/// The HTML stream is rendered using [render_to_stream_in_order], and includes everything described in
/// the documentation for that function.
///
/// This can then be set up at an appropriate route in your application:
/// ```
/// use axum::{handler::Handler, Router};
/// use leptos::*;
/// use leptos_config::get_configuration;
/// use std::{env, net::SocketAddr};
///
/// #[component]
/// fn MyApp() -> impl IntoView {
///     view! { <main>"Hello, world!"</main> }
/// }
///
/// # if false { // don't actually try to run a server in a doctest...
/// #[cfg(feature = "default")]
/// #[tokio::main]
/// async fn main() {
///     let conf = get_configuration(Some("Cargo.toml")).await.unwrap();
///     let leptos_options = conf.leptos_options;
///     let addr = leptos_options.site_addr.clone();
///
///     // build our application with a route
///     let app =
///         Router::new().fallback(leptos_axum::render_app_to_stream_in_order(
///             leptos_options,
///             || view! { <MyApp/> },
///         ));
///
///     // run our app with hyper
///     let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
///     axum::serve(listener, app.into_make_service())
///         .await
///         .unwrap();
/// }
/// # }
/// ```
///
/// ## Provided Context Types
/// This function always provides context values including the following types:
/// - [`Parts`]
/// - [`ResponseOptions`]
/// - [`MetaContext`](leptos_meta::MetaContext)
/// - [`RouterIntegrationContext`](leptos_router::RouterIntegrationContext)
#[tracing::instrument(level = "trace", fields(error), skip_all)]
pub fn render_app_to_stream_in_order<IV>(
    options: LeptosOptions,
    app_fn: impl Fn() -> IV + Clone + Send + 'static,
) -> impl Fn(
    Request<Body>,
) -> Pin<Box<dyn Future<Output = Response<Body>> + Send + 'static>>
       + Clone
       + Send
       + 'static
where
    IV: IntoView + 'static,
{
    render_app_to_stream_in_order_with_context(options, || {}, app_fn)
}

/// Returns an Axum [Handler](axum::handler::Handler) that listens for a `GET` request and tries
/// to route it using [leptos_router], serving an HTML stream of your application.
///
/// This version allows us to pass Axum State/Extension/Extractor or other infro from Axum or network
/// layers above Leptos itself. To use it, you'll need to write your own handler function that provides
/// the data to leptos in a closure. An example is below
/// ```ignore
/// async fn custom_handler(Path(id): Path<String>, Extension(options): Extension<Arc<LeptosOptions>>, req: Request<Body>) -> Response{
///     let handler = leptos_axum::render_app_to_stream_with_context((*options).clone(),
///     || {
///         provide_context(id.clone());
///     },
///     || view! { <TodoApp/> }
/// );
///     handler(req).await.into_response()
/// }
/// ```
/// Otherwise, this function is identical to [render_app_to_stream].
///
/// ## Provided Context Types
/// This function always provides context values including the following types:
/// - [`Parts`]
/// - [`ResponseOptions`]
/// - [`MetaContext`](leptos_meta::MetaContext)
/// - [`RouterIntegrationContext`](leptos_router::RouterIntegrationContext)
#[tracing::instrument(level = "trace", fields(error), skip_all)]
pub fn render_app_to_stream_with_context<IV>(
    options: LeptosOptions,
    additional_context: impl Fn() + 'static + Clone + Send,
    app_fn: impl Fn() -> IV + Clone + Send + 'static,
) -> impl Fn(
    Request<Body>,
) -> Pin<Box<dyn Future<Output = Response<Body>> + Send + 'static>>
       + Clone
       + Send
       + 'static
where
    IV: IntoView + 'static,
{
    render_app_to_stream_with_context_and_replace_blocks(
        options,
        additional_context,
        app_fn,
        false,
    )
}
/// Returns an Axum [Handler](axum::handler::Handler) that listens for a `GET` request and tries
/// to route it using [leptos_router], serving an HTML stream of your application. It allows you
/// to pass in a context function with additional info to be made available to the app
/// The difference between calling this and `render_app_to_stream_with_context()` is that this
/// one respects the `SsrMode` on each Route, and thus requires `Vec<AxumRouteListing>` for route checking.
/// This is useful if you are using `.leptos_routes_with_handler()`.
#[tracing::instrument(level = "trace", fields(error), skip_all)]
pub fn render_route_with_context<IV>(
    options: LeptosOptions,
    paths: Vec<AxumRouteListing>,
    additional_context: impl Fn() + 'static + Clone + Send,
    app_fn: impl Fn() -> IV + Clone + Send + 'static,
) -> impl Fn(
    Request<Body>,
) -> Pin<Box<dyn Future<Output = Response<Body>> + Send + 'static>>
       + Clone
       + Send
       + 'static
where
    IV: IntoView + 'static,
{
    let ooo = render_app_to_stream_with_context(
        LeptosOptions::from_ref(&options),
        additional_context.clone(),
        app_fn.clone(),
    );
    let pb = render_app_to_stream_with_context_and_replace_blocks(
        LeptosOptions::from_ref(&options),
        additional_context.clone(),
        app_fn.clone(),
        true,
    );
    let io = render_app_to_stream_in_order_with_context(
        LeptosOptions::from_ref(&options),
        additional_context.clone(),
        app_fn.clone(),
    );
    let asyn = render_app_async_stream_with_context(
        LeptosOptions::from_ref(&options),
        additional_context.clone(),
        app_fn.clone(),
    );

    move |req| {
        // 1. Process route to match the values in routeListing
        let path = req
            .extensions()
            .get::<MatchedPath>()
            .expect("Failed to get Axum router rule")
            .as_str();
        // 2. Find RouteListing in paths. This should probably be optimized, we probably don't want to
        // search for this every time
        let listing: &AxumRouteListing =
            paths.iter().find(|r| r.path() == path).unwrap_or_else(|| {
                panic!(
                    "Failed to find the route {path} requested by the user. \
                     This suggests that the routing rules in the Router that \
                     call this handler needs to be edited!"
                )
            });
        // 3. Match listing mode against known, and choose function
        match listing.mode() {
            SsrMode::OutOfOrder => ooo(req),
            SsrMode::PartiallyBlocked => pb(req),
            SsrMode::InOrder => io(req),
            SsrMode::Async => asyn(req),
        }
    }
}

/// Returns an Axum [Handler](axum::handler::Handler) that listens for a `GET` request and tries
/// to route it using [leptos_router], serving an HTML stream of your application.
///
/// This version allows us to pass Axum State/Extension/Extractor or other info from Axum or network
/// layers above Leptos itself. To use it, you'll need to write your own handler function that provides
/// the data to leptos in a closure.
///
/// `replace_blocks` additionally lets you specify whether `<Suspense/>` fragments that read
/// from blocking resources should be retrojected into the HTML that's initially served, rather
/// than dynamically inserting them with JavaScript on the client. This means you will have
/// better support if JavaScript is not enabled, in exchange for a marginally slower response time.
///
/// Otherwise, this function is identical to [render_app_to_stream_with_context].
///
/// ## Provided Context Types
/// This function always provides context values including the following types:
/// - [`Parts`]
/// - [`ResponseOptions`]
/// - [`MetaContext`](leptos_meta::MetaContext)
/// - [`RouterIntegrationContext`](leptos_router::RouterIntegrationContext)
#[tracing::instrument(level = "trace", fields(error), skip_all)]
pub fn render_app_to_stream_with_context_and_replace_blocks<IV>(
    options: LeptosOptions,
    additional_context: impl Fn() + 'static + Clone + Send,
    app_fn: impl Fn() -> IV + Clone + Send + 'static,
    replace_blocks: bool,
) -> impl Fn(
    Request<Body>,
) -> Pin<Box<dyn Future<Output = Response<Body>> + Send + 'static>>
       + Clone
       + Send
       + 'static
where
    IV: IntoView + 'static,
{
    handle_response(options, additional_context, app_fn, |app, chunks| {
        Box::pin(async move {
            Box::pin(app.to_html_stream_out_of_order().chain(chunks()))
                as PinnedStream<String>
        })
    })
}

/// Returns an Axum [Handler](axum::handler::Handler) that listens for a `GET` request and tries
/// to route it using [leptos_router], serving an in-order HTML stream of your application.
/// This stream will pause at each `<Suspense/>` node and wait for it to resolve before
/// sending down its HTML. The app will become interactive once it has fully loaded.
///
/// This version allows us to pass Axum State/Extension/Extractor or other infro from Axum or network
/// layers above Leptos itself. To use it, you'll need to write your own handler function that provides
/// the data to leptos in a closure. An example is below
/// ```ignore
/// async fn custom_handler(Path(id): Path<String>, Extension(options): Extension<Arc<LeptosOptions>>, req: Request<Body>) -> Response{
///     let handler = leptos_axum::render_app_to_stream_in_order_with_context((*options).clone(),
///     move || {
///         provide_context(id.clone());
///     },
///     || view! { <TodoApp/> }
/// );
///     handler(req).await.into_response()
/// }
/// ```
/// Otherwise, this function is identical to [render_app_to_stream].
///
/// ## Provided Context Types
/// This function always provides context values including the following types:
/// - [`Parts`]
/// - [`ResponseOptions`]
/// - [`MetaContext`](leptos_meta::MetaContext)
/// - [`RouterIntegrationContext`](leptos_router::RouterIntegrationContext)
#[tracing::instrument(level = "trace", fields(error), skip_all)]
pub fn render_app_to_stream_in_order_with_context<IV>(
    options: LeptosOptions,
    additional_context: impl Fn() + 'static + Clone + Send,
    app_fn: impl Fn() -> IV + Clone + Send + 'static,
) -> impl Fn(
    Request<Body>,
) -> Pin<Box<dyn Future<Output = Response<Body>> + Send + 'static>>
       + Clone
       + Send
       + 'static
where
    IV: IntoView + 'static,
{
    handle_response(options, additional_context, app_fn, |app, chunks| {
        Box::pin(async move {
            Box::pin(app.to_html_stream_in_order().chain(chunks()))
                as PinnedStream<String>
        })
    })
}

fn handle_response<IV>(
    options: LeptosOptions,
    additional_context: impl Fn() + 'static + Clone + Send,
    app_fn: impl Fn() -> IV + Clone + Send + 'static,
    stream_builder: fn(
        IV,
        Box<dyn FnOnce() -> Pin<Box<dyn Stream<Item = String> + Send>> + Send>,
    ) -> Pin<
        Box<
            dyn Future<Output = Pin<Box<dyn Stream<Item = String> + Send>>>
                + Send,
        >,
    >,
) -> impl Fn(
    Request<Body>,
) -> Pin<Box<dyn Future<Output = Response<Body>> + Send + 'static>>
       + Clone
       + Send
       + 'static
where
    IV: IntoView + 'static,
{
    move |req: Request<Body>| {
        let options = options.clone();
        let app_fn = app_fn.clone();
        let add_context = additional_context.clone();
        let res_options = ResponseOptions::default();

        let owner = Owner::new_root(Some(Arc::new(SsrSharedContext::new())));
        Box::pin(Sandboxed::new(async move {
            let meta_context = ServerMetaContext::new();
            let stream = ScopedFuture::new(owner.with(|| {
                // Need to get the path and query string of the Request
                // For reasons that escape me, if the incoming URI protocol is https, it provides the absolute URI
                let path = req.uri().path_and_query().unwrap().as_str();

                let full_path = format!("http://leptos.dev{path}");
                let (_, req_parts) = generate_request_and_parts(req);
                provide_contexts(
                    &full_path,
                    &meta_context,
                    req_parts,
                    res_options.clone(),
                );
                add_context();

                // run app
                let app = app_fn();

                // TODO nonce

                let shared_context = Owner::current_shared_context().unwrap();
                let chunks = Box::new(move || {
                    Box::pin(
                        shared_context
                            .pending_data()
                            .unwrap()
                            .map(|chunk| format!("<script>{chunk}</script>")),
                    )
                        as Pin<Box<dyn Stream<Item = String> + Send>>
                });

                // convert app to appropriate response type
                // and chain the app stream, followed by chunks
                // in theory, we could select here, and intersperse them
                // the problem is that during the DOM walk, that would be mean random <script> tags
                // interspersed where we expect other children
                //
                // we also don't actually start hydrating until after the whole stream is complete,
                // so it's not useful to send those scripts down earlier.
                stream_builder(app, chunks)
            }));
            let stream = stream.await;
            let stream = meta_context.inject_meta_context(stream).await;

            // TODO test this
            /*if let Some(status) = res_options.status {
                *res.status_mut() = status
            }

            let headers = res.headers_mut();

            let mut res_headers = res_options.headers.clone();
            headers.extend(res_headers.drain());

            if !headers.contains_key(header::CONTENT_TYPE) {
                // Set the Content Type headers on all responses. This makes Firefox show the page source
                // without complaining
                headers.insert(
                    header::CONTENT_TYPE,
                    HeaderValue::from_str("text/html; charset=utf-8").unwrap(),
                );
            }*/

            Html(Body::from_stream(Sandboxed::new(
                stream
                    .map(|chunk| Ok(chunk) as Result<String, std::io::Error>)
                    // drop the owner, cleaning up the reactive runtime,
                    // once the stream is over
                    .chain(once(async move {
                        drop(owner);
                        Ok(Default::default())
                    })),
            )))
            .into_response()
        }))
    }
}

#[tracing::instrument(level = "trace", fields(error), skip_all)]
fn provide_contexts(
    path: &str,
    meta_context: &ServerMetaContext,
    parts: Parts,
    default_res_options: ResponseOptions,
) {
    provide_context(RequestUrl::new(path));
    provide_context(meta_context.clone());
    provide_context(parts);
    provide_context(default_res_options);
    // TODO server redirect
    // provide_server_redirect(redirect);
    #[cfg(feature = "nonce")]
    leptos::nonce::provide_nonce();
}

/// Returns an Axum [Handler](axum::handler::Handler) that listens for a `GET` request and tries
/// to route it using [leptos_router], asynchronously rendering an HTML page after all
/// `async` [Resource](leptos::Resource)s have loaded.
///
/// The provides a [MetaContext] and a [RouterIntegrationContext] to app’s context before
/// rendering it, and includes any meta tags injected using [leptos_meta].
///
/// The HTML stream is rendered using [render_to_string_async], and includes everything described in
/// the documentation for that function.
///
/// This can then be set up at an appropriate route in your application:
/// ```
/// use axum::{handler::Handler, Router};
/// use leptos::*;
/// use leptos_config::get_configuration;
/// use std::{env, net::SocketAddr};
///
/// #[component]
/// fn MyApp() -> impl IntoView {
///     view! { <main>"Hello, world!"</main> }
/// }
///
/// # if false { // don't actually try to run a server in a doctest...
/// #[cfg(feature = "default")]
/// #[tokio::main]
/// async fn main() {
///     let conf = get_configuration(Some("Cargo.toml")).await.unwrap();
///     let leptos_options = conf.leptos_options;
///     let addr = leptos_options.site_addr.clone();
///
///     // build our application with a route
///     let app = Router::new().fallback(leptos_axum::render_app_async(
///         leptos_options,
///         || view! { <MyApp/> },
///     ));
///
///     // run our app with hyper
///     // `axum::Server` is a re-export of `hyper::Server`
///     let listener =
///         tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
///     axum::serve(listener, app.into_make_service())
///         .await
///         .unwrap();
/// }
/// # }
/// ```
///
/// ## Provided Context Types
/// This function always provides context values including the following types:
/// - [`Parts`]
/// - [`ResponseOptions`]
/// - [`MetaContext`](leptos_meta::MetaContext)
/// - [`RouterIntegrationContext`](leptos_router::RouterIntegrationContext)
#[tracing::instrument(level = "trace", fields(error), skip_all)]
pub fn render_app_async<IV>(
    options: LeptosOptions,
    app_fn: impl Fn() -> IV + Clone + Send + 'static,
) -> impl Fn(
    Request<Body>,
) -> Pin<Box<dyn Future<Output = Response<Body>> + Send + 'static>>
       + Clone
       + Send
       + 'static
where
    IV: IntoView + 'static,
{
    render_app_async_with_context(options, || {}, app_fn)
}

/// Returns an Axum [Handler](axum::handler::Handler) that listens for a `GET` request and tries
/// to route it using [leptos_router], asynchronously rendering an HTML page after all
/// `async` [Resource](leptos::Resource)s have loaded.
///
/// This version allows us to pass Axum State/Extension/Extractor or other infro from Axum or network
/// layers above Leptos itself. To use it, you'll need to write your own handler function that provides
/// the data to leptos in a closure. An example is below
/// ```ignore
/// async fn custom_handler(Path(id): Path<String>, Extension(options): Extension<Arc<LeptosOptions>>, req: Request<Body>) -> Response{
///     let handler = leptos_axum::render_app_async_with_context((*options).clone(),
///     move || {
///         provide_context(id.clone());
///     },
///     || view! { <TodoApp/> }
/// );
///     handler(req).await.into_response()
/// }
/// ```
/// Otherwise, this function is identical to [render_app_to_stream].
///
/// ## Provided Context Types
/// This function always provides context values including the following types:
/// - [`Parts`]
/// - [`ResponseOptions`]
/// - [`MetaContext`](leptos_meta::MetaContext)
/// - [`RouterIntegrationContext`](leptos_router::RouterIntegrationContext)
#[tracing::instrument(level = "trace", fields(error), skip_all)]
pub fn render_app_async_stream_with_context<IV>(
    options: LeptosOptions,
    additional_context: impl Fn() + 'static + Clone + Send,
    app_fn: impl Fn() -> IV + Clone + Send + 'static,
) -> impl Fn(
    Request<Body>,
) -> Pin<Box<dyn Future<Output = Response<Body>> + Send + 'static>>
       + Clone
       + Send
       + 'static
where
    IV: IntoView + 'static,
{
    handle_response(options, additional_context, app_fn, |app, chunks| {
        Box::pin(async move {
            let app = app.to_html_stream_in_order().collect::<String>().await;
            let chunks = chunks();
            Box::pin(once(async move { app }).chain(chunks))
                as PinnedStream<String>
        })
    })
}

/// Returns an Axum [Handler](axum::handler::Handler) that listens for a `GET` request and tries
/// to route it using [leptos_router], asynchronously rendering an HTML page after all
/// `async` [Resource](leptos::Resource)s have loaded.
///
/// This version allows us to pass Axum State/Extension/Extractor or other infro from Axum or network
/// layers above Leptos itself. To use it, you'll need to write your own handler function that provides
/// the data to leptos in a closure. An example is below
/// ```ignore
/// async fn custom_handler(Path(id): Path<String>, Extension(options): Extension<Arc<LeptosOptions>>, req: Request<Body>) -> Response{
///     let handler = leptos_axum::render_app_async_with_context((*options).clone(),
///     move || {
///         provide_context(id.clone());
///     },
///     || view! { <TodoApp/> }
/// );
///     handler(req).await.into_response()
/// }
/// ```
/// Otherwise, this function is identical to [render_app_to_stream].
///
/// ## Provided Context Types
/// This function always provides context values including the following types:
/// - [`Parts`]
/// - [`ResponseOptions`]
/// - [`MetaContext`](leptos_meta::MetaContext)
/// - [`RouterIntegrationContext`](leptos_router::RouterIntegrationContext)
#[tracing::instrument(level = "trace", fields(error), skip_all)]
pub fn render_app_async_with_context<IV>(
    options: LeptosOptions,
    additional_context: impl Fn() + 'static + Clone + Send,
    app_fn: impl Fn() -> IV + Clone + Send + 'static,
) -> impl Fn(
    Request<Body>,
) -> Pin<Box<dyn Future<Output = Response<Body>> + Send + 'static>>
       + Clone
       + Send
       + 'static
where
    IV: IntoView + 'static,
{
    handle_response(options, additional_context, app_fn, |app, chunks| {
        Box::pin(async move {
            let app = app.to_html_stream_in_order().collect::<String>().await;
            let chunks = chunks();
            Box::pin(once(async move { app }).chain(chunks))
                as PinnedStream<String>
        })
    })
}

/// Generates a list of all routes defined in Leptos's Router in your app. We can then use this to automatically
/// create routes in Axum's Router without having to use wildcard matching or fallbacks. Takes in your root app Element
/// as an argument so it can walk you app tree. This version is tailored to generate Axum compatible paths.
#[tracing::instrument(level = "trace", fields(error), skip_all)]
pub fn generate_route_list<IV>(
    app_fn: impl Fn() -> IV + 'static + Clone,
) -> Vec<AxumRouteListing>
where
    IV: IntoView + 'static,
{
    generate_route_list_with_exclusions_and_ssg(app_fn, None).0
}

/// Generates a list of all routes defined in Leptos's Router in your app. We can then use this to automatically
/// create routes in Axum's Router without having to use wildcard matching or fallbacks. Takes in your root app Element
/// as an argument so it can walk you app tree. This version is tailored to generate Axum compatible paths.
#[tracing::instrument(level = "trace", fields(error), skip_all)]
pub fn generate_route_list_with_ssg<IV>(
    app_fn: impl Fn() -> IV + 'static + Clone,
) -> (Vec<AxumRouteListing>, StaticDataMap)
where
    IV: IntoView + 'static,
{
    generate_route_list_with_exclusions_and_ssg(app_fn, None)
}

/// Generates a list of all routes defined in Leptos's Router in your app. We can then use this to automatically
/// create routes in Axum's Router without having to use wildcard matching or fallbacks. Takes in your root app Element
/// as an argument so it can walk you app tree. This version is tailored to generate Axum compatible paths. Adding excluded_routes
/// to this function will stop `.leptos_routes()` from generating a route for it, allowing a custom handler. These need to be in Axum path format
#[tracing::instrument(level = "trace", fields(error), skip_all)]
pub fn generate_route_list_with_exclusions<IV>(
    app_fn: impl Fn() -> IV + 'static + Clone,
    excluded_routes: Option<Vec<String>>,
) -> Vec<AxumRouteListing>
where
    IV: IntoView + 'static,
{
    generate_route_list_with_exclusions_and_ssg(app_fn, excluded_routes).0
}

/// TODO docs
pub async fn build_static_routes<IV>(
    options: &LeptosOptions,
    app_fn: impl Fn() -> IV + 'static + Send + Clone,
    routes: &[RouteListing],
    static_data_map: StaticDataMap,
) where
    IV: IntoView + 'static,
{
    todo!()
    /*
    let options = options.clone();
    let routes = routes.to_owned();
    spawn_task!(async move {
        leptos_router::build_static_routes(
            &options,
            app_fn,
            &routes,
            &static_data_map,
        )
        .await
        .expect("could not build static routes")
    });*/
}

/// Generates a list of all routes defined in Leptos's Router in your app. We can then use this to automatically
/// create routes in Axum's Router without having to use wildcard matching or fallbacks. Takes in your root app Element
/// as an argument so it can walk you app tree. This version is tailored to generate Axum compatible paths. Adding excluded_routes
/// to this function will stop `.leptos_routes()` from generating a route for it, allowing a custom handler. These need to be in Axum path format
#[tracing::instrument(level = "trace", fields(error), skip_all)]
pub fn generate_route_list_with_exclusions_and_ssg<IV>(
    app_fn: impl Fn() -> IV + 'static + Clone,
    excluded_routes: Option<Vec<String>>,
) -> (Vec<AxumRouteListing>, StaticDataMap)
where
    IV: IntoView + 'static,
{
    generate_route_list_with_exclusions_and_ssg_and_context(
        app_fn,
        excluded_routes,
        || {},
    )
}

#[derive(Clone, Debug, Default)]
/// A route that this application can serve.
pub struct AxumRouteListing {
    path: String,
    mode: SsrMode,
    methods: Vec<leptos_router::Method>,
    static_mode: Option<(StaticMode, StaticDataMap)>,
}

impl From<RouteListing> for AxumRouteListing {
    fn from(value: RouteListing) -> Self {
        let path = value.path().to_axum_path();
        let path = if path.is_empty() {
            "/".to_string()
        } else {
            path
        };
        let mode = value.mode();
        let methods = value.methods().collect();
        let static_mode = value.into_static_parts();
        Self {
            path,
            mode,
            methods,
            static_mode,
        }
    }
}

impl AxumRouteListing {
    /// Create a route listing from its parts.
    pub fn new(
        path: String,
        mode: SsrMode,
        methods: impl IntoIterator<Item = leptos_router::Method>,
        static_mode: Option<(StaticMode, StaticDataMap)>,
    ) -> Self {
        Self {
            path,
            mode,
            methods: methods.into_iter().collect(),
            static_mode,
        }
    }

    /// The path this route handles.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// The rendering mode for this path.
    pub fn mode(&self) -> SsrMode {
        self.mode
    }

    /// The HTTP request methods this path can handle.
    pub fn methods(&self) -> impl Iterator<Item = leptos_router::Method> + '_ {
        self.methods.iter().copied()
    }

    /// Whether this route is statically rendered.
    #[inline(always)]
    pub fn static_mode(&self) -> Option<StaticMode> {
        self.static_mode.as_ref().map(|n| n.0)
    }
}

/// Generates a list of all routes defined in Leptos's Router in your app. We can then use this to automatically
/// create routes in Axum's Router without having to use wildcard matching or fallbacks. Takes in your root app Element
/// as an argument so it can walk you app tree. This version is tailored to generate Axum compatible paths. Adding excluded_routes
/// to this function will stop `.leptos_routes()` from generating a route for it, allowing a custom handler. These need to be in Axum path format
/// Additional context will be provided to the app Element.
#[tracing::instrument(level = "trace", fields(error), skip_all)]
pub fn generate_route_list_with_exclusions_and_ssg_and_context<IV>(
    app_fn: impl Fn() -> IV + 'static + Clone,
    excluded_routes: Option<Vec<String>>,
    additional_context: impl Fn() + 'static + Clone,
) -> (Vec<AxumRouteListing>, StaticDataMap)
where
    IV: IntoView + 'static,
{
    init_executor();

    let owner = Owner::new_root(None);
    let routes = owner
        .with(|| {
            // stub out a path for now
            provide_context(RequestUrl::new(""));
            additional_context();
            RouteList::generate(&app_fn)
        })
        .unwrap_or_default();

    // Axum's Router defines Root routes as "/" not ""
    let mut routes = routes
        .into_inner()
        .into_iter()
        .map(AxumRouteListing::from)
        .collect::<Vec<_>>();

    (
        if routes.is_empty() {
            vec![AxumRouteListing::new(
                "/".to_string(),
                Default::default(),
                [leptos_router::Method::Get],
                None,
            )]
        } else {
            // Routes to exclude from auto generation
            if let Some(excluded_routes) = excluded_routes {
                routes
                    .retain(|p| !excluded_routes.iter().any(|e| e == p.path()))
            }
            routes
        },
        StaticDataMap::new(), // TODO
                              //static_data_map,
    )
}

/// This trait allows one to pass a list of routes and a render function to Axum's router, letting us avoid
/// having to use wildcards or manually define all routes in multiple places.
pub trait LeptosRoutes<S>
where
    S: Clone + Send + Sync + 'static,
{
    fn leptos_routes<IV>(
        self,
        options: &S,
        paths: Vec<AxumRouteListing>,
        app_fn: impl Fn() -> IV + Clone + Send + 'static,
    ) -> Self
    where
        IV: IntoView + 'static;

    fn leptos_routes_with_context<IV>(
        self,
        options: &S,
        paths: Vec<AxumRouteListing>,
        additional_context: impl Fn() + 'static + Clone + Send,
        app_fn: impl Fn() -> IV + Clone + Send + 'static,
    ) -> Self
    where
        IV: IntoView + 'static;

    fn leptos_routes_with_handler<H, T>(
        self,
        paths: Vec<AxumRouteListing>,
        handler: H,
    ) -> Self
    where
        H: axum::handler::Handler<T, S>,
        T: 'static;
}
/*
#[cfg(feature = "default")]
fn handle_static_response<IV>(
    path: String,
    options: LeptosOptions,
    app_fn: impl Fn() -> IV + Clone + Send + 'static,
    additional_context: impl Fn() + Clone + Send + 'static,
    res: StaticResponse,
) -> Pin<Box<dyn Future<Output = Response<String>> + 'static>>
where
    IV: IntoView + 'static,
{
    Box::pin(async move {
        match res {
            StaticResponse::ReturnResponse {
                body,
                status,
                content_type,
            } => {
                let mut res = Response::new(body);
                if let Some(v) = content_type {
                    res.headers_mut().insert(
                        HeaderName::from_static("content-type"),
                        HeaderValue::from_static(v),
                    );
                }
                *res.status_mut() = match status {
                    StaticStatusCode::Ok => StatusCode::OK,
                    StaticStatusCode::NotFound => StatusCode::NOT_FOUND,
                    StaticStatusCode::InternalServerError => {
                        StatusCode::INTERNAL_SERVER_ERROR
                    }
                };
                res
            }
            StaticResponse::RenderDynamic => {
                let res = render_dynamic(
                    &path,
                    &options,
                    app_fn.clone(),
                    additional_context.clone(),
                )
                .await;
                handle_static_response(
                    path,
                    options,
                    app_fn,
                    additional_context,
                    res,
                )
                .await
            }
            StaticResponse::RenderNotFound => {
                let res = not_found_page(
                    tokio::fs::read_to_string(not_found_path(&options)).await,
                );
                handle_static_response(
                    path,
                    options,
                    app_fn,
                    additional_context,
                    res,
                )
                .await
            }
            StaticResponse::WriteFile { body, path } => {
                if let Some(path) = path.parent() {
                    if let Err(e) = std::fs::create_dir_all(path) {
                        tracing::error!(
                            "encountered error {} writing directories {}",
                            e,
                            path.display()
                        );
                    }
                }
                if let Err(e) = std::fs::write(&path, &body) {
                    tracing::error!(
                        "encountered error {} writing file {}",
                        e,
                        path.display()
                    );
                }
                handle_static_response(
                    path.to_str().unwrap().to_string(),
                    options,
                    app_fn,
                    additional_context,
                    StaticResponse::ReturnResponse {
                        body,
                        status: StaticStatusCode::Ok,
                        content_type: Some("text/html"),
                    },
                )
                .await
            }
        }
    })
}*/

#[cfg(feature = "default")]
fn static_route<IV, S>(
    router: axum::Router<S>,
    path: &str,
    options: LeptosOptions,
    app_fn: impl Fn() -> IV + Clone + Send + 'static,
    additional_context: impl Fn() + Clone + Send + 'static,
    method: leptos_router::Method,
    mode: StaticMode,
) -> axum::Router<S>
where
    IV: IntoView + 'static,
    S: Clone + Send + Sync + 'static,
{
    todo!()
    /*match mode {
        StaticMode::Incremental => {
            let handler = move |req: Request<Body>| {
                Box::pin({
                    let path = req.uri().path().to_string();
                    let options = options.clone();
                    let app_fn = app_fn.clone();
                    let additional_context = additional_context.clone();

                    async move {
                        let (tx, rx) = futures::channel::oneshot::channel();
                        spawn_task!(async move {
                            let res = incremental_static_route(
                                tokio::fs::read_to_string(static_file_path(
                                    &options, &path,
                                ))
                                .await,
                            );
                            let res = handle_static_response(
                                path.clone(),
                                options,
                                app_fn,
                                additional_context,
                                res,
                            )
                            .await;

                            let _ = tx.send(res);
                        });
                        rx.await.expect("to complete HTML rendering")
                    }
                })
            };
            router.route(
                path,
                match method {
                    leptos_router::Method::Get => get(handler),
                    leptos_router::Method::Post => post(handler),
                    leptos_router::Method::Put => put(handler),
                    leptos_router::Method::Delete => delete(handler),
                    leptos_router::Method::Patch => patch(handler),
                },
            )
        }
        StaticMode::Upfront => {
            let handler = move |req: Request<Body>| {
                Box::pin({
                    let path = req.uri().path().to_string();
                    let options = options.clone();
                    let app_fn = app_fn.clone();
                    let additional_context = additional_context.clone();

                    async move {
                        let (tx, rx) = futures::channel::oneshot::channel();
                        spawn_task!(async move {
                            let res = upfront_static_route(
                                tokio::fs::read_to_string(static_file_path(
                                    &options, &path,
                                ))
                                .await,
                            );
                            let res = handle_static_response(
                                path.clone(),
                                options,
                                app_fn,
                                additional_context,
                                res,
                            )
                            .await;

                            let _ = tx.send(res);
                        });
                        rx.await.expect("to complete HTML rendering")
                    }
                })
            };
            router.route(
                path,
                match method {
                    leptos_router::Method::Get => get(handler),
                    leptos_router::Method::Post => post(handler),
                    leptos_router::Method::Put => put(handler),
                    leptos_router::Method::Delete => delete(handler),
                    leptos_router::Method::Patch => patch(handler),
                },
            )
        }
    }*/
}

trait AxumPath {
    fn to_axum_path(&self) -> String;
}

impl AxumPath for &[PathSegment] {
    fn to_axum_path(&self) -> String {
        let mut path = String::new();
        for segment in self.iter() {
            if !segment.as_raw_str().starts_with('/') {
                path.push('/');
            }
            match segment {
                PathSegment::Static(s) => path.push_str(s),
                PathSegment::Param(s) => {
                    path.push(':');
                    path.push_str(s);
                }
                PathSegment::Splat(s) => {
                    path.push('*');
                    path.push_str(s);
                }
                PathSegment::Unit => {}
            }
        }
        path
    }
}

/// The default implementation of `LeptosRoutes` which takes in a list of paths, and dispatches GET requests
/// to those paths to Leptos's renderer.
impl<S> LeptosRoutes<S> for axum::Router<S>
where
    LeptosOptions: FromRef<S>,
    S: Clone + Send + Sync + 'static,
{
    #[tracing::instrument(level = "trace", fields(error), skip_all)]
    fn leptos_routes<IV>(
        self,
        options: &S,
        paths: Vec<AxumRouteListing>,
        app_fn: impl Fn() -> IV + Clone + Send + 'static,
    ) -> Self
    where
        IV: IntoView + 'static,
    {
        self.leptos_routes_with_context(options, paths, || {}, app_fn)
    }

    #[tracing::instrument(level = "trace", fields(error), skip_all)]
    fn leptos_routes_with_context<IV>(
        self,
        options: &S,
        paths: Vec<AxumRouteListing>,
        additional_context: impl Fn() + 'static + Clone + Send,
        app_fn: impl Fn() -> IV + Clone + Send + 'static,
    ) -> Self
    where
        IV: IntoView + 'static,
    {
        init_executor();

        // S represents the router's finished state allowing us to provide
        // it to the user's server functions.
        let state = options.clone();
        let cx_with_state = move || {
            provide_context::<S>(state.clone());
            additional_context();
        };

        let mut router = self;

        // register server functions
        println!(
            "server fn paths are {:?}",
            server_fn::axum::server_fn_paths().collect::<Vec<_>>()
        );
        for (path, method) in server_fn::axum::server_fn_paths() {
            println!("registering {path}");
            let cx_with_state = cx_with_state.clone();
            let handler = move |req: Request<Body>| async move {
                handle_server_fns_with_context(cx_with_state, req).await
            };
            router = router.route(
                path,
                match method {
                    Method::GET => get(handler),
                    Method::POST => post(handler),
                    Method::PUT => put(handler),
                    Method::DELETE => delete(handler),
                    Method::PATCH => patch(handler),
                    _ => {
                        panic!(
                            "Unsupported server function HTTP method: \
                             {method:?}"
                        );
                    }
                },
            );
        }

        // register router paths
        for listing in paths.iter() {
            let path = listing.path();

            for method in listing.methods() {
                let cx_with_state = cx_with_state.clone();
                let cx_with_state_and_method = move || {
                    provide_context(method);
                    cx_with_state();
                };
                router = if let Some(static_mode) = listing.static_mode() {
                    #[cfg(feature = "default")]
                    {
                        static_route(
                            router,
                            &path,
                            LeptosOptions::from_ref(options),
                            app_fn.clone(),
                            cx_with_state_and_method.clone(),
                            method,
                            static_mode,
                        )
                    }
                    #[cfg(not(feature = "default"))]
                    {
                        _ = static_mode;
                        panic!(
                            "Static site generation is not currently \
                             supported on WASM32 server targets."
                        )
                    }
                } else {
                    router.route(
                    &path,
                    match listing.mode() {
                        SsrMode::OutOfOrder => {
                            let s = render_app_to_stream_with_context(
                                LeptosOptions::from_ref(options),
                                cx_with_state_and_method.clone(),
                                app_fn.clone(),
                            );
                            match method {
                                leptos_router::Method::Get => get(s),
                                leptos_router::Method::Post => post(s),
                                leptos_router::Method::Put => put(s),
                                leptos_router::Method::Delete => delete(s),
                                leptos_router::Method::Patch => patch(s),
                            }
                        }
                        SsrMode::PartiallyBlocked => {
                            let s = render_app_to_stream_with_context_and_replace_blocks(
                                LeptosOptions::from_ref(options),
                                cx_with_state_and_method.clone(),
                                app_fn.clone(),
                                true
                            );
                            match method {
                                leptos_router::Method::Get => get(s),
                                leptos_router::Method::Post => post(s),
                                leptos_router::Method::Put => put(s),
                                leptos_router::Method::Delete => delete(s),
                                leptos_router::Method::Patch => patch(s),
                            }
                        }
                        SsrMode::InOrder => {
                            let s = render_app_to_stream_in_order_with_context(
                                LeptosOptions::from_ref(options),
                                cx_with_state_and_method.clone(),
                                app_fn.clone(),
                            );
                            match method {
                                leptos_router::Method::Get => get(s),
                                leptos_router::Method::Post => post(s),
                                leptos_router::Method::Put => put(s),
                                leptos_router::Method::Delete => delete(s),
                                leptos_router::Method::Patch => patch(s),
                            }
                        }
                        SsrMode::Async => {
                            let s = render_app_async_with_context(
                                LeptosOptions::from_ref(options),
                                cx_with_state_and_method.clone(),
                                app_fn.clone(),
                            );
                            match method {
                                leptos_router::Method::Get => get(s),
                                leptos_router::Method::Post => post(s),
                                leptos_router::Method::Put => put(s),
                                leptos_router::Method::Delete => delete(s),
                                leptos_router::Method::Patch => patch(s),
                            }
                        }
                    },
                )
                };
            }
        }

        router
    }

    #[tracing::instrument(level = "trace", fields(error), skip_all)]
    fn leptos_routes_with_handler<H, T>(
        self,
        paths: Vec<AxumRouteListing>,
        handler: H,
    ) -> Self
    where
        H: axum::handler::Handler<T, S>,
        T: 'static,
    {
        let mut router = self;
        for listing in paths.iter() {
            for method in listing.methods() {
                router = router.route(
                    listing.path(),
                    match method {
                        leptos_router::Method::Get => get(handler.clone()),
                        leptos_router::Method::Post => post(handler.clone()),
                        leptos_router::Method::Put => put(handler.clone()),
                        leptos_router::Method::Delete => {
                            delete(handler.clone())
                        }
                        leptos_router::Method::Patch => patch(handler.clone()),
                    },
                );
            }
        }
        router
    }
}

/// A helper to make it easier to use Axum extractors in server functions.
///
/// It is generic over some type `T` that implements [`FromRequestParts`] and can
/// therefore be used in an extractor. The compiler can often infer this type.
///
/// Any error that occurs during extraction is converted to a [`ServerFnError`].
///
/// ```rust,ignore
/// // MyQuery is some type that implements `Deserialize + Serialize`
/// #[server]
/// pub async fn query_extract() -> Result<MyQuery, ServerFnError> {
///     use axum::{extract::Query, http::Method};
///     use leptos_axum::*;
///     let Query(query) = extract().await?;
///
///     Ok(query)
/// }
/// ```
pub async fn extract<T>() -> Result<T, ServerFnError>
where
    T: Sized + FromRequestParts<()>,
    T::Rejection: Debug,
{
    extract_with_state::<T, ()>(&()).await
}

/// A helper to make it easier to use Axum extractors in server functions. This
/// function is compatible with extractors that require access to `State`.
///
/// It is generic over some type `T` that implements [`FromRequestParts`] and can
/// therefore be used in an extractor. The compiler can often infer this type.
///
/// Any error that occurs during extraction is converted to a [`ServerFnError`].
///
/// ```rust,ignore
/// // MyQuery is some type that implements `Deserialize + Serialize`
/// #[server]
/// pub async fn query_extract() -> Result<MyQuery, ServerFnError> {
///     use axum::{extract::Query, http::Method};
///     use leptos_axum::*;
///     let Query(query) = extract().await?;
///
///     Ok(query)
/// }
/// ```
pub async fn extract_with_state<T, S>(state: &S) -> Result<T, ServerFnError>
where
    T: Sized + FromRequestParts<S>,
    T::Rejection: Debug,
{
    let mut parts = use_context::<Parts>().ok_or_else(|| {
        ServerFnError::new(
            "should have had Parts provided by the leptos_axum integration",
        )
    })?;
    T::from_request_parts(&mut parts, state)
        .await
        .map_err(|e| ServerFnError::ServerError(format!("{e:?}")))
}
