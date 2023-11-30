#![forbid(unsafe_code)]
//! Provides functions to easily integrate Leptos with Axum.
//!
//! ## JS Fetch Integration
//! The `leptos_axum` integration supports running in JavaScript-hosted WebAssembly
//! runtimes, e.g., running inside Deno, Cloudflare Workers, or other JS environments.
//! To run in this environment, you need to disable the default feature set and enable
//! the `wasm` feature on `leptos_axum` in your `Cargo.toml`.
//! ```toml
//! leptos_axum = { version = "0.5.4", default-features = false, features = ["wasm"] }
//! ```
//!
//! ## Features
//! - `default`: supports running in a typical native Tokio/Axum environment
//! - `wasm`: with `default-features = false`, supports running in a JS Fetch-based
//!   environment
//! - `nonce`: activates Leptos features that automatically provide a CSP [`Nonce`](leptos::nonce::Nonce) via context
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
    extract::{FromRef, FromRequestParts, MatchedPath, Path, RawQuery},
    http::{
        header::{self, HeaderName, HeaderValue},
        method::Method,
        request::Parts,
        uri::Uri,
        version::Version,
        HeaderMap, Request, Response, StatusCode,
    },
    response::IntoResponse,
    routing::{delete, get, patch, post, put},
};
use futures::{
    channel::mpsc::{Receiver, Sender},
    Future, SinkExt, Stream, StreamExt,
};
use http_body_util::BodyExt;
use leptos::{
    leptos_server::{server_fn_by_path, Payload},
    server_fn::Encoding,
    ssr::*,
    *,
};
use leptos_integration_utils::{build_async_response, html_parts_separated};
use leptos_meta::{generate_head_metadata_separated, MetaContext};
use leptos_router::*;
use once_cell::sync::OnceCell;
use parking_lot::RwLock;
use std::{fmt::Debug, io, pin::Pin, sync::Arc, thread::available_parallelism};
use tokio_util::task::LocalPoolHandle;
use tracing::Instrument;
/// A struct to hold the parts of the incoming Request. Since `http::Request` isn't cloneable, we're forced
/// to construct this for Leptos to use in Axum
#[derive(Debug, Clone)]
pub struct RequestParts {
    pub version: Version,
    pub method: Method,
    pub uri: Uri,
    pub headers: HeaderMap<HeaderValue>,
    pub body: Bytes,
}

/// Convert http::Parts to RequestParts(and vice versa). Body and Extensions will
/// be lost in the conversion
impl From<Parts> for RequestParts {
    fn from(parts: Parts) -> Self {
        Self {
            version: parts.version,
            method: parts.method,
            uri: parts.uri,
            headers: parts.headers,
            body: Bytes::default(),
        }
    }
}

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
    if let Some(response_options) = use_context::<ResponseOptions>() {
        response_options.set_status(StatusCode::FOUND);
        response_options.insert_header(
            header::LOCATION,
            header::HeaderValue::from_str(path)
                .expect("Failed to create HeaderValue"),
        );
    }
}

/// Decomposes an HTTP request into its parts, allowing you to read its headers
/// and other data without consuming the body. Creates a new Request from the
/// original parts for further processing
pub async fn generate_request_and_parts(
    req: Request<Body>,
) -> (Request<Body>, RequestParts) {
    // provide request headers as context in server scope
    let (parts, body) = req.into_parts();
    let body = body.collect().await.unwrap_or_default().to_bytes();
    let request_parts = RequestParts {
        method: parts.method.clone(),
        uri: parts.uri.clone(),
        headers: parts.headers.clone(),
        version: parts.version,
        body: body.clone(),
    };
    let request = Request::from_parts(parts, body.into());

    (request, request_parts)
}

/// An Axum handlers to listens for a request with Leptos server function arguments in the body,
/// run the server function if found, and return the resulting [Response].
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
/// - [RequestParts]
/// - [ResponseOptions]
#[tracing::instrument(level = "trace", fields(error), skip_all)]
pub async fn handle_server_fns(
    Path(fn_name): Path<String>,
    headers: HeaderMap,
    RawQuery(query): RawQuery,
    req: Request<Body>,
) -> impl IntoResponse {
    handle_server_fns_inner(fn_name, headers, query, || {}, req).await
}

