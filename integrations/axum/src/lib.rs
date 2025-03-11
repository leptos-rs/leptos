#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![allow(clippy::type_complexity)]

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

#[cfg(feature = "default")]
use axum::http::Uri;
use axum::{
    body::{Body, Bytes},
    extract::{FromRef, FromRequestParts, MatchedPath, State},
    http::{
        header::{self, HeaderName, HeaderValue, ACCEPT, LOCATION, REFERER},
        request::Parts,
        HeaderMap, Method, Request, Response, StatusCode,
    },
    response::IntoResponse,
    routing::{delete, get, patch, post, put},
};
#[cfg(feature = "default")]
use dashmap::DashMap;
use futures::{stream::once, Future, Stream, StreamExt};
use hydration_context::SsrSharedContext;
use leptos::{
    config::LeptosOptions,
    context::{provide_context, use_context},
    prelude::*,
    reactive::{computed::ScopedFuture, owner::Owner},
    IntoView,
};
use leptos_integration_utils::{
    BoxedFnOnce, ExtendResponse, PinnedFuture, PinnedStream,
};
use leptos_meta::ServerMetaContext;
#[cfg(feature = "default")]
use leptos_router::static_routes::ResolvedStaticPath;
use leptos_router::{
    components::provide_server_redirect, location::RequestUrl,
    static_routes::RegenerationFn, ExpandOptionals, PathSegment, RouteList,
    RouteListing, SsrMode,
};
#[cfg(feature = "default")]
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use server_fn::{redirect::REDIRECT_HEADER, ServerFnError};
#[cfg(feature = "default")]
use std::path::Path;
use std::{collections::HashSet, fmt::Debug, io, pin::Pin, sync::Arc};
#[cfg(feature = "default")]
use tower::util::ServiceExt;
#[cfg(feature = "default")]
use tower_http::services::ServeDir;
// use tracing::Instrument; // TODO check tracing span -- was this used in 0.6 for a missing link?

/// This struct lets you define headers and override the status of the Response from an Element or a Server Function
/// Typically contained inside of a ResponseOptions. Setting this is useful for cookies and custom responses.
#[derive(Debug, Clone, Default)]
pub struct ResponseParts {
    /// If provided, this will overwrite any other status code for this response.
    pub status: Option<StatusCode>,
    /// The map of headers that should be added to the response.
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
/// ```
/// use leptos::prelude::*;
///
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

struct AxumResponse(Response<Body>);

impl ExtendResponse for AxumResponse {
    type ResponseOptions = ResponseOptions;

    fn from_stream(
        stream: impl Stream<Item = String> + Send + 'static,
    ) -> Self {
        AxumResponse(
            Body::from_stream(
                stream.map(|chunk| Ok(chunk) as Result<String, std::io::Error>),
            )
            .into_response(),
        )
    }

