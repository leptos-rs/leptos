#![forbid(unsafe_code)]
//! Provides functions to easily integrate Leptos with Axum.
//!
//! For more details on how to use the integrations, see the
//! [`examples`](https://github.com/leptos-rs/leptos/tree/main/examples)
//! directory in the Leptos repository.

use axum::{
    body::{Body, Bytes, Full, StreamBody},
    extract::{Path, RawQuery},
    http::{
        header::{HeaderName, HeaderValue},
        HeaderMap, Request, StatusCode,
    },
    response::IntoResponse,
    routing::{delete, get, patch, post, put},
};
use futures::{
    channel::mpsc::{Receiver, Sender},
    Future, SinkExt, Stream, StreamExt,
};
use http::{
    header, method::Method, request::Parts, uri::Uri, version::Version,
    Response,
};
use hyper::body;
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
use std::{io, pin::Pin, sync::Arc, thread::available_parallelism};
use tokio::task::LocalSet;
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

/// Adding this Struct to your Scope inside of a Server Fn or Element will allow you to override details of the Response
/// like status and add Headers/Cookies. Because Elements and Server Fns are lower in the tree than the Response generation
/// code, it needs to be wrapped in an `Arc<RwLock<>>` so that it can be surfaced.
#[derive(Debug, Clone, Default)]
pub struct ResponseOptions(pub Arc<RwLock<ResponseParts>>);

impl ResponseOptions {
    /// A less boilerplatey way to overwrite the contents of `ResponseOptions` with a new `ResponseParts`
    pub fn overwrite(&self, parts: ResponseParts) {
        let mut writable = self.0.write();
        *writable = parts
    }
    /// Set the status of the returned Response
    pub fn set_status(&self, status: StatusCode) {
        let mut writeable = self.0.write();
        let res_parts = &mut *writeable;
        res_parts.status = Some(status);
    }
    /// Insert a header, overwriting any previous value with the same key
    pub fn insert_header(&self, key: HeaderName, value: HeaderValue) {
        let mut writeable = self.0.write();
        let res_parts = &mut *writeable;
        res_parts.headers.insert(key, value);
    }
    /// Append a header, leaving any header with the same key intact
    pub fn append_header(&self, key: HeaderName, value: HeaderValue) {
        let mut writeable = self.0.write();
        let res_parts = &mut *writeable;
        res_parts.headers.append(key, value);
    }
}

/// Provides an easy way to redirect the user from within a server function. Mimicking the Remix `redirect()`,
/// it sets a StatusCode of 302 and a LOCATION header with the provided value.
/// If looking to redirect from the client, `leptos_router::use_navigate()` should be used instead
pub fn redirect(cx: leptos::Scope, path: &str) {
    if let Some(response_options) = use_context::<ResponseOptions>(cx) {
        response_options.set_status(StatusCode::FOUND);
        response_options.insert_header(
            header::LOCATION,
            header::HeaderValue::from_str(path)
                .expect("Failed to create HeaderValue"),
        );
    }
}

/// Decomposes an HTTP request into its parts, allowing you to read its headers
/// and other data without consuming the body.
#[deprecated(note = "Replaced with generate_request_and_parts() to allow for \
                     putting LeptosRequest in the Context")]