/// Leptos pool causes wasm to panic and leptos_reactive::spawn::spawn_local causes native
/// to panic so we define a macro to conditionally compile the correct code.
macro_rules! spawn_task {
    ($block:expr) => {
        cfg_if::cfg_if! {
            if #[cfg(feature = "wasm")] {
                spawn_local($block);
            } else if #[cfg(feature = "default")] {
                let pool_handle = get_leptos_pool();
                pool_handle.spawn_pinned(move || { $block });
            } else {
                eprintln!("It appears you have set 'default-features = false' on 'leptos_axum', \
                but are not using the 'wasm' feature. Either remove 'default-features = false' or, \
                if you are running in a JS-hosted WASM server environment, add the 'wasm' feature.");
                spawn_local($block);
            }
        }
    };
}

/// An Axum handlers to listens for a request with Leptos server function arguments in the body,
/// run the server function if found, and return the resulting [Response].
///
/// This can then be set up at an appropriate route in your application:
///
/// This version allows you to pass in a closure to capture additional data from the layers above leptos
/// and store it in context. To use it, you'll need to define your own route, and a handler function
/// that takes in the data you'd like. See the [render_app_to_stream_with_context] docs for an example
/// of one that should work much like this one.
///
/// **NOTE**: If your server functions expect a context, make sure to provide it both in
/// [`handle_server_fns_with_context`] **and** in [`leptos_routes_with_context`] (or whatever
/// rendering method you are using). During SSR, server functions are called by the rendering
/// method, while subsequent calls from the client are handled by the server function handler.
/// The same context needs to be provided to both handlers.
///
/// ## Provided Context Types
/// This function always provides context values including the following types:
/// - [RequestParts]
/// - [ResponseOptions]
#[tracing::instrument(level = "trace", fields(error), skip_all)]
pub async fn handle_server_fns_with_context(
    Path(fn_name): Path<String>,
    headers: HeaderMap,
    RawQuery(query): RawQuery,
    additional_context: impl Fn() + 'static + Clone + Send,
    req: Request<Body>,
) -> impl IntoResponse {
    handle_server_fns_inner(fn_name, headers, query, additional_context, req)
        .await
}
#[tracing::instrument(level = "trace", fields(error), skip_all)]
async fn handle_server_fns_inner(
    fn_name: String,
    headers: HeaderMap,
    query: Option<String>,
    additional_context: impl Fn() + 'static + Clone + Send,
    req: Request<Body>,
) -> impl IntoResponse {
    // Axum Path extractor doesn't remove the first slash from the path, while Actix does
    let fn_name = fn_name
        .strip_prefix('/')
        .map(|fn_name| fn_name.to_string())
        .unwrap_or(fn_name);

    let (tx, rx) = futures::channel::oneshot::channel();

    spawn_task!(async move {
        let res =
            if let Some(server_fn) = server_fn_by_path(fn_name.as_str()) {
                let runtime = create_runtime();

                additional_context();

                let (req, req_parts) = generate_request_and_parts(req).await;

                provide_context(req_parts.clone());
                provide_context(ExtractorHelper::from(req));
                // Add this so that we can set headers and status of the response
                provide_context(ResponseOptions::default());

                let query: &Bytes = &query.unwrap_or("".to_string()).into();
                let data = match &server_fn.encoding() {
                    Encoding::Url | Encoding::Cbor => &req_parts.body,
                    Encoding::GetJSON | Encoding::GetCBOR => query,
                };
                let res = match server_fn.call((), data).await {
                    Ok(serialized) => {
                        // If ResponseOptions are set, add the headers and status to the request
                        let res_options = use_context::<ResponseOptions>();

                        // if this is Accept: application/json then send a serialized JSON response
                        let accept_header = headers
                            .get("Accept")
                            .and_then(|value| value.to_str().ok());
                        let mut res = Response::builder();

                        // Add headers from ResponseParts if they exist. These should be added as long
                        // as the server function returns an OK response
                        let res_options_outer = res_options.unwrap().0;
                        let res_options_inner = res_options_outer.read();
                        let (status, mut res_headers) = (
                            res_options_inner.status,
                            res_options_inner.headers.clone(),
                        );

                        if accept_header == Some("application/json")
                            || accept_header
                                == Some("application/x-www-form-urlencoded")
                            || accept_header == Some("application/cbor")
                        {
                            res = res.status(StatusCode::OK);
                        }
                        // otherwise, it's probably a <form> submit or something: redirect back to the referrer
                        else {
                            let referer = headers
                                .get("Referer")
                                .and_then(|value| value.to_str().ok())
                                .unwrap_or("/");

                            res = res
                                .status(StatusCode::SEE_OTHER)
                                .header("Location", referer);
                        }
                        // Override StatusCode if it was set in a Resource or Element
                        res = match status {
                            Some(status) => res.status(status),
                            None => res,
                        };
                        // This must be after the default referrer
                        // redirect so that it overwrites the one above
                        if let Some(header_ref) = res.headers_mut() {
                            header_ref.extend(res_headers.drain());
                        };
                        match serialized {
                            Payload::Binary(data) => res
                                .header("Content-Type", "application/cbor")
                                .body(Body::from(data)),
                            Payload::Url(data) => res
                                .header(
                                    "Content-Type",
                                    "application/x-www-form-urlencoded",
                                )
                                .body(Body::from(data)),
                            Payload::Json(data) => res
                                .header("Content-Type", "application/json")
                                .body(Body::from(data)),
                        }
                    }
                    Err(e) => Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .body(Body::from(
                            serde_json::to_string(&e)
                                .unwrap_or_else(|_| e.to_string()),
                        )),
                };
                // clean up the scope
                runtime.dispose();
                res
            } else {
                Response::builder().status(StatusCode::BAD_REQUEST).body(
                    Body::from(format!(
                        "Could not find a server function at the route \
                         {fn_name}. \n\nIt's likely that either
                         1. The API prefix you specify in the `#[server]` \
                         macro doesn't match the prefix at which your server \
                         function handler is mounted, or \n2. You are on a \
                         platform that doesn't support automatic server \
                         function registration and you need to call \
                         ServerFn::register_explicit() on the server function \
                         type, somewhere in your `main` function.",
                    )),
                )
            }
            .expect("could not build Response");

        _ = tx.send(res);
    });

    rx.await.unwrap()
}