    fn extend_response(&mut self, res_options: &Self::ResponseOptions) {
        let mut res_options = res_options.0.write();
        if let Some(status) = res_options.status {
            *self.0.status_mut() = status;
        }
        self.0
            .headers_mut()
            .extend(std::mem::take(&mut res_options.headers));
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
///    to using [`ActionForm`] without JS/WASM present.)
///
/// Using it with a non-blocking [`Resource`] will not work if you are using streaming rendering,
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
        #[cfg(feature = "tracing")]
        {
            tracing::warn!(
                "Couldn't retrieve either Parts or ResponseOptions while \
                 trying to redirect()."
            );
        }
        #[cfg(not(feature = "tracing"))]
        {
            eprintln!(
                "Couldn't retrieve either Parts or ResponseOptions while \
                 trying to redirect()."
            );
        }
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
/// ```no_run
/// use axum::{handler::Handler, routing::post, Router};
/// use leptos::prelude::*;
/// use std::net::SocketAddr;
///
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
///
/// # #[cfg(not(feature = "default"))]
/// # fn main() { }
/// ```
/// Leptos provides a generic implementation of `handle_server_fns`. If access to more specific parts of the Request is desired,
/// you can specify your own server fn handler based on this one and give it it's own route in the server macro.
///
/// ## Provided Context Types
/// This function always provides context values including the following types:
/// - [`Parts`]
/// - [`ResponseOptions`]
#[cfg_attr(
    feature = "tracing",
    tracing::instrument(level = "trace", fields(error), skip_all)
)]
pub async fn handle_server_fns(req: Request<Body>) -> impl IntoResponse {
    handle_server_fns_inner(|| {}, req).await
}

fn init_executor() {
    #[cfg(feature = "wasm")]
    let _ = any_spawner::Executor::init_wasm_bindgen();
    #[cfg(all(not(feature = "wasm"), feature = "default"))]
    let _ = any_spawner::Executor::init_tokio();
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
#[cfg_attr(
    feature = "tracing",
    tracing::instrument(level = "trace", fields(error), skip_all)
)]
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

    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let (req, parts) = generate_request_and_parts(req);

    if let Some(mut service) =
        server_fn::axum::get_server_fn_service(&path, method)
    {
        let owner = Owner::new();
        owner
            .with(|| {
                ScopedFuture::new(async move {
                    additional_context();
                    provide_context(parts);
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
                    let mut res = AxumResponse(service.run(req).await);

                    // if it accepts text/html (i.e., is a plain form post) and doesn't already have a
                    // Location set, then redirect to the Referer
                    if accepts_html {
                        if let Some(referrer) = referrer {
                            let has_location =
                                res.0.headers().get(LOCATION).is_some();
                            if !has_location {
                                *res.0.status_mut() = StatusCode::FOUND;
                                res.0.headers_mut().insert(LOCATION, referrer);
                            }
                        }
                    }

                    // apply status code and headers if user changed them
                    res.extend_response(&res_options);
                    Ok(res.0)
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
    .expect("could not build Response")
}

/// A stream of bytes of HTML.
pub type PinnedHtmlStream =
    Pin<Box<dyn Stream<Item = io::Result<Bytes>> + Send>>;

/// Returns an Axum [Handler](axum::handler::Handler) that listens for a `GET` request and tries
/// to route it using [leptos_router], serving an HTML stream of your application.
///
/// This can then be set up at an appropriate route in your application:
/// ```no_run
/// use axum::{handler::Handler, Router};
/// use leptos::{config::get_configuration, prelude::*};
/// use std::{env, net::SocketAddr};
///
/// #[component]
/// fn MyApp() -> impl IntoView {
///     view! { <main>"Hello, world!"</main> }
/// }
///
/// #[cfg(feature = "default")]
/// #[tokio::main]
/// async fn main() {
///     let conf = get_configuration(Some("Cargo.toml")).unwrap();
///     let leptos_options = conf.leptos_options;
///     let addr = leptos_options.site_addr.clone();
///
///     // build our application with a route
///     let app = Router::new().fallback(leptos_axum::render_app_to_stream(
///         || { /* your application here */ },
///     ));
///
///     // run our app with hyper
///     let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
///     axum::serve(listener, app.into_make_service())
///         .await
///         .unwrap();
/// }
///
/// # #[cfg(not(feature = "default"))]
/// # fn main() { }
/// ```
///
/// ## Provided Context Types
/// This function always provides context values including the following types:
/// - [`Parts`]
/// - [`ResponseOptions`]
/// - [`ServerMetaContext`]
#[cfg_attr(
    feature = "tracing",
    tracing::instrument(level = "trace", fields(error), skip_all)
)]
pub fn render_app_to_stream<IV>(
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
    render_app_to_stream_with_context(|| {}, app_fn)
}

/// Returns an Axum [Handler](axum::handler::Handler) that listens for a `GET` request and tries
/// to route it using [leptos_router], serving an HTML stream of your application.
/// The difference between calling this and `render_app_to_stream_with_context()` is that this
/// one respects the `SsrMode` on each Route and thus requires `Vec<AxumRouteListing>` for route checking.
/// This is useful if you are using `.leptos_routes_with_handler()`
#[cfg_attr(
    feature = "tracing",
    tracing::instrument(level = "trace", fields(error), skip_all)
)]
pub fn render_route<S, IV>(
    paths: Vec<AxumRouteListing>,
    app_fn: impl Fn() -> IV + Clone + Send + 'static,
) -> impl Fn(
    State<S>,
    Request<Body>,
) -> Pin<Box<dyn Future<Output = Response<Body>> + Send + 'static>>
       + Clone
       + Send
       + 'static
where
    IV: IntoView + 'static,
    LeptosOptions: FromRef<S>,
    S: Send + 'static,
{
    render_route_with_context(paths, || {}, app_fn)
}

/// Returns an Axum [Handler](axum::handler::Handler) that listens for a `GET` request and tries
/// to route it using [leptos_router], serving an in-order HTML stream of your application.
/// This stream will pause at each `<Suspense/>` node and wait for it to resolve before
/// sending down its HTML. The app will become interactive once it has fully loaded.
///
/// This can then be set up at an appropriate route in your application:
/// ```no_run
/// use axum::{handler::Handler, Router};
/// use leptos::{config::get_configuration, prelude::*};
/// use std::{env, net::SocketAddr};
///
/// #[component]
/// fn MyApp() -> impl IntoView {
///     view! { <main>"Hello, world!"</main> }
/// }
///
/// #[cfg(feature = "default")]
/// #[tokio::main]
/// async fn main() {
///     let conf = get_configuration(Some("Cargo.toml")).unwrap();
///     let leptos_options = conf.leptos_options;
///     let addr = leptos_options.site_addr.clone();
///
///     // build our application with a route
///     let app = Router::new().fallback(
///         leptos_axum::render_app_to_stream_in_order(|| view! { <MyApp/> }),
///     );
///
///     // run our app with hyper
///     let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
///     axum::serve(listener, app.into_make_service())
///         .await
///         .unwrap();
/// }
///
/// # #[cfg(not(feature = "default"))]
/// # fn main() { }
/// ```
///
/// ## Provided Context Types
/// This function always provides context values including the following types:
/// - [`Parts`]
/// - [`ResponseOptions`]
/// - [`ServerMetaContext`]
#[cfg_attr(
    feature = "tracing",
    tracing::instrument(level = "trace", fields(error), skip_all)
)]
pub fn render_app_to_stream_in_order<IV>(
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
    render_app_to_stream_in_order_with_context(|| {}, app_fn)
}