pub async fn generate_request_parts(req: Request<Body>) -> RequestParts {
    // provide request headers as context in server scope
    let (parts, body) = req.into_parts();
    let body = body::to_bytes(body).await.unwrap_or_default();
    RequestParts {
        method: parts.method,
        uri: parts.uri,
        headers: parts.headers,
        version: parts.version,
        body,
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
    let body = body::to_bytes(body).await.unwrap_or_default();
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

/// A struct to hold the [`http::request::Request`] and allow users to take ownership of it
/// Required by `Request` not being `Clone`. See
/// [this issue](https://github.com/hyperium/http/pull/574) for eventual resolution:
#[derive(Debug, Default)]
pub struct LeptosRequest<B>(Arc<RwLock<Option<Request<B>>>>);

impl<B> Clone for LeptosRequest<B> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<B> LeptosRequest<B> {
    /// Overwrite the contents of a LeptosRequest with a new `Request<B>`
    pub fn overwrite(&self, req: Option<Request<B>>) {
        let mut writable = self.0.write();
        *writable = req
    }
    /// Consume the inner `Request<B>` inside the LeptosRequest and return it
    ///```rust, ignore
    /// use axum::{
    /// RequestPartsExt,
    /// headers::Host
    /// };
    /// #[server(GetHost, "/api")]
    /// pub async fn get_host(cx: Scope) -> Result((), ServerFnError){
    ///  let req = use_context::<leptos_axum::LeptosRequest<axum::body::Body>>(cx);
    ///  if let Some(req) = req{
    ///     let owned_req = req.take_request().unwrap();
    ///     let (mut parts, _body) = owned_req.into_parts();
    ///     let host: TypedHeader<Host> = parts.extract().await().unwrap();
    ///     println!("Host: {host:#?}");
    ///  }
    /// }
    /// ```
    pub fn take_request(&self) -> Option<Request<B>> {
        let mut writable = self.0.write();
        writable.take()
    }
    /// Can be used to get immutable access to the interior fields of Request
    /// and do something with them
    pub fn with(&self, with_fn: impl Fn(Option<&Request<B>>)) {
        let readable = self.0.read();
        with_fn(readable.as_ref());
    }

    /// Can be used to mutate the fields of the Request
    pub fn update(&self, update_fn: impl Fn(Option<&mut Request<B>>)) {
        let mut writable = self.0.write();
        update_fn(writable.as_mut());
    }
}
/// Generate a wrapper for the http::Request::Request type that allows one to
/// process it, access the body, and use axum Extractors on it.
/// Required by Request not being Clone. See
/// [this issue](https://github.com/hyperium/http/pull/574) for eventual resolution:
pub async fn generate_leptos_request<B>(req: Request<B>) -> LeptosRequest<B>
where
    B: Default + std::fmt::Debug,
{
    let leptos_request = LeptosRequest::default();
    leptos_request.overwrite(Some(req));
    leptos_request
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
/// #[tokio::main]
/// async fn main() {
///     let addr = SocketAddr::from(([127, 0, 0, 1], 8082));
///
///     // build our application with a route
///     let app = Router::new()
///         .route("/api/*fn_name", post(leptos_axum::handle_server_fns));
///
///     // run our app with hyper
///     // `axum::Server` is a re-export of `hyper::Server`
///     axum::Server::bind(&addr)
///         .serve(app.into_make_service())
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
    handle_server_fns_inner(fn_name, headers, query, |_| {}, req).await
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
/// ## Provided Context Types
/// This function always provides context values including the following types:
/// - [RequestParts]
/// - [ResponseOptions]
#[tracing::instrument(level = "trace", fields(error), skip_all)]
pub async fn handle_server_fns_with_context(
    Path(fn_name): Path<String>,
    headers: HeaderMap,
    RawQuery(query): RawQuery,
    additional_context: impl Fn(leptos::Scope) + 'static + Clone + Send,
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
    additional_context: impl Fn(leptos::Scope) + 'static + Clone + Send,
    req: Request<Body>,
) -> impl IntoResponse {
    // Axum Path extractor doesn't remove the first slash from the path, while Actix does
    let fn_name = fn_name
        .strip_prefix('/')
        .map(|fn_name| fn_name.to_string())
        .unwrap_or(fn_name);

    let (tx, rx) = futures::channel::oneshot::channel();
    let pool_handle = get_leptos_pool();
    pool_handle.spawn_pinned(move || {
        async move {
            let res = if let Some(server_fn) =
                server_fn_by_path(fn_name.as_str())
            {
                let runtime = create_runtime();
                let (cx, disposer) = raw_scope_and_disposer(runtime);

                additional_context(cx);

                let (req, req_parts) = generate_request_and_parts(req).await;
                let leptos_req = generate_leptos_request(req).await; // Add this so we can get details about the Request
                provide_context(cx, req_parts.clone());
                provide_context(cx, leptos_req);
                // Add this so that we can set headers and status of the response
                provide_context(cx, ResponseOptions::default());

                let query: &Bytes = &query.unwrap_or("".to_string()).into();
                let data = match &server_fn.encoding {
                    Encoding::Url | Encoding::Cbor => &req_parts.body,
                    Encoding::GetJSON | Encoding::GetCBOR => query,
                };
                let res = match (server_fn.trait_obj)(cx, data).await {
                    Ok(serialized) => {
                        // If ResponseOptions are set, add the headers and status to the request
                        let res_options = use_context::<ResponseOptions>(cx);

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
                                .body(Full::from(data)),
                            Payload::Url(data) => res
                                .header(
                                    "Content-Type",
                                    "application/x-www-form-urlencoded",
                                )
                                .body(Full::from(data)),
                            Payload::Json(data) => res
                                .header("Content-Type", "application/json")
                                .body(Full::from(data)),
                        }
                    }
                    Err(e) => Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .body(Full::from(
                            serde_json::to_string(&e)
                                .unwrap_or_else(|_| e.to_string()),
                        )),
                };
                // clean up the scope
                disposer.dispose();
                runtime.dispose();
                res
            } else {
                Response::builder().status(StatusCode::BAD_REQUEST).body(
                    Full::from(format!(
                        "Could not find a server function at the route \
                         {fn_name}. \n\nIt's likely that you need to call \
                         ServerFn::register() on the server function type, \
                         somewhere in your `main` function."
                    )),
                )
            }
            .expect("could not build Response");

            _ = tx.send(res);
        }
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
/// fn MyApp(cx: Scope) -> impl IntoView {
///     view! { cx, <main>"Hello, world!"</main> }
/// }
///
/// # if false { // don't actually try to run a server in a doctest...
/// #[tokio::main]
/// async fn main() {
///     let conf = get_configuration(Some("Cargo.toml")).await.unwrap();
///     let leptos_options = conf.leptos_options;
///     let addr = leptos_options.site_addr.clone();
///
///     // build our application with a route
///     let app = Router::new().fallback(leptos_axum::render_app_to_stream(
///         leptos_options,
///         |cx| view! { cx, <MyApp/> },
///     ));
///
///     // run our app with hyper
///     // `axum::Server` is a re-export of `hyper::Server`
///     axum::Server::bind(&addr)
///         .serve(app.into_make_service())
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
    app_fn: impl Fn(leptos::Scope) -> IV + Clone + Send + 'static,
) -> impl Fn(
    Request<Body>,
) -> Pin<
    Box<
        dyn Future<Output = Response<StreamBody<PinnedHtmlStream>>>
            + Send
            + 'static,
    >,
> + Clone
       + Send
       + 'static
where
    IV: IntoView,
{
    render_app_to_stream_with_context(options, |_| {}, app_fn)
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
/// fn MyApp(cx: Scope) -> impl IntoView {
///     view! { cx, <main>"Hello, world!"</main> }
/// }
///
/// # if false { // don't actually try to run a server in a doctest...
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
///             |cx| view! { cx, <MyApp/> },
///         ));
///
///     // run our app with hyper
///     // `axum::Server` is a re-export of `hyper::Server`
///     axum::Server::bind(&addr)
///         .serve(app.into_make_service())
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
    app_fn: impl Fn(leptos::Scope) -> IV + Clone + Send + 'static,
) -> impl Fn(
    Request<Body>,
) -> Pin<
    Box<
        dyn Future<Output = Response<StreamBody<PinnedHtmlStream>>>
            + Send
            + 'static,
    >,
> + Clone
       + Send
       + 'static
where
    IV: IntoView,
{
    render_app_to_stream_in_order_with_context(options, |_| {}, app_fn)
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
///     move |cx| {
///         provide_context(cx, id.clone());
///     },
///     |cx| view! { cx, <TodoApp/> }
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
    additional_context: impl Fn(leptos::Scope) + 'static + Clone + Send,
    app_fn: impl Fn(leptos::Scope) -> IV + Clone + Send + 'static,
) -> impl Fn(
    Request<Body>,
) -> Pin<
    Box<
        dyn Future<Output = Response<StreamBody<PinnedHtmlStream>>>
            + Send
            + 'static,
    >,
> + Clone
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
            let local_pool = get_leptos_pool();
            let (tx, rx) = futures::channel::mpsc::channel(8);

            let current_span = tracing::Span::current();
            local_pool.spawn_pinned(move || async move {
                let app = {
                    // Need to get the path and query string of the Request
                    // For reasons that escape me, if the incoming URI protocol is https, it provides the absolute URI
                    // if http, it returns a relative path. Adding .path() seems to make it explicitly return the relative uri
                    let path = req.uri().path_and_query().unwrap().as_str();

                    let full_path = format!("http://leptos.dev{path}");
                    let (req, req_parts) = generate_request_and_parts(req).await;
                    let leptos_req = generate_leptos_request(req).await;
                    move |cx| {
                        provide_contexts(cx, full_path, req_parts,leptos_req, default_res_options);
                        app_fn(cx).into_view(cx)
                    }
                };
                let (bundle, runtime, scope) =
                    leptos::leptos_dom::ssr::render_to_stream_with_prefix_undisposed_with_context(
                        app,
                        |cx| generate_head_metadata_separated(cx).1.into(),
                        add_context,
                    );

                    forward_stream(&options, res_options2, bundle, runtime, scope, tx).await;
            }.instrument(current_span));
            async move { generate_response(res_options3, rx).await }
        })
    }
}
#[tracing::instrument(level = "info", fields(error), skip_all)]
async fn generate_response(
    res_options: ResponseOptions,
    rx: Receiver<String>,
) -> Response<StreamBody<PinnedHtmlStream>> {
    let mut stream = Box::pin(rx.map(|html| Ok(Bytes::from(html))));

    // Get the first and second chunks in the stream, which renders the app shell, and thus allows Resources to run
    let first_chunk = stream.next().await;
    let second_chunk = stream.next().await;

    // Extract the resources now that they've been rendered
    let res_options = res_options.0.read();

    let complete_stream =
        futures::stream::iter([first_chunk.unwrap(), second_chunk.unwrap()])
            .chain(stream);

    let mut res = Response::new(StreamBody::new(
        Box::pin(complete_stream) as PinnedHtmlStream
    ));

    if let Some(status) = res_options.status {
        *res.status_mut() = status
    }
    let mut res_headers = res_options.headers.clone();
    res.headers_mut().extend(res_headers.drain());

    res
}
#[tracing::instrument(level = "info", fields(error), skip_all)]
async fn forward_stream(
    options: &LeptosOptions,
    res_options2: ResponseOptions,
    bundle: impl Stream<Item = String> + 'static,
    runtime: RuntimeId,
    scope: ScopeId,
    mut tx: Sender<String>,
) {
    let cx = Scope { runtime, id: scope };
    let (head, tail) =
        html_parts_separated(options, use_context::<MetaContext>(cx).as_ref());

    _ = tx.send(head).await;
    let mut shell = Box::pin(bundle);
    while let Some(fragment) = shell.next().await {
        _ = tx.send(fragment).await;
    }
    _ = tx.send(tail.to_string()).await;

    // Extract the value of ResponseOptions from here
    let res_options = use_context::<ResponseOptions>(cx).unwrap();

    let new_res_parts = res_options.0.read().clone();

    let mut writable = res_options2.0.write();
    *writable = new_res_parts;

    runtime.dispose();

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
///     move |cx| {
///         provide_context(cx, id.clone());
///     },
///     |cx| view! { cx, <TodoApp/> }
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
    additional_context: impl Fn(leptos::Scope) + 'static + Clone + Send,
    app_fn: impl Fn(leptos::Scope) -> IV + Clone + Send + 'static,
) -> impl Fn(
    Request<Body>,
) -> Pin<
    Box<
        dyn Future<Output = Response<StreamBody<PinnedHtmlStream>>>
            + Send
            + 'static,
    >,