pub type PinnedHtmlStream =
    Pin<Box<dyn Stream<Item = io::Result<Bytes>> + Send>>;

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
/// - [RequestParts]
/// - [ResponseOptions]
/// - [MetaContext](leptos_meta::MetaContext)
/// - [RouterIntegrationContext](leptos_router::RouterIntegrationContext)
#[tracing::instrument(level = "info", fields(error), skip_all)]
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
    IV: IntoView,
{
    render_app_to_stream_with_context(options, || {}, app_fn)
}

/// Returns an Axum [Handler](axum::handler::Handler) that listens for a `GET` request and tries
/// to route it using [leptos_router], serving an HTML stream of your application.
/// The difference between calling this and `render_app_to_stream_with_context()` is that this
/// one respects the `SsrMode` on each Route and thus requires `Vec<RouteListing>` for route checking.
/// This is useful if you are using `.leptos_routes_with_handler()`
#[tracing::instrument(level = "info", fields(error), skip_all)]
pub fn render_route<IV>(
    options: LeptosOptions,
    paths: Vec<RouteListing>,
    app_fn: impl Fn() -> IV + Clone + Send + 'static,
) -> impl Fn(
    Request<Body>,
) -> Pin<Box<dyn Future<Output = Response<Body>> + Send + 'static>>
       + Clone
       + Send
       + 'static
where
    IV: IntoView,
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
/// - [RequestParts]
/// - [ResponseOptions]
/// - [MetaContext](leptos_meta::MetaContext)
/// - [RouterIntegrationContext](leptos_router::RouterIntegrationContext)
#[tracing::instrument(level = "info", fields(error), skip_all)]
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
    IV: IntoView,
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
/// - [RequestParts]
/// - [ResponseOptions]
/// - [MetaContext](leptos_meta::MetaContext)
/// - [RouterIntegrationContext](leptos_router::RouterIntegrationContext)
#[tracing::instrument(level = "info", fields(error), skip_all)]
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
    IV: IntoView,
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
/// one respects the `SsrMode` on each Route, and thus requires `Vec<RouteListing>` for route checking.
/// This is useful if you are using `.leptos_routes_with_handler()`.
#[tracing::instrument(level = "info", fields(error), skip_all)]
pub fn render_route_with_context<IV>(
    options: LeptosOptions,
    paths: Vec<RouteListing>,
    additional_context: impl Fn() + 'static + Clone + Send,
    app_fn: impl Fn() -> IV + Clone + Send + 'static,
) -> impl Fn(
    Request<Body>,
) -> Pin<Box<dyn Future<Output = Response<Body>> + Send + 'static>>
       + Clone
       + Send
       + 'static