/// Returns an Axum [Handler](axum::handler::Handler) that listens for a `GET` request and tries
/// to route it using [leptos_router], serving an HTML stream of your application.
///
/// This version allows us to pass Axum State/Extension/Extractor or other infro from Axum or network
/// layers above Leptos itself. To use it, you'll need to write your own handler function that provides
/// the data to leptos in a closure. An example is below
/// ```
/// use axum::{
///     body::Body,
///     extract::Path,
///     http::Request,
///     response::{IntoResponse, Response},
/// };
/// use leptos::{config::LeptosOptions, context::provide_context, prelude::*};
///
/// async fn custom_handler(
///     Path(id): Path<String>,
///     req: Request<Body>,
/// ) -> Response {
///     let handler = leptos_axum::render_app_to_stream_with_context(
///         move || {
///             provide_context(id.clone());
///         },
///         || { /* your app here */ },
///     );
///     handler(req).await.into_response()
/// }
/// ```
/// Otherwise, this function is identical to [render_app_to_stream].
///
/// ## Provided Context Types
/// This function always provides context values including the following types:
/// - [`Parts`]
/// - [`ResponseOptions`]
/// - [`ServerMetaContext`]
#[cfg_attr(
    feature = "tracing",
    tracing::instrument(level = "trace", fields(error), skip_all)
)]
pub fn render_app_to_stream_with_context<IV>(
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
#[cfg_attr(
    feature = "tracing",
    tracing::instrument(level = "trace", fields(error), skip_all)
)]
pub fn render_route_with_context<S, IV>(
    paths: Vec<AxumRouteListing>,
    additional_context: impl Fn() + 'static + Clone + Send,
    app_fn: impl Fn() -> IV + Clone + Send + 'static,
) -> impl Fn(
    State<S>,
    Request<Body>,
) -> Pin<Box<dyn Future<Output = Response<Body>> + Send + 'static>>
       + Clone
       + Send
       + 'static