> + Clone
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
                let local_pool = get_leptos_pool();
                let current_span = tracing::Span::current();
                local_pool.spawn_pinned(|| async move {
                    let app = {
                        let full_path = full_path.clone();
                        let (req, req_parts) = generate_request_and_parts(req).await;
                        let leptos_req = generate_leptos_request(req).await;
                        move |cx| {
                            provide_contexts(cx, full_path, req_parts,leptos_req, default_res_options);
                            app_fn(cx).into_view(cx)
                        }
                    };

                    let (bundle, runtime, scope) =
                        leptos::ssr::render_to_stream_in_order_with_prefix_undisposed_with_context(
                            app,
                            |cx| generate_head_metadata_separated(cx).1.into(),
                            add_context,
                        );

                    forward_stream(&options, res_options2, bundle, runtime, scope, tx).await;
                }.instrument(current_span));

                generate_response(res_options3, rx).await
            }
        })
    }
}
#[tracing::instrument(level = "trace", fields(error), skip_all)]
fn provide_contexts<B: 'static + std::fmt::Debug + std::default::Default>(
    cx: Scope,
    path: String,
    req_parts: RequestParts,
    leptos_req: LeptosRequest<B>,
    default_res_options: ResponseOptions,
) {
    let integration = ServerIntegration { path };
    provide_context(cx, RouterIntegrationContext::new(integration));
    provide_context(cx, MetaContext::new());
    provide_context(cx, req_parts);
    provide_context(cx, leptos_req);
    provide_context(cx, default_res_options);
    provide_server_redirect(cx, move |path| redirect(cx, path));
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
/// fn MyApp(cx: Scope) -> impl IntoView {
///     view! { cx, <main>"Hello, world!"</main> }
/// }
///
/// # if false { // don't actually try to run a server in a doctest...
/// #[tokio::main]
/// async fn main() {
///     let conf = get_configuration(Some("Cargo.toml")).await.unwrap();
///     let leptos_options = conf.leptos_options;
///     let addr = leptos_options.site_addr.clone();
///
///     // build our application with a route
///     let app = Router::new().fallback(leptos_axum::render_app_async(
///         leptos_options,
///         |cx| view! { cx, <MyApp/> },
///     ));
///
///     // run our app with hyper
///     // `axum::Server` is a re-export of `hyper::Server`
///     axum::Server::bind(&addr)
///         .serve(app.into_make_service())
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
    app_fn: impl Fn(leptos::Scope) -> IV + Clone + Send + 'static,
) -> impl Fn(
    Request<Body>,
) -> Pin<Box<dyn Future<Output = Response<String>> + Send + 'static>>
       + Clone
       + Send
       + 'static