where
    IV: IntoView,
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
        let listing: &RouteListing =
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
/// - [RequestParts]
/// - [ResponseOptions]
/// - [MetaContext](leptos_meta::MetaContext)
/// - [RouterIntegrationContext](leptos_router::RouterIntegrationContext)
#[tracing::instrument(level = "info", fields(error), skip_all)]
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
    IV: IntoView,
{
    move |req: Request<Body>| {
        Box::pin({
            let options = options.clone();
            let app_fn = app_fn.clone();
            let add_context = additional_context.clone();
            let default_res_options = ResponseOptions::default();
            let res_options2 = default_res_options.clone();
            let res_options3 = default_res_options.clone();
            let (tx, rx) = futures::channel::mpsc::channel(8);

            let current_span = tracing::Span::current();
            spawn_task!(async move {
                let app = {
                    // Need to get the path and query string of the Request
                    // For reasons that escape me, if the incoming URI protocol is https, it provides the absolute URI
                    // if http, it returns a relative path. Adding .path() seems to make it explicitly return the relative uri
                    let path = req.uri().path_and_query().unwrap().as_str();

                    let full_path = format!("http://leptos.dev{path}");
                    let (req, req_parts) = generate_request_and_parts(req).await;
                    move || {
                        provide_contexts(full_path, req_parts, req.into(), default_res_options);
                        app_fn().into_view()
                    }
                };
                let (bundle, runtime) =
                    leptos::leptos_dom::ssr::render_to_stream_with_prefix_undisposed_with_context_and_block_replacement(
                        app,
                        || generate_head_metadata_separated().1.into(),
                        add_context,
                        replace_blocks
                    );

                    forward_stream(&options, res_options2, bundle, tx).await;

                    runtime.dispose();
            }.instrument(current_span));

            generate_response(res_options3, rx)
        })
    }
}