where
    IV: IntoView + 'static,
    LeptosOptions: FromRef<S>,
    S: Send + 'static,
{
    let ooo = render_app_to_stream_with_context(
        additional_context.clone(),
        app_fn.clone(),
    );
    let pb = render_app_to_stream_with_context_and_replace_blocks(
        additional_context.clone(),
        app_fn.clone(),
        true,
    );
    let io = render_app_to_stream_in_order_with_context(
        additional_context.clone(),
        app_fn.clone(),
    );
    let asyn = render_app_async_stream_with_context(
        additional_context.clone(),
        app_fn.clone(),
    );

    move |state, req| {
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
            SsrMode::Static(_) => {
                #[cfg(feature = "default")]
                {
                    let regenerate = listing.regenerate.clone();
                    handle_static_route(
                        additional_context.clone(),
                        app_fn.clone(),
                        regenerate,
                    )(state, req)
                }
                #[cfg(not(feature = "default"))]
                {
                    _ = state;
                    panic!(
                        "Static routes are not currently supported on WASM32 \
                         server targets."
                    );
                }
            }
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
/// - [`ServerMetaContext`]
#[cfg_attr(
    feature = "tracing",
    tracing::instrument(level = "trace", fields(error), skip_all)
)]
pub fn render_app_to_stream_with_context_and_replace_blocks<IV>(
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
    _ = replace_blocks; // TODO
    handle_response(additional_context, app_fn, |app, chunks| {
        Box::pin(async move {
            let app = if cfg!(feature = "dont-use-islands-router") {
                app.to_html_stream_out_of_order_branching()
            } else {
                app.to_html_stream_out_of_order()
            };
            Box::pin(app.chain(chunks())) as PinnedStream<String>
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
/// ```
/// use axum::{
///     body::Body,
///     extract::Path,
///     http::Request,
///     response::{IntoResponse, Response},
/// };
/// use leptos::context::provide_context;
///
/// async fn custom_handler(
///     Path(id): Path<String>,
///     req: Request<Body>,
/// ) -> Response {
///     let handler = leptos_axum::render_app_to_stream_in_order_with_context(
///         move || {
///             provide_context(id.clone());
///         },
///         || { /* your application here */ },
///     );
///     handler(req).await.into_response()
/// }
/// ```
/// Otherwise, this function is identical to [render_app_to_stream].
///
/// ## Provided Context Types
/// This function always provides context values including the following types:
/// - [`Parts`]
/// - [`ResponseOptions`]
/// - [`ServerMetaContext`]
#[cfg_attr(
    feature = "tracing",
    tracing::instrument(level = "trace", fields(error), skip_all)
)]
pub fn render_app_to_stream_in_order_with_context<IV>(
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
    handle_response(additional_context, app_fn, |app, chunks| {
        let app = if cfg!(feature = "dont-use-islands-router") {
            app.to_html_stream_in_order_branching()
        } else {
            app.to_html_stream_in_order()
        };
        Box::pin(async move {
            Box::pin(app.chain(chunks())) as PinnedStream<String>
        })
    })
}

fn handle_response<IV>(
    additional_context: impl Fn() + 'static + Clone + Send,
    app_fn: impl Fn() -> IV + Clone + Send + 'static,
    stream_builder: fn(
        IV,
        BoxedFnOnce<PinnedStream<String>>,
    ) -> PinnedFuture<PinnedStream<String>>,
) -> impl Fn(Request<Body>) -> PinnedFuture<Response<Body>> + Clone + Send + 'static
where
    IV: IntoView + 'static,
{
    move |req: Request<Body>| {
        let app_fn = app_fn.clone();
        let additional_context = additional_context.clone();
        handle_response_inner(additional_context, app_fn, req, stream_builder)
    }
}

fn handle_response_inner<IV>(
    additional_context: impl Fn() + 'static + Clone + Send,
    app_fn: impl FnOnce() -> IV + Send + 'static,
    req: Request<Body>,
    stream_builder: fn(
        IV,
        BoxedFnOnce<PinnedStream<String>>,
    ) -> PinnedFuture<PinnedStream<String>>,
) -> PinnedFuture<Response<Body>>
where
    IV: IntoView + 'static,
{
    Box::pin(async move {
        let add_context = additional_context.clone();
        let res_options = ResponseOptions::default();
        let (meta_context, meta_output) = ServerMetaContext::new();

        let additional_context = {
            let meta_context = meta_context.clone();
            let res_options = res_options.clone();
            move || {
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
            }
        };

        let res = AxumResponse::from_app(
            app_fn,
            meta_output,
            additional_context,
            res_options,
            stream_builder,
        )
        .await;

        res.0
    })
}

#[cfg_attr(
    feature = "tracing",
    tracing::instrument(level = "trace", fields(error), skip_all)
)]
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
    provide_server_redirect(redirect);
    leptos::nonce::provide_nonce();
}

/// Returns an Axum [Handler](axum::handler::Handler) that listens for a `GET` request and tries
/// to route it using [leptos_router], asynchronously rendering an HTML page after all
/// `async` resources have loaded.
///
/// This can then be set up at an appropriate route in your application:
/// ```no_run
/// use axum::{handler::Handler, Router};
/// use leptos::{config::get_configuration, prelude::*};
/// use std::{env, net::SocketAddr};
///
/// #[component]
/// fn MyApp() -> impl IntoView {
///     view! { <main>"Hello, world!"</main> }
/// }
///
/// #[cfg(feature = "default")]
/// #[tokio::main]
/// async fn main() {
///     let conf = get_configuration(Some("Cargo.toml")).unwrap();
///     let leptos_options = conf.leptos_options;
///     let addr = leptos_options.site_addr.clone();
///
///     // build our application with a route
///     let app = Router::new()
///         .fallback(leptos_axum::render_app_async(|| view! { <MyApp/> }));
///
///     // run our app with hyper
///     // `axum::Server` is a re-export of `hyper::Server`
///     let listener =
///         tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
///     axum::serve(listener, app.into_make_service())
///         .await
///         .unwrap();
/// }
///
/// # #[cfg(not(feature = "default"))]
/// # fn main() { }
/// ```
///
/// ## Provided Context Types
/// This function always provides context values including the following types:
/// - [`Parts`]
/// - [`ResponseOptions`]
/// - [`ServerMetaContext`]
#[cfg_attr(
    feature = "tracing",
    tracing::instrument(level = "trace", fields(error), skip_all)
)]
pub fn render_app_async<IV>(
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
    render_app_async_with_context(|| {}, app_fn)
}