where
    IV: IntoView,
{
    render_app_async_with_context(options, |_| {}, app_fn)
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
///     move |cx| {
///         provide_context(cx, id.clone());
///     },
///     |cx| view! { cx, <TodoApp/> }
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
    additional_context: impl Fn(leptos::Scope) + 'static + Clone + Send,
    app_fn: impl Fn(leptos::Scope) -> IV + Clone + Send + 'static,
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
                let local_pool = get_leptos_pool();
                local_pool.spawn_pinned(move || {
                    async move {
                        let app = {
                            let full_path = full_path.clone();
                            let (req, req_parts) = generate_request_and_parts(req).await;
                            let leptos_req = generate_leptos_request(req).await;
                            move |cx| {
                                provide_contexts(cx, full_path, req_parts,leptos_req, default_res_options);
                                app_fn(cx).into_view(cx)
                            }
                        };

                        let (stream, runtime, scope) =
                            render_to_stream_in_order_with_prefix_undisposed_with_context(
                                app,
                                |_| "".into(),
                                add_context,
                            );

                        // Extract the value of ResponseOptions from here
                        let cx = leptos::Scope { runtime, id: scope };
                        let res_options =
                            use_context::<ResponseOptions>(cx).unwrap();

                        let html = build_async_response(stream, &options, runtime, scope).await;

                        let new_res_parts = res_options.0.read().clone();

                        let mut writable = res_options2.0.write();
                        *writable = new_res_parts;

                        _ = tx.send(html);
                    }
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
pub async fn generate_route_list<IV>(
    app_fn: impl FnOnce(Scope) -> IV + 'static,
) -> Vec<RouteListing>
where
    IV: IntoView + 'static,
{
    generate_route_list_with_exclusions(app_fn, None).await
}

/// Generates a list of all routes defined in Leptos's Router in your app. We can then use this to automatically
/// create routes in Axum's Router without having to use wildcard matching or fallbacks. Takes in your root app Element
/// as an argument so it can walk you app tree. This version is tailored to generate Axum compatible paths. Adding excluded_routes
/// to this function will stop `.leptos_routes()` from generating a route for it, allowing a custom handler. These need to be in Axum path format
#[tracing::instrument(level = "trace", fields(error), skip_all)]
pub async fn generate_route_list_with_exclusions<IV>(
    app_fn: impl FnOnce(Scope) -> IV + 'static,
    excluded_routes: Option<Vec<String>>,
) -> Vec<RouteListing>
where
    IV: IntoView + 'static,
{
    #[derive(Default, Clone, Debug)]
    pub struct Routes(pub Arc<RwLock<Vec<RouteListing>>>);

    let routes = Routes::default();
    let routes_inner = routes.clone();

    let local = LocalSet::new();
    // Run the local task set.

    local
        .run_until(async move {
            tokio::task::spawn_local(async move {
                let routes = leptos_router::generate_route_list_inner(app_fn);
                let mut writable = routes_inner.0.write();
                *writable = routes;
            })
            .await
            .unwrap();
        })
        .await;

    let routes = routes.0.read().to_owned();
    // Axum's Router defines Root routes as "/" not ""
    let mut routes = routes
        .into_iter()
        .map(|listing| {
            let path = listing.path();
            if path.is_empty() {
                RouteListing::new(
                    "/".to_string(),
                    listing.mode(),
                    listing.methods(),
                )
            } else {
                listing
            }
        })
        .collect::<Vec<_>>();

    if routes.is_empty() {
        vec![RouteListing::new(
            "/",
            Default::default(),
            [leptos_router::Method::Get],
        )]
    } else {
        // Routes to exclude from auto generation
        if let Some(excluded_routes) = excluded_routes {
            routes.retain(|p| !excluded_routes.iter().any(|e| e == p.path()))
        }
        routes
    }
}

/// This trait allows one to pass a list of routes and a render function to Axum's router, letting us avoid
/// having to use wildcards or manually define all routes in multiple places.
pub trait LeptosRoutes {
    fn leptos_routes<IV>(
        self,
        options: LeptosOptions,
        paths: Vec<RouteListing>,
        app_fn: impl Fn(leptos::Scope) -> IV + Clone + Send + 'static,
    ) -> Self
    where
        IV: IntoView + 'static;

    fn leptos_routes_with_context<IV>(
        self,
        options: LeptosOptions,
        paths: Vec<RouteListing>,
        additional_context: impl Fn(leptos::Scope) + 'static + Clone + Send,
        app_fn: impl Fn(leptos::Scope) -> IV + Clone + Send + 'static,
    ) -> Self
    where
        IV: IntoView + 'static;

    fn leptos_routes_with_handler<H, T>(
        self,
        paths: Vec<RouteListing>,
        handler: H,
    ) -> Self
    where
        H: axum::handler::Handler<T, (), axum::body::Body>,
        T: 'static;
}
/// The default implementation of `LeptosRoutes` which takes in a list of paths, and dispatches GET requests
/// to those paths to Leptos's renderer.
impl LeptosRoutes for axum::Router {
    #[tracing::instrument(level = "info", fields(error), skip_all)]
    fn leptos_routes<IV>(
        self,
        options: LeptosOptions,
        paths: Vec<RouteListing>,
        app_fn: impl Fn(leptos::Scope) -> IV + Clone + Send + 'static,
    ) -> Self
    where
        IV: IntoView + 'static,
    {
        self.leptos_routes_with_context(options, paths, |_| {}, app_fn)
    }

    #[tracing::instrument(level = "trace", fields(error), skip_all)]
    fn leptos_routes_with_context<IV>(
        self,
        options: LeptosOptions,
        paths: Vec<RouteListing>,
        additional_context: impl Fn(leptos::Scope) + 'static + Clone + Send,
        app_fn: impl Fn(leptos::Scope) -> IV + Clone + Send + 'static,
    ) -> Self
    where
        IV: IntoView + 'static,
    {
        let mut router = self;
        for listing in paths.iter() {
            let path = listing.path();

            for method in listing.methods() {
                router = router.route(
                    path,
                    match listing.mode() {
                        SsrMode::OutOfOrder => {
                            let s = render_app_to_stream_with_context(
                                options.clone(),
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
                        SsrMode::InOrder => {
                            let s = render_app_to_stream_in_order_with_context(
                                options.clone(),
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
                                options.clone(),
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
                );
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
        H: axum::handler::Handler<T, (), axum::body::Body>,
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