#[tracing::instrument(level = "info", fields(error), skip_all)]
async fn generate_response(
    res_options: ResponseOptions,
    rx: Receiver<String>,
) -> Response<Body> {
    let mut stream = Box::pin(rx.map(|html| Ok(Bytes::from(html))));

    // Get the first and second chunks in the stream, which renders the app shell, and thus allows Resources to run
    let first_chunk = stream.next().await;

    let second_chunk = stream.next().await;

    // Extract the resources now that they've been rendered
    let res_options = res_options.0.read();

    let complete_stream =
        futures::stream::iter([first_chunk.unwrap(), second_chunk.unwrap()])
            .chain(stream);

    let mut res =
        Body::from_stream(Box::pin(complete_stream) as PinnedHtmlStream)
            .into_response();

    if let Some(status) = res_options.status {
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
    }
    res
}
#[tracing::instrument(level = "info", fields(error), skip_all)]
async fn forward_stream(
    options: &LeptosOptions,
    res_options2: ResponseOptions,
    bundle: impl Stream<Item = String> + 'static,
    mut tx: Sender<String>,
) {
    let mut shell = Box::pin(bundle);
    let first_app_chunk = shell.next().await.unwrap_or_default();

    let (head, tail) =
        html_parts_separated(options, use_context::<MetaContext>().as_ref());

    _ = tx.send(head).await;

    _ = tx.send(first_app_chunk).await;

    while let Some(fragment) = shell.next().await {
        _ = tx.send(fragment).await;
    }

    _ = tx.send(tail.to_string()).await;

    // Extract the value of ResponseOptions from here
    let res_options = use_context::<ResponseOptions>().unwrap();

    let new_res_parts = res_options.0.read().clone();

    let mut writable = res_options2.0.write();
    *writable = new_res_parts;

    tx.close_channel();
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
/// - [RequestParts]
/// - [ResponseOptions]
/// - [MetaContext](leptos_meta::MetaContext)
/// - [RouterIntegrationContext](leptos_router::RouterIntegrationContext)
#[tracing::instrument(level = "info", fields(error), skip_all)]
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
    IV: IntoView,
{
    move |req: Request<Body>| {
        Box::pin({
            let options = options.clone();
            let app_fn = app_fn.clone();
            let add_context = additional_context.clone();
            let default_res_options = ResponseOptions::default();
            let res_options2 = default_res_options.clone();
            let res_options3 = default_res_options.clone();

            async move {
                // Need to get the path and query string of the Request
                // For reasons that escape me, if the incoming URI protocol is https, it provides the absolute URI
                // if http, it returns a relative path. Adding .path() seems to make it explicitly return the relative uri
                let path = req.uri().path_and_query().unwrap().as_str();

                let full_path = format!("http://leptos.dev{path}");

                let (tx, rx) = futures::channel::mpsc::channel(8);
                let current_span = tracing::Span::current();
                spawn_task!(async move {
                    let app = {
                        let full_path = full_path.clone();
                        let (req, req_parts) = generate_request_and_parts(req).await;
                        move || {
                            provide_contexts(full_path, req_parts, req.into(), default_res_options);
                            app_fn().into_view()
                        }
                    };

                    let (bundle, runtime) =
                        leptos::ssr::render_to_stream_in_order_with_prefix_undisposed_with_context(
                            app,
                            || generate_head_metadata_separated().1.into(),
                            add_context,
                        );

                    forward_stream(&options, res_options2, bundle, tx).await;

                    runtime.dispose();
                }.instrument(current_span));

                generate_response(res_options3, rx).await
            }
        })
    }
}

#[tracing::instrument(level = "trace", fields(error), skip_all)]
fn provide_contexts(
    path: String,
    req_parts: RequestParts,
    extractor: ExtractorHelper,
    default_res_options: ResponseOptions,
) {
    let integration = ServerIntegration { path };
    provide_context(RouterIntegrationContext::new(integration));
    provide_context(MetaContext::new());
    provide_context(req_parts);
    provide_context(extractor);
    provide_context(default_res_options);
    provide_server_redirect(redirect);
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
/// - [RequestParts]
/// - [ResponseOptions]
/// - [MetaContext](leptos_meta::MetaContext)
/// - [RouterIntegrationContext](leptos_router::RouterIntegrationContext)
#[tracing::instrument(level = "info", fields(error), skip_all)]
pub fn render_app_async<IV>(
    options: LeptosOptions,
    app_fn: impl Fn() -> IV + Clone + Send + 'static,
) -> impl Fn(
    Request<Body>,
) -> Pin<Box<dyn Future<Output = Response<String>> + Send + 'static>>
       + Clone
       + Send
       + 'static
where
    IV: IntoView,
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
/// - [RequestParts]
/// - [ResponseOptions]
/// - [MetaContext](leptos_meta::MetaContext)
/// - [RouterIntegrationContext](leptos_router::RouterIntegrationContext)
#[tracing::instrument(level = "info", fields(error), skip_all)]
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
    IV: IntoView,
{
    move |req: Request<Body>| {
        Box::pin({
            let options = options.clone();
            let app_fn = app_fn.clone();
            let add_context = additional_context.clone();
            let default_res_options = ResponseOptions::default();
            let res_options2 = default_res_options.clone();
            let res_options3 = default_res_options.clone();

            async move {
                // Need to get the path and query string of the Request
                // For reasons that escape me, if the incoming URI protocol is https, it provides the absolute URI
                // if http, it returns a relative path. Adding .path() seems to make it explicitly return the relative uri
                let path = req.uri().path_and_query().unwrap().as_str();

                let full_path = format!("http://leptos.dev{path}");

                let (tx, rx) = futures::channel::oneshot::channel();
                spawn_task!(async move {
                    let app = {
                        let full_path = full_path.clone();
                        let (req, req_parts) =
                            generate_request_and_parts(req).await;
                        move || {
                            provide_contexts(
                                full_path,
                                req_parts,
                                req.into(),
                                default_res_options,
                            );
                            app_fn().into_view()
                        }
                    };

                    let (stream, runtime) =
                        render_to_stream_in_order_with_prefix_undisposed_with_context(
                            app,
                            || "".into(),
                            add_context,
                        );

                    // Extract the value of ResponseOptions from here
                    let res_options = use_context::<ResponseOptions>().unwrap();

                    let html =
                        build_async_response(stream, &options, runtime).await;

                    let new_res_parts = res_options.0.read().clone();

                    let mut writable = res_options2.0.write();
                    *writable = new_res_parts;

                    _ = tx.send(html);
                });

                let html = rx.await.expect("to complete HTML rendering");

                let res_options = res_options3.0.read();

                let complete_stream =
                    futures::stream::iter([Ok(Bytes::from(html))]);

                let mut res = Body::from_stream(
                    Box::pin(complete_stream) as PinnedHtmlStream
                )
                .into_response();
                if let Some(status) = res_options.status {
                    *res.status_mut() = status
                }
                let headers = res.headers_mut();
                let mut res_headers = res_options.headers.clone();

                headers.extend(res_headers.drain());

                // This one doesn't use generate_response(), so we need to do this seperately
                if !headers.contains_key(header::CONTENT_TYPE) {
                    // Set the Content Type headers on all responses. This makes Firefox show the page source
                    // without complaining
                    headers.insert(
                        header::CONTENT_TYPE,
                        HeaderValue::from_str("text/html; charset=utf-8")
                            .unwrap(),
                    );
                }

                res
            }
        })
    }
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
/// - [RequestParts]
/// - [ResponseOptions]
/// - [MetaContext](leptos_meta::MetaContext)
/// - [RouterIntegrationContext](leptos_router::RouterIntegrationContext)
#[tracing::instrument(level = "info", fields(error), skip_all)]
pub fn render_app_async_with_context<IV>(
    options: LeptosOptions,
    additional_context: impl Fn() + 'static + Clone + Send,
    app_fn: impl Fn() -> IV + Clone + Send + 'static,
) -> impl Fn(
    Request<Body>,
) -> Pin<Box<dyn Future<Output = Response<String>> + Send + 'static>>
       + Clone
       + Send
       + 'static
where
    IV: IntoView,
{
    move |req: Request<Body>| {
        Box::pin({
            let options = options.clone();
            let app_fn = app_fn.clone();
            let add_context = additional_context.clone();
            let default_res_options = ResponseOptions::default();
            let res_options2 = default_res_options.clone();
            let res_options3 = default_res_options.clone();

            async move {
                // Need to get the path and query string of the Request
                // For reasons that escape me, if the incoming URI protocol is https, it provides the absolute URI
                // if http, it returns a relative path. Adding .path() seems to make it explicitly return the relative uri
                let path = req.uri().path_and_query().unwrap().as_str();

                let full_path = format!("http://leptos.dev{path}");

                let (tx, rx) = futures::channel::oneshot::channel();

                spawn_task!(async move {
                    let app = {
                        let full_path = full_path.clone();
                        let (req, req_parts) =
                            generate_request_and_parts(req).await;
                        move || {
                            provide_contexts(
                                full_path,
                                req_parts,
                                req.into(),
                                default_res_options,
                            );
                            app_fn().into_view()
                        }
                    };

                    let (stream, runtime) =
                            render_to_stream_in_order_with_prefix_undisposed_with_context(
                                app,
                                || "".into(),
                                add_context,
                            );

                    // Extract the value of ResponseOptions from here
                    let res_options = use_context::<ResponseOptions>().unwrap();

                    let html =
                        build_async_response(stream, &options, runtime).await;

                    let new_res_parts = res_options.0.read().clone();

                    let mut writable = res_options2.0.write();
                    *writable = new_res_parts;

                    _ = tx.send(html);
                });

                let html = rx.await.expect("to complete HTML rendering");

                let mut res = Response::new(html);

                let res_options = res_options3.0.read();

                if let Some(status) = res_options.status {
                    *res.status_mut() = status
                }
                let mut res_headers = res_options.headers.clone();
                res.headers_mut().extend(res_headers.drain());

                res
            }
        })
    }
}

/// Generates a list of all routes defined in Leptos's Router in your app. We can then use this to automatically
/// create routes in Axum's Router without having to use wildcard matching or fallbacks. Takes in your root app Element
/// as an argument so it can walk you app tree. This version is tailored to generate Axum compatible paths.
#[tracing::instrument(level = "trace", fields(error), skip_all)]
pub fn generate_route_list<IV>(
    app_fn: impl Fn() -> IV + 'static + Clone,
) -> Vec<RouteListing>
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
) -> (Vec<RouteListing>, StaticDataMap)
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
) -> Vec<RouteListing>
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
    });
}