/// Returns an Axum [Handler](axum::handler::Handler) that listens for a `GET` request and tries
/// to route it using [leptos_router], asynchronously rendering an HTML page after all
/// `async` resources have loaded.
///
/// This version allows us to pass Axum State/Extension/Extractor or other infro from Axum or network
/// layers above Leptos itself. To use it, you'll need to write your own handler function that provides
/// the data to leptos in a closure. An example is below
/// ```
/// use axum::{
///     body::Body,
///     extract::Path,
///     http::Request,
///     response::{IntoResponse, Response},
/// };
/// use leptos::context::provide_context;
///
/// async fn custom_handler(
///     Path(id): Path<String>,
///     req: Request<Body>,
/// ) -> Response {
///     let handler = leptos_axum::render_app_async_with_context(
///         move || {
///             provide_context(id.clone());
///         },
///         || { /* your application here */ },
///     );
///     handler(req).await.into_response()
/// }
/// ```
/// Otherwise, this function is identical to [render_app_to_stream].
///
/// ## Provided Context Types
/// This function always provides context values including the following types:
/// - [`Parts`]
/// - [`ResponseOptions`]
/// - [`ServerMetaContext`]
#[cfg_attr(
    feature = "tracing",
    tracing::instrument(level = "trace", fields(error), skip_all)
)]
pub fn render_app_async_stream_with_context<IV>(
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
    handle_response(additional_context, app_fn, |app, chunks| {
        Box::pin(async move {
            let app = if cfg!(feature = "dont-use-islands-router") {
                app.to_html_stream_in_order_branching()
            } else {
                app.to_html_stream_in_order()
            };
            let app = app.collect::<String>().await;
            let chunks = chunks();
            Box::pin(once(async move { app }).chain(chunks))
                as PinnedStream<String>
        })
    })
}

/// Returns an Axum [Handler](axum::handler::Handler) that listens for a `GET` request and tries
/// to route it using [leptos_router], asynchronously rendering an HTML page after all
/// `async` resources have loaded.
///
/// This version allows us to pass Axum State/Extension/Extractor or other infro from Axum or network
/// layers above Leptos itself. To use it, you'll need to write your own handler function that provides
/// the data to leptos in a closure. An example is below
/// ```
/// use axum::{
///     body::Body,
///     extract::Path,
///     http::Request,
///     response::{IntoResponse, Response},
/// };
/// use leptos::context::provide_context;
///
/// async fn custom_handler(
///     Path(id): Path<String>,
///     req: Request<Body>,
/// ) -> Response {
///     let handler = leptos_axum::render_app_async_with_context(
///         move || {
///             provide_context(id.clone());
///         },
///         || { /* your application here */ },
///     );
///     handler(req).await.into_response()
/// }
/// ```
/// Otherwise, this function is identical to [render_app_to_stream].
///
/// ## Provided Context Types
/// This function always provides context values including the following types:
/// - [`Parts`]
/// - [`ResponseOptions`]
/// - [`ServerMetaContext`]
#[cfg_attr(
    feature = "tracing",
    tracing::instrument(level = "trace", fields(error), skip_all)
)]
pub fn render_app_async_with_context<IV>(
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
    handle_response(additional_context, app_fn, async_stream_builder)
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

/// Generates a list of all routes defined in Leptos's Router in your app. We can then use this to automatically
/// create routes in Axum's Router without having to use wildcard matching or fallbacks. Takes in your root app Element
/// as an argument so it can walk you app tree. This version is tailored to generate Axum compatible paths.
#[cfg_attr(
    feature = "tracing",
    tracing::instrument(level = "trace", fields(error), skip_all)
)]
pub fn generate_route_list<IV>(
    app_fn: impl Fn() -> IV + 'static + Clone + Send,
) -> Vec<AxumRouteListing>
where
    IV: IntoView + 'static,
{
    generate_route_list_with_exclusions_and_ssg(app_fn, None).0
}

/// Generates a list of all routes defined in Leptos's Router in your app. We can then use t.clone()his to automatically
/// create routes in Axum's Router without having to use wildcard matching or fallbacks. Takes in your root app Element
/// as an argument so it can walk you app tree. This version is tailored to generate Axum compatible paths.
#[cfg_attr(
    feature = "tracing",
    tracing::instrument(level = "trace", fields(error), skip_all)
)]
pub fn generate_route_list_with_ssg<IV>(
    app_fn: impl Fn() -> IV + 'static + Clone + Send,
) -> (Vec<AxumRouteListing>, StaticRouteGenerator)
where
    IV: IntoView + 'static,
{
    generate_route_list_with_exclusions_and_ssg(app_fn, None)
}

/// Generates a list of all routes defined in Leptos's Router in your app. We can then use this to automatically
/// create routes in Axum's Router without having to use wildcard matching or fallbacks. Takes in your root app Element
/// as an argument so it can walk you app tree. This version is tailored to generate Axum compatible paths. Adding excluded_routes
/// to this function will stop `.leptos_routes()` from generating a route for it, allowing a custom handler. These need to be in Axum path format
#[cfg_attr(
    feature = "tracing",
    tracing::instrument(level = "trace", fields(error), skip_all)
)]
pub fn generate_route_list_with_exclusions<IV>(
    app_fn: impl Fn() -> IV + 'static + Clone + Send,
    excluded_routes: Option<Vec<String>>,
) -> Vec<AxumRouteListing>
where
    IV: IntoView + 'static,
{
    generate_route_list_with_exclusions_and_ssg(app_fn, excluded_routes).0
}

/// Generates a list of all routes defined in Leptos's Router in your app. We can then use this to automatically
/// create routes in Axum's Router without having to use wildcard matching or fallbacks. Takes in your root app Element
/// as an argument so it can walk you app tree. This version is tailored to generate Axum compatible paths. Adding excluded_routes
/// to this function will stop `.leptos_routes()` from generating a route for it, allowing a custom handler. These need to be in Axum path format
#[cfg_attr(
    feature = "tracing",
    tracing::instrument(level = "trace", fields(error), skip_all)
)]
pub fn generate_route_list_with_exclusions_and_ssg<IV>(
    app_fn: impl Fn() -> IV + 'static + Clone + Send,
    excluded_routes: Option<Vec<String>>,
) -> (Vec<AxumRouteListing>, StaticRouteGenerator)
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
    #[allow(unused)]
    regenerate: Vec<RegenerationFn>,
    exclude: bool,
}

trait IntoRouteListing: Sized {
    fn into_route_listing(self) -> Vec<AxumRouteListing>;
}

impl IntoRouteListing for RouteListing {
    fn into_route_listing(self) -> Vec<AxumRouteListing> {
        self.path()
            .to_vec()
            .expand_optionals()
            .into_iter()
            .map(|path| {
                let path = path.to_axum_path();
                let path = if path.is_empty() {
                    "/".to_string()
                } else {
                    path
                };
                let mode = self.mode();
                let methods = self.methods().collect();
                let regenerate = self.regenerate().into();
                AxumRouteListing {
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

impl AxumRouteListing {
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
    pub fn mode(&self) -> &SsrMode {
        &self.mode
    }

    /// The HTTP request methods this path can handle.
    pub fn methods(&self) -> impl Iterator<Item = leptos_router::Method> + '_ {
        self.methods.iter().copied()
    }
}

/// Generates a list of all routes defined in Leptos's Router in your app. We can then use this to automatically
/// create routes in Axum's Router without having to use wildcard matching or fallbacks. Takes in your root app Element
/// as an argument so it can walk you app tree. This version is tailored to generate Axum compatible paths. Adding excluded_routes
/// to this function will stop `.leptos_routes()` from generating a route for it, allowing a custom handler. These need to be in Axum path format
/// Additional context will be provided to the app Element.
#[cfg_attr(
    feature = "tracing",
    tracing::instrument(level = "trace", fields(error), skip_all)
)]
pub fn generate_route_list_with_exclusions_and_ssg_and_context<IV>(
    app_fn: impl Fn() -> IV + Clone + Send + 'static,
    excluded_routes: Option<Vec<String>>,
    additional_context: impl Fn() + Clone + Send + 'static,
) -> (Vec<AxumRouteListing>, StaticRouteGenerator)
where
    IV: IntoView + 'static,
{
    // do some basic reactive setup
    init_executor();
    let owner = Owner::new_root(Some(Arc::new(SsrSharedContext::new())));

    let routes = owner
        .with(|| {
            // stub out a path for now
            provide_context(RequestUrl::new(""));
            let (mock_parts, _) = Request::new(Body::from("")).into_parts();
            let (mock_meta, _) = ServerMetaContext::new();
            provide_contexts("", &mock_meta, mock_parts, Default::default());
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
        vec![AxumRouteListing::new(
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
            .map(|path| AxumRouteListing {
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
    #[cfg(feature = "default")]
    fn render_route<IV: IntoView + 'static>(
        path: String,
        app_fn: impl Fn() -> IV + Clone + Send + 'static,
        additional_context: impl Fn() + Clone + Send + 'static,
    ) -> impl Future<Output = (Owner, String)> {
        let (meta_context, meta_output) = ServerMetaContext::new();
        let additional_context = {
            let add_context = additional_context.clone();
            move || {
                let full_path = format!("http://leptos.dev{path}");
                let mock_req = Request::builder()
                    .method(Method::GET)
                    .header("Accept", "text/html")
                    .body(Body::empty())
                    .unwrap();
                let (mock_parts, _) = mock_req.into_parts();
                let res_options = ResponseOptions::default();
                provide_contexts(
                    &full_path,
                    &meta_context,
                    mock_parts,
                    res_options,
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
        #[cfg(feature = "default")]
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

        #[cfg(not(feature = "default"))]
        {
            _ = routes;
            _ = app_fn;
            _ = additional_context;
            Self(Box::new(|_| {
                panic!(
                    "Static routes are not currently supported on WASM32 \
                     server targets."
                );
            }))
        }
    }

    /// Generates the routes.
    pub async fn generate(self, options: &LeptosOptions) {
        (self.0)(options).await
    }
}

#[cfg(feature = "default")]
static STATIC_HEADERS: Lazy<DashMap<String, ResponseOptions>> =
    Lazy::new(DashMap::new);

#[cfg(feature = "default")]
fn was_404(owner: &Owner) -> bool {
    let resp = owner.with(|| expect_context::<ResponseOptions>());
    let status = resp.0.read().status;

    if let Some(status) = status {
        return status == StatusCode::NOT_FOUND;
    }

    false
}

#[cfg(feature = "default")]
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

#[cfg(feature = "default")]
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

#[cfg(feature = "default")]
fn handle_static_route<S, IV>(
    additional_context: impl Fn() + 'static + Clone + Send,
    app_fn: impl Fn() -> IV + Clone + Send + 'static,
    regenerate: Vec<RegenerationFn>,
) -> impl Fn(
    State<S>,
    Request<Body>,
) -> Pin<Box<dyn Future<Output = Response<Body>> + Send + 'static>>
       + Clone
       + Send
       + 'static
where
    LeptosOptions: FromRef<S>,
    S: Send + 'static,
    IV: IntoView + 'static,
{
    use tower_http::services::ServeFile;

    move |state, req| {
        let app_fn = app_fn.clone();
        let additional_context = additional_context.clone();
        let regenerate = regenerate.clone();
        Box::pin(async move {
            let options = LeptosOptions::from_ref(&state);
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
                let headers = STATIC_HEADERS.get(orig_path).map(|v| v.clone());
                (headers, None)
            };

            // if html is Some(_), it means that `was_error_response` is true and we're not
            // actually going to cache this route, just return it as HTML
            //
            // this if for thing like 404s, where we do not want to cache an endless series of
            // typos (or malicious requests)
            let mut res = AxumResponse(match html {
                Some(html) => axum::response::Html(html).into_response(),
                None => match ServeFile::new(path).oneshot(req).await {
                    Ok(res) => res.into_response(),
                    Err(err) => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Something went wrong: {err}"),
                    )
                        .into_response(),
                },
            });

            if let Some(options) = response_options {
                res.extend_response(&options);
            }

            res.0
        })
    }
}

/// This trait allows one to pass a list of routes and a render function to Axum's router, letting us avoid
/// having to use wildcards or manually define all routes in multiple places.
pub trait LeptosRoutes<S>
where
    S: Clone + Send + Sync + 'static,
    LeptosOptions: FromRef<S>,
{
    /// Adds routes to the Axum router that have either
    /// 1) been generated by `leptos_router`, or
    /// 2) handle a server function.
    fn leptos_routes<IV>(
        self,
        options: &S,
        paths: Vec<AxumRouteListing>,
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
        options: &S,
        paths: Vec<AxumRouteListing>,
        additional_context: impl Fn() + 'static + Clone + Send,
        app_fn: impl Fn() -> IV + Clone + Send + 'static,
    ) -> Self
    where
        IV: IntoView + 'static;

    /// Extends the Axum router with the given paths, and handles the requests with the given
    /// handler.
    fn leptos_routes_with_handler<H, T>(
        self,
        paths: Vec<AxumRouteListing>,
        handler: H,
    ) -> Self
    where
        H: axum::handler::Handler<T, S>,
        T: 'static;
}

trait AxumPath {
    fn to_axum_path(&self) -> String;
}

impl AxumPath for Vec<PathSegment> {
    fn to_axum_path(&self) -> String {
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
                    path.push(':');
                    path.push_str(s);
                }
                PathSegment::Splat(s) => {
                    path.push('*');
                    path.push_str(s);
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

/// The default implementation of `LeptosRoutes` which takes in a list of paths, and dispatches GET requests
/// to those paths to Leptos's renderer.
impl<S> LeptosRoutes<S> for axum::Router<S>
where
    S: Clone + Send + Sync + 'static,
    LeptosOptions: FromRef<S>,
{
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "trace", fields(error), skip_all)
    )]
    fn leptos_routes<IV>(
        self,
        state: &S,
        paths: Vec<AxumRouteListing>,
        app_fn: impl Fn() -> IV + Clone + Send + 'static,
    ) -> Self
    where
        IV: IntoView + 'static,
    {
        self.leptos_routes_with_context(state, paths, || {}, app_fn)
    }

    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "trace", fields(error), skip_all)
    )]
    fn leptos_routes_with_context<IV>(
        self,
        state: &S,
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
        let state = state.clone();
        let cx_with_state = move || {
            provide_context::<S>(state.clone());
            additional_context();
        };

        let mut router = self;

        let excluded = paths
            .iter()
            .filter(|&p| p.exclude)
            .map(|p| p.path.as_str())
            .collect::<HashSet<_>>();

        // register server functions
        for (path, method) in server_fn::axum::server_fn_paths() {
            let cx_with_state = cx_with_state.clone();
            let handler = move |req: Request<Body>| async move {
                handle_server_fns_with_context(cx_with_state, req).await
            };

            if !excluded.contains(path) {
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
        }

        // register router paths
        for listing in paths.iter().filter(|p| !p.exclude) {
            let path = listing.path();

            for method in listing.methods() {
                let cx_with_state = cx_with_state.clone();
                let cx_with_state_and_method = move || {
                    provide_context(method);
                    cx_with_state();
                };
                router = if matches!(listing.mode(), SsrMode::Static(_)) {
                    #[cfg(feature = "default")]
                    {
                        router.route(
                            path,
                            get(handle_static_route(
                                cx_with_state_and_method.clone(),
                                app_fn.clone(),
                                listing.regenerate.clone(),
                            )),
                        )
                    }
                    #[cfg(not(feature = "default"))]
                    {
                        panic!(
                            "Static routes are not currently supported on \
                             WASM32 server targets."
                        );
                    }
                } else {
                    router.route(
                    path,
                    match listing.mode() {
                        SsrMode::OutOfOrder => {
                            let s = render_app_to_stream_with_context(
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
                        _ => unreachable!()
                    },
                )
                };
            }
        }

        router
    }

    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "trace", fields(error), skip_all)
    )]
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
        for listing in paths.iter().filter(|p| !p.exclude) {
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
/// ```rust
/// use leptos::prelude::*;
///
/// #[server]
/// pub async fn request_method() -> Result<String, ServerFnError> {
///     use axum::http::Method;
///     use leptos_axum::extract;
///
///     // you can extract anything that a regular Axum extractor can extract
///     // from the head (not from the body of the request)
///     let method: Method = extract().await?;
///
///     Ok(format!("{method:?}"))
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

/// A reasonable handler for serving static files (like JS/WASM/CSS) and 404 errors.
///
/// This is provided as a convenience, but is a fairly simple function. If you need to adapt it,
/// simply reuse the source code of this function in your own application.
#[cfg(feature = "default")]
pub fn file_and_error_handler_with_context<S, IV>(
    additional_context: impl Fn() + 'static + Clone + Send,
    shell: fn(LeptosOptions) -> IV,
) -> impl Fn(
    Uri,
    State<S>,
    Request<Body>,
) -> Pin<Box<dyn Future<Output = Response<Body>> + Send + 'static>>
       + Clone
       + Send
       + 'static
where
    IV: IntoView + 'static,
    S: Send + Sync + Clone + 'static,
    LeptosOptions: FromRef<S>,
{
    move |uri: Uri, State(state): State<S>, req: Request<Body>| {
        Box::pin({
            let additional_context = additional_context.clone();
            async move {
                let options = LeptosOptions::from_ref(&state);
                let res =
                    get_static_file(uri, &options.site_root, req.headers());
                let res = res.await.unwrap();

                if res.status() == StatusCode::OK {
                    res.into_response()
                } else {
                    let mut res = handle_response_inner(
                        move || {
                            additional_context();
                            provide_context(state.clone());
                        },
                        move || shell(options),
                        req,
                        |app, chunks| {
                            Box::pin(async move {
                                let app = app
                                    .to_html_stream_in_order()
                                    .collect::<String>()
                                    .await;
                                let chunks = chunks();
                                Box::pin(once(async move { app }).chain(chunks))
                                    as PinnedStream<String>
                            })
                        },
                    )
                    .await;
                    *res.status_mut() = StatusCode::NOT_FOUND;
                    res
                }
            }
        })
    }
}

/// A reasonable handler for serving static files (like JS/WASM/CSS) and 404 errors.
///
/// This is provided as a convenience, but is a fairly simple function. If you need to adapt it,
/// simply reuse the source code of this function in your own application.
#[cfg(feature = "default")]
pub fn file_and_error_handler<S, IV>(
    shell: fn(LeptosOptions) -> IV,
) -> impl Fn(
    Uri,
    State<S>,
    Request<Body>,
) -> Pin<Box<dyn Future<Output = Response<Body>> + Send + 'static>>
       + Clone
       + Send
       + 'static
where
    IV: IntoView + 'static,
    S: Send + Sync + Clone + 'static,
    LeptosOptions: FromRef<S>,
{
    file_and_error_handler_with_context(move || (), shell)
}

#[cfg(feature = "default")]
async fn get_static_file(
    uri: Uri,
    root: &str,
    headers: &HeaderMap<HeaderValue>,
) -> Result<Response<Body>, (StatusCode, String)> {
    use axum::http::header::ACCEPT_ENCODING;

    let req = Request::builder().uri(uri);

    let req = match headers.get(ACCEPT_ENCODING) {
        Some(value) => req.header(ACCEPT_ENCODING, value),
        None => req,
    };

    let req = req.body(Body::empty()).unwrap();
    // `ServeDir` implements `tower::Service` so we can call it with `tower::ServiceExt::oneshot`
    // This path is relative to the cargo root
    match ServeDir::new(root)
        .precompressed_gzip()
        .precompressed_br()
        .oneshot(req)
        .await
    {
        Ok(res) => Ok(res.into_response()),
        Err(err) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {err}"),
        )),
    }
}