/// Generates a list of all routes defined in Leptos's Router in your app. We can then use this to automatically
/// create routes in Axum's Router without having to use wildcard matching or fallbacks. Takes in your root app Element
/// as an argument so it can walk you app tree. This version is tailored to generate Axum compatible paths. Adding excluded_routes
/// to this function will stop `.leptos_routes()` from generating a route for it, allowing a custom handler. These need to be in Axum path format
#[tracing::instrument(level = "trace", fields(error), skip_all)]
pub fn generate_route_list_with_exclusions_and_ssg<IV>(
    app_fn: impl Fn() -> IV + 'static + Clone,
    excluded_routes: Option<Vec<String>>,
) -> (Vec<RouteListing>, StaticDataMap)
where
    IV: IntoView + 'static,
{
    let (routes, static_data_map) =
        leptos_router::generate_route_list_inner(app_fn);
    // Axum's Router defines Root routes as "/" not ""
    let mut routes = routes
        .into_iter()
        .map(|listing| {
            let path = listing.path();
            if path.is_empty() {
                RouteListing::new(
                    "/".to_string(),
                    listing.path(),
                    listing.mode(),
                    listing.methods(),
                    listing.static_mode(),
                )
            } else {
                listing
            }
        })
        .collect::<Vec<_>>();

    (
        if routes.is_empty() {
            vec![RouteListing::new(
                "/",
                "",
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
        static_data_map,
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
        paths: Vec<RouteListing>,
        app_fn: impl Fn() -> IV + Clone + Send + 'static,
    ) -> Self
    where
        IV: IntoView + 'static;

    fn leptos_routes_with_context<IV>(
        self,
        options: &S,
        paths: Vec<RouteListing>,
        additional_context: impl Fn() + 'static + Clone + Send,
        app_fn: impl Fn() -> IV + Clone + Send + 'static,
    ) -> Self
    where
        IV: IntoView + 'static;

    fn leptos_routes_with_handler<H, T>(
        self,
        paths: Vec<RouteListing>,
        handler: H,
    ) -> Self
    where
        H: axum::handler::Handler<T, S>,
        T: 'static;
}

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
}

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
    match mode {
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
    }
}

/// The default implementation of `LeptosRoutes` which takes in a list of paths, and dispatches GET requests
/// to those paths to Leptos's renderer.
impl<S> LeptosRoutes<S> for axum::Router<S>
where
    LeptosOptions: FromRef<S>,
    S: Clone + Send + Sync + 'static,
{
    #[tracing::instrument(level = "info", fields(error), skip_all)]
    fn leptos_routes<IV>(
        self,
        options: &S,
        paths: Vec<RouteListing>,
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
        paths: Vec<RouteListing>,
        additional_context: impl Fn() + 'static + Clone + Send,
        app_fn: impl Fn() -> IV + Clone + Send + 'static,
    ) -> Self
    where
        IV: IntoView + 'static,
    {
        let mut router = self;
        for listing in paths.iter() {
            let path = listing.path();

            for method in listing.methods() {
                router = if let Some(static_mode) = listing.static_mode() {
                    #[cfg(feature = "default")]
                    {
                        static_route(
                            router,
                            path,
                            LeptosOptions::from_ref(options),
                            app_fn.clone(),
                            additional_context.clone(),
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
                    path,
                    match listing.mode() {
                        SsrMode::OutOfOrder => {
                            let s = render_app_to_stream_with_context(
                                LeptosOptions::from_ref(options),
                                additional_context.clone(),
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
                                additional_context.clone(),
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
                                additional_context.clone(),
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
                                additional_context.clone(),
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
        paths: Vec<RouteListing>,
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

#[tracing::instrument(level = "trace", fields(error), skip_all)]
fn get_leptos_pool() -> LocalPoolHandle {
    static LOCAL_POOL: OnceCell<LocalPoolHandle> = OnceCell::new();
    LOCAL_POOL
        .get_or_init(|| {
            tokio_util::task::LocalPoolHandle::new(
                available_parallelism().map(Into::into).unwrap_or(1),
            )
        })
        .clone()
}

#[derive(Clone, Debug)]
struct ExtractorHelper {
    parts: Arc<tokio::sync::Mutex<Parts>>,
}

impl ExtractorHelper {
    pub fn new(parts: Parts) -> Self {
        Self {
            parts: Arc::new(tokio::sync::Mutex::new(parts)),
        }
    }

    pub async fn extract<F, T, U, S>(
        &self,
        f: F,
        s: S,
    ) -> Result<U, T::Rejection>
    where
        S: Sized,
        F: Extractor<T, U, S>,
        T: core::fmt::Debug + Send + FromRequestParts<S> + 'static,
        T::Rejection: core::fmt::Debug + Send + 'static,
    {
        let mut parts = self.parts.lock().await;
        let data = T::from_request_parts(&mut parts, &s).await?;
        Ok(f.call(data).await)
    }
}

impl<B> From<Request<B>> for ExtractorHelper {
    fn from(req: Request<B>) -> Self {
        // TODO provide body for extractors there, too?
        let (parts, _) = req.into_parts();
        ExtractorHelper::new(parts)
    }
}

/// A helper to make it easier to use Axum extractors in server functions. This takes
/// a handler function as its argument. The handler rules similar to Axum
/// [handlers](https://docs.rs/axum/latest/axum/extract/index.html#intro): it is an async function
/// whose arguments are “extractors.”
///
/// ```rust,ignore
/// #[server(QueryExtract, "/api")]
/// pub async fn query_extract() -> Result<String, ServerFnError> {
///     use axum::{extract::Query, http::Method};
///     use leptos_axum::extract;
///
///     extract(|method: Method, res: Query<MyQuery>| async move {
///             format!("{method:?} and {}", res.q)
///         },
///     )
///     .await
///     .map_err(|e| ServerFnError::ServerError("Could not extract method and query...".to_string()))
/// }
/// ```
///
/// > This function only supports extractors for
/// which the state is `()`. If the state is not `()`, use [`extract_with_state`].
#[tracing::instrument(level = "trace", fields(error), skip_all)]
pub async fn extract<T, U>(
    f: impl Extractor<T, U, ()>,
) -> Result<U, T::Rejection>
where
    T: core::fmt::Debug + Send + FromRequestParts<()> + 'static,
    T::Rejection: core::fmt::Debug + Send + 'static,
{
    extract_with_state((), f).await
}

/// A helper to make it easier to use Axum extractors in server functions, with a
/// simpler API than [`extract`].
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
///     let Query(query) = extractor().await?;
///
///     Ok(query)
/// }
/// ```
pub async fn extractor<T>() -> Result<T, ServerFnError>
where
    T: Sized + FromRequestParts<()>,
    T::Rejection: Debug,
{
    let ctx = use_context::<ExtractorHelper>().expect(
        "should have had ExtractorHelper provided by the leptos_axum \
         integration",
    );
    let mut parts = ctx.parts.lock().await;
    T::from_request_parts(&mut parts, &())
        .await
        .map_err(|e| ServerFnError::ServerError(format!("{e:?}")))
}

/// A helper to make it easier to use Axum extractors in server functions. This takes
/// a handler function and state as its arguments. The handler rules similar to Axum
/// [handlers](https://docs.rs/axum/latest/axum/extract/index.html#intro): it is an async function
/// whose arguments are “extractors.”
///
/// ```rust,ignore
/// #[server(QueryExtract, "/api")]
/// pub async fn query_extract() -> Result<String, ServerFnError> {
///     use axum::{extract::Query, http::Method};
///     use leptos_axum::extract;
///     let state: ServerState = use_context::<crate::ServerState>()
///          .ok_or(ServerFnError::ServerError("No server state".to_string()))?;
///
///     extract_with_state(&state, |method: Method, res: Query<MyQuery>| async move {
///             format!("{method:?} and {}", res.q)
///         },
///     )
///     .await
///     .map_err(|e| ServerFnError::ServerError("Could not extract method and query...".to_string()))
/// }
/// ```
#[tracing::instrument(level = "trace", fields(error), skip_all)]
pub async fn extract_with_state<T, U, S>(
    state: S,
    f: impl Extractor<T, U, S>,
) -> Result<U, T::Rejection>
where
    S: Sized,
    T: core::fmt::Debug + Send + FromRequestParts<S> + 'static,
    T::Rejection: core::fmt::Debug + Send + 'static,
{
    use_context::<ExtractorHelper>()
        .expect(
            "should have had ExtractorHelper provided by the leptos_axum \
             integration",
        )
        .extract(f, state)
        .await
}

pub trait Extractor<T, U, S>
where
    S: Sized,
    T: FromRequestParts<S>,
{
    fn call(self, args: T) -> Pin<Box<dyn Future<Output = U>>>;
}

macro_rules! factory_tuple ({ $($param:ident)* } => {
    impl<Func, Fut, U, S, $($param,)*> Extractor<($($param,)*), U, S> for Func
    where
        $($param: FromRequestParts<S> + Send,)*
        Func: FnOnce($($param),*) -> Fut + 'static,
        Fut: Future<Output = U> + 'static,
        S: Sized + Send + Sync
    {
        #[inline]
        #[allow(non_snake_case)]
        fn call(self, ($($param,)*): ($($param,)*)) -> Pin<Box<dyn Future<Output = U>>> {
            Box::pin((self)($($param,)*))
        }
    }
});

factory_tuple! { A }
factory_tuple! { A B }
factory_tuple! { A B C }
factory_tuple! { A B C D }
factory_tuple! { A B C D E }
factory_tuple! { A B C D E F }
factory_tuple! { A B C D E F G }
factory_tuple! { A B C D E F G H }
factory_tuple! { A B C D E F G H I }
factory_tuple! { A B C D E F G H I J }
factory_tuple! { A B C D E F G H I J K }
factory_tuple! { A B C D E F G H I J K L }
factory_tuple! { A B C D E F G H I J K L M }
factory_tuple! { A B C D E F G H I J K L M N }
factory_tuple! { A B C D E F G H I J K L M N O }
factory_tuple! { A B C D E F G H I J K L M N O P }
