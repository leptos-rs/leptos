#![forbid(unsafe_code)]

//! Provides functions to easily integrate Leptos with Viz.
//!
//! For more details on how to use the integrations, see the
//! [`examples`](https://github.com/leptos-rs/leptos/tree/main/examples)
//! directory in the Leptos repository.

use futures::{
    channel::mpsc::{Receiver, Sender},
    Future, SinkExt, Stream, StreamExt,
};
use http::{header, method::Method, uri::Uri, version::Version, StatusCode};
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
use parking_lot::RwLock;
use std::{pin::Pin, sync::Arc};
use tokio::task::spawn_blocking;
use viz::{
    headers::{HeaderMap, HeaderName, HeaderValue},
    Body, Bytes, Error, Handler, IntoResponse, Request, RequestExt, Response,
    ResponseExt, Result, Router,
};

/// A struct to hold the parts of the incoming Request. Since `http::Request` isn't cloneable, we're forced
/// to construct this for Leptos to use in viz
#[derive(Debug, Clone)]
pub struct RequestParts {
    pub version: Version,
    pub method: Method,
    pub uri: Uri,
    pub headers: HeaderMap<HeaderValue>,
    pub body: Bytes,
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
/// and other data without consuming the body.
pub async fn generate_request_parts(req: Request) -> RequestParts {
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

/// A Viz handlers to listens for a request with Leptos server function arguments in the body,
/// run the server function if found, and return the resulting [Response].
///
/// This can then be set up at an appropriate route in your application:
///
/// ```
/// use leptos::*;
/// use std::net::SocketAddr;
/// use viz::{Router, ServiceMaker};
///
/// # if false { // don't actually try to run a server in a doctest...
/// #[tokio::main]
/// async fn main() {
///     let addr = SocketAddr::from(([127, 0, 0, 1], 8082));
///
///     // build our application with a route
///     let app =
///         Router::new().post("/api/:fn_name*", leptos_viz::handle_server_fns);
///
///     // run our app with hyper
///     // `viz::Server` is a re-export of `hyper::Server`
///     viz::Server::bind(&addr)
///         .serve(ServiceMaker::from(app))
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
pub async fn handle_server_fns(req: Request) -> Result<Response> {
    handle_server_fns_inner(req, || {}).await
}

/// A Viz handlers to listens for a request with Leptos server function arguments in the body,
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
pub async fn handle_server_fns_with_context(
    req: Request,
    additional_context: impl Fn() + Clone + Send + 'static,
) -> Result<Response> {
    handle_server_fns_inner(req, additional_context).await
}

async fn handle_server_fns_inner(
    req: Request,
    additional_context: impl Fn() + Clone + Send + 'static,
) -> Result<Response> {
    let fn_name = req.params::<String>()?;
    let headers = req.headers().clone();
    let query = req.query_string().unwrap_or("").to_owned().into();
    let (tx, rx) = futures::channel::oneshot::channel();
    spawn_blocking({
        move || {
            tokio::runtime::Runtime::new()
                .expect("couldn't spawn runtime")
                .block_on({
                    async move {
                        let res = if let Some(server_fn) =
                            server_fn_by_path(fn_name.as_str())
                        {
                            let runtime = create_runtime();

                            additional_context();

                            let req_parts = generate_request_parts(req).await;
                            // Add this so we can get details about the Request
                            provide_context(req_parts.clone());
                            // Add this so that we can set headers and status of the response
                            provide_context(ResponseOptions::default());

                            let data = match &server_fn.encoding() {
                                Encoding::Url | Encoding::Cbor => {
                                    &req_parts.body
                                }
                                Encoding::GetJSON | Encoding::GetCBOR => &query,
                            };

                            let res = match server_fn.call((), data).await {
                                Ok(serialized) => {
                                    // If ResponseOptions are set, add the headers and status to the request
                                    let res_options =
                                        use_context::<ResponseOptions>();

                                    // if this is Accept: application/json then send a serialized JSON response
                                    let accept_header = headers
                                        .get("Accept")
                                        .and_then(|value| value.to_str().ok());
                                    let mut res = Response::builder();

                                    // Add headers from ResponseParts if they exist. These should be added as long
                                    // as the server function returns an OK response
                                    let res_options_outer =
                                        res_options.unwrap().0;
                                    let res_options_inner =
                                        res_options_outer.read();
                                    let (status, mut res_headers) = (
                                        res_options_inner.status,
                                        res_options_inner.headers.clone(),
                                    );

                                    if let Some(header_ref) = res.headers_mut()
                                    {
                                        header_ref.extend(res_headers.drain());
                                    };

                                    if accept_header == Some("application/json")
                                        || accept_header
                                            == Some(
                                                "application/\
                                                 x-www-form-urlencoded",
                                            )
                                        || accept_header
                                            == Some("application/cbor")
                                    {
                                        res = res.status(StatusCode::OK);
                                    }
                                    // otherwise, it's probably a <form> submit or something: redirect back to the referrer
                                    else {
                                        let referer = headers
                                            .get("Referer")
                                            .and_then(|value| {
                                                value.to_str().ok()
                                            })
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
                                    match serialized {
                                        Payload::Binary(data) => res
                                            .header(
                                                header::CONTENT_TYPE,
                                                "application/cbor",
                                            )
                                            .body(Body::from(data)),
                                        Payload::Url(data) => res
                                            .header(
                                                header::CONTENT_TYPE,
                                                "application/\
                                                 x-www-form-urlencoded",
                                            )
                                            .body(Body::from(data)),
                                        Payload::Json(data) => res
                                            .header(
                                                header::CONTENT_TYPE,
                                                "application/json",
                                            )
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
                            runtime.dispose();
                            res
                        } else {
                            Response::builder()
                                .status(StatusCode::BAD_REQUEST)
                                .body(Body::from(format!(
                                    "Could not find a server function at the \
                                     route {fn_name}. \n\nIt's likely that \
                                     either
                                    1. The API prefix you specify in the \
                                     `#[server]` macro doesn't match the \
                                     prefix at which your server function \
                                     handler is mounted, or \n2. You are on a \
                                     platform that doesn't support automatic \
                                     server function registration and you \
                                     need to call \
                                     ServerFn::register_explicit() on the \
                                     server function type, somewhere in your \
                                     `main` function.",
                                )))
                        }
                        .expect("could not build Response");

                        _ = tx.send(res);
                    }
                })
        }
    });

    rx.await.map_err(Error::normal)
}
/// Returns a Viz [Handler] that listens for a `GET` request and tries
/// to route it using [leptos_router], serving an HTML stream of your application.
///
/// The provides a [MetaContext] and a [RouterIntegrationContext] to app’s context before
/// rendering it, and includes any meta tags injected using [leptos_meta].
///
/// The HTML stream is rendered using [render_to_stream], and includes everything described in
/// the documentation for that function.
///
/// This can then be set up at an appropriate route in your application:
/// ```
/// use leptos::*;
/// use leptos_config::get_configuration;
/// use std::{env, net::SocketAddr};
/// use viz::{Router, ServiceMaker};
///
/// #[component]
/// fn MyApp() -> impl IntoView {
///     view! { <main>"Hello, world!"</main> }
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
///     let app = Router::new().any(
///         "*",
///         leptos_viz::render_app_to_stream(
///             leptos_options,
///             || view! { <MyApp/> },
///         ),
///     );
///
///     // run our app with hyper
///     // `viz::Server` is a re-export of `hyper::Server`
///     viz::Server::bind(&addr)
///         .serve(ServiceMaker::from(app))
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
/// - [MetaContext]
/// - [RouterIntegrationContext]
pub fn render_app_to_stream<IV>(
    options: LeptosOptions,
    app_fn: impl Fn() -> IV + Clone + Send + 'static,
) -> impl Fn(
    Request,
) -> Pin<Box<dyn Future<Output = Result<Response>> + Send + 'static>>
       + Clone
       + Send
       + 'static
where
    IV: IntoView,
{
    render_app_to_stream_with_context(options, || {}, app_fn)
}

/// Returns a Viz [Handler] that listens for a `GET` request and tries
/// to route it using [leptos_router], serving an HTML stream of your application.
/// This stream will pause at each `<Suspense/>` node and wait for it to resolve before
/// sending down its HTML. The app will become interactive once it has fully loaded.
///
/// The provides a [MetaContext] and a [RouterIntegrationContext] to app’s context before
/// rendering it, and includes any meta tags injected using [leptos_meta].
///
/// The HTML stream is rendered using [render_to_stream], and includes everything described in
/// the documentation for that function.
///
/// This can then be set up at an appropriate route in your application:
/// ```
/// use leptos::*;
/// use leptos_config::get_configuration;
/// use std::{env, net::SocketAddr};
/// use viz::{Router, ServiceMaker};
///
/// #[component]
/// fn MyApp() -> impl IntoView {
///     view! { <main>"Hello, world!"</main> }
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
///     let app = Router::new().any(
///         "*",
///         leptos_viz::render_app_to_stream_in_order(
///             leptos_options,
///             || view! { <MyApp/> },
///         ),
///     );
///
///     // run our app with hyper
///     // `viz::Server` is a re-export of `hyper::Server`
///     viz::Server::bind(&addr)
///         .serve(ServiceMaker::from(app))
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
/// - [MetaContext]
/// - [RouterIntegrationContext]
pub fn render_app_to_stream_in_order<IV>(
    options: LeptosOptions,
    app_fn: impl Fn() -> IV + Clone + Send + 'static,
) -> impl Fn(
    Request,
) -> Pin<Box<dyn Future<Output = Result<Response>> + Send + 'static>>
       + Clone
       + Send
       + 'static
where
    IV: IntoView,
{
    render_app_to_stream_in_order_with_context(options, || {}, app_fn)
}

/// Returns a Viz [Handler] that listens for a `GET` request and tries
/// to route it using [leptos_router], serving an HTML stream of your application.
///
/// This version allows us to pass Viz State/Extractor or other infro from Viz or network
/// layers above Leptos itself. To use it, you'll need to write your own handler function that provides
/// the data to leptos in a closure. An example is below
/// ```ignore
/// async fn custom_handler(req: Request) -> Result<Response> {
///     let id = req.params::<String>()?;
///     let options = &*req.state::<Arc<LeptosOptions>>().ok_or(Error::Responder(Response::text("missing state type LeptosOptions")))?;
///     let handler = leptos_viz::render_app_to_stream_with_context(options.clone(),
///     move || {
///         provide_context(id.clone());
///     },
///     || view! { <TodoApp/> }
/// );
///     handler(req).await
/// }
/// ```
/// Otherwise, this function is identical to [render_app_to_stream].
///
/// ## Provided Context Types
/// This function always provides context values including the following types:
/// - [RequestParts]
/// - [ResponseOptions]
/// - [MetaContext]
/// - [RouterIntegrationContext]
pub fn render_app_to_stream_with_context<IV>(
    options: LeptosOptions,
    additional_context: impl Fn() + Clone + Send + 'static,
    app_fn: impl Fn() -> IV + Clone + Send + 'static,
) -> impl Fn(
    Request,
) -> Pin<Box<dyn Future<Output = Result<Response>> + Send + 'static>>
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

/// Returns a Viz [Handler] that listens for a `GET` request and tries
/// to route it using [leptos_router], serving an HTML stream of your application.
///
/// This version allows us to pass Viz State/Extractor or other infro from Viz or network
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
/// - [MetaContext]
/// - [RouterIntegrationContext]
pub fn render_app_to_stream_with_context_and_replace_blocks<IV>(
    options: LeptosOptions,
    additional_context: impl Fn() + Clone + Send + 'static,
    app_fn: impl Fn() -> IV + Clone + Send + 'static,
    replace_blocks: bool,
) -> impl Fn(
    Request,
) -> Pin<Box<dyn Future<Output = Result<Response>> + Send + 'static>>
       + Clone
       + Send
       + 'static
where
    IV: IntoView,
{
    move |req: Request| {
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

                spawn_blocking({
                    let app_fn = app_fn.clone();
                    let add_context = add_context.clone();
                    move || {
                        tokio::runtime::Runtime::new()
                            .expect("couldn't spawn runtime")
                            .block_on({
                                let app_fn = app_fn.clone();
                                let add_context = add_context.clone();
                                async move {
                                    tokio::task::LocalSet::new()
                                        .run_until(async {
                                            let app = {
                                                let full_path = full_path.clone();
                                                let req_parts = generate_request_parts(req).await;
                                                move || {
                                                    provide_contexts(full_path, req_parts, default_res_options);
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
                                        })
                                        .await;
                                }
                            });
                    }
                });

                generate_response(res_options3, rx).await
            }
        })
    }
}

async fn generate_response(
    res_options: ResponseOptions,
    rx: Receiver<String>,
) -> Result<Response> {
    let mut stream =
        Box::pin(rx.map(|html| Ok::<_, std::io::Error>(Bytes::from(html))));

    // Get the first and second chunks in the stream, which renders the app shell, and thus allows Resources to run
    let first_chunk = stream.next().await;
    let second_chunk = stream.next().await;

    // Extract the resources now that they've been rendered
    let res_options = res_options.0.read();

    let complete_stream =
        futures::stream::iter([first_chunk.unwrap(), second_chunk.unwrap()])
            .chain(stream);

    let mut res = Response::stream(complete_stream);

    if let Some(status) = res_options.status {
        *res.status_mut() = status
    }
    let mut res_headers = res_options.headers.clone();
    res.headers_mut().extend(res_headers.drain());

    Ok(res)
}

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

/// Returns a Viz [Handler] that listens for a `GET` request and tries
/// to route it using [leptos_router], serving an in-order HTML stream of your application.
/// This stream will pause at each `<Suspense/>` node and wait for it to resolve before
/// sending down its HTML. The app will become interactive once it has fully loaded.
///
/// This version allows us to pass Viz State/Extractor or other infro from Viz or network
/// layers above Leptos itself. To use it, you'll need to write your own handler function that provides
/// the data to leptos in a closure. An example is below
/// ```ignore
/// async fn custom_handler(req: Request) -> Result<Response> {
///     let id = req.params::<String>()?;
///     let options = &*req.state::<Arc<LeptosOptions>>().ok_or(StateError::new::<Arc<LeptosOptions>>())?;
///     let handler = leptos_viz::render_app_to_stream_in_order_with_context(options.clone(),
///     move || {
///         provide_context(id.clone());
///     },
///     || view! { <TodoApp/> }
/// );
///     handler(req).await
/// }
/// ```
/// Otherwise, this function is identical to [render_app_to_stream].
///
/// ## Provided Context Types
/// This function always provides context values including the following types:
/// - [RequestParts]
/// - [ResponseOptions]
/// - [MetaContext]
/// - [RouterIntegrationContext]
pub fn render_app_to_stream_in_order_with_context<IV>(
    options: LeptosOptions,
    additional_context: impl Fn() + 'static + Clone + Send,
    app_fn: impl Fn() -> IV + Clone + Send + 'static,
) -> impl Fn(
    Request,
) -> Pin<Box<dyn Future<Output = Result<Response>> + Send + 'static>>
       + Clone
       + Send
       + 'static
where
    IV: IntoView,
{
    move |req: Request| {
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

                spawn_blocking({
                    let app_fn = app_fn.clone();
                    let add_context = add_context.clone();
                    move || {
                        tokio::runtime::Runtime::new()
                            .expect("couldn't spawn runtime")
                            .block_on({
                                let app_fn = app_fn.clone();
                                let add_context = add_context.clone();
                                async move {
                                    tokio::task::LocalSet::new()
                                        .run_until(async {
                                            let app = {
                                                let full_path = full_path.clone();
                                                let req_parts = generate_request_parts(req).await;
                                                move || {
                                                    provide_contexts(full_path, req_parts, default_res_options);
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
                                        })
                                        .await;
                                }
                            });
                    }
                });

                generate_response(res_options3, rx).await
            }
        })
    }
}

fn provide_contexts(
    path: String,
    req_parts: RequestParts,
    default_res_options: ResponseOptions,
) {
    let integration = ServerIntegration { path };
    provide_context(RouterIntegrationContext::new(integration));
    provide_context(MetaContext::new());
    provide_context(req_parts);
    provide_context(default_res_options);
    provide_server_redirect(redirect);
    #[cfg(feature = "nonce")]
    leptos::nonce::provide_nonce();
}

/// Returns a Viz [Handler] that listens for a `GET` request and tries
/// to route it using [leptos_router], asynchronously rendering an HTML page after all
/// `async` [Resource]s have loaded.
///
/// The provides a [MetaContext] and a [RouterIntegrationContext] to app’s context before
/// rendering it, and includes any meta tags injected using [leptos_meta].
///
/// The HTML stream is rendered using [render_to_string_async], and includes everything described in
/// the documentation for that function.
///
/// This can then be set up at an appropriate route in your application:
/// ```
/// use leptos::*;
/// use leptos_config::get_configuration;
/// use std::{env, net::SocketAddr};
/// use viz::{Router, ServiceMaker};
///
/// #[component]
/// fn MyApp() -> impl IntoView {
///     view! { <main>"Hello, world!"</main> }
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
///     let app = Router::new().any(
///         "*",
///         leptos_viz::render_app_async(leptos_options, || view! { <MyApp/> }),
///     );
///
///     // run our app with hyper
///     // `viz::Server` is a re-export of `hyper::Server`
///     viz::Server::bind(&addr)
///         .serve(ServiceMaker::from(app))
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
/// - [MetaContext]
/// - [RouterIntegrationContext]
pub fn render_app_async<IV>(
    options: LeptosOptions,
    app_fn: impl Fn() -> IV + Clone + Send + 'static,
) -> impl Fn(
    Request,
) -> Pin<Box<dyn Future<Output = Result<Response>> + Send + 'static>>
       + Clone
       + Send
       + 'static
where
    IV: IntoView,
{
    render_app_async_with_context(options, || {}, app_fn)
}

/// Returns a Viz [Handler] that listens for a `GET` request and tries
/// to route it using [leptos_router], asynchronously rendering an HTML page after all
/// `async` [Resource]s have loaded.
///
/// This version allows us to pass Viz State/Extractor or other infro from Viz or network
/// layers above Leptos itself. To use it, you'll need to write your own handler function that provides
/// the data to leptos in a closure. An example is below
/// ```ignore
/// async fn custom_handler(req: Request) -> Result<Response> {
///     let id = req.params::<String>()?;
///     let options = &*req.state::<Arc<LeptosOptions>>().ok_or(StateError::new::<Arc<LeptosOptions>>())?;
///     let handler = leptos_viz::render_app_async_with_context(options.clone(),
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
/// - [MetaContext]
/// - [RouterIntegrationContext]
pub fn render_app_async_with_context<IV>(
    options: LeptosOptions,
    additional_context: impl Fn() + 'static + Clone + Send,
    app_fn: impl Fn() -> IV + Clone + Send + 'static,
) -> impl Fn(
    Request,
) -> Pin<Box<dyn Future<Output = Result<Response>> + Send + 'static>>
       + Clone
       + Send
       + 'static
where
    IV: IntoView,
{
    move |req: Request| {
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

                spawn_blocking({
                    let app_fn = app_fn.clone();
                    let add_context = add_context.clone();
                    move || {
                        tokio::runtime::Runtime::new()
                            .expect("couldn't spawn runtime")
                            .block_on({
                                let app_fn = app_fn.clone();
                                let add_context = add_context.clone();
                                async move {
                                    tokio::task::LocalSet::new()
                                        .run_until(async {
                                            let app = {
                                                let full_path = full_path.clone();
                                                let req_parts = generate_request_parts(req).await;
                                                move || {
                                                    provide_contexts(full_path, req_parts, default_res_options);
                                                    app_fn().into_view()
                                                }
                                            };

                                            let (stream, runtime) =
                                                render_to_stream_with_prefix_undisposed_with_context(
                                                    app,
                                                    || "".into(),
                                                    add_context,
                                                );

                                            // Extract the value of ResponseOptions from here
                                            let res_options =
                                                use_context::<ResponseOptions>().unwrap();

                                            let html = build_async_response(stream, &options, runtime).await;

                                            let new_res_parts = res_options.0.read().clone();

                                            let mut writable = res_options2.0.write();
                                            *writable = new_res_parts;

                                            _ = tx.send(html);
                                        })
                                        .await;
                                }
                            });
                    }
                });

                let html = rx.await.expect("to complete HTML rendering");

                let mut res = Response::html(html);

                let res_options = res_options3.0.read();

                if let Some(status) = res_options.status {
                    *res.status_mut() = status
                }
                let mut res_headers = res_options.headers.clone();
                res.headers_mut().extend(res_headers.drain());

                Ok(res)
            }
        })
    }
}

/// Generates a list of all routes defined in Leptos's Router in your app. We can then use this to automatically
/// create routes in Viz's Router without having to use wildcard matching or fallbacks. Takes in your root app Element
/// as an argument so it can walk you app tree. This version is tailored to generate Viz compatible paths.
pub fn generate_route_list<IV>(
    app_fn: impl Fn() -> IV + 'static + Clone,
) -> Vec<RouteListing>
where
    IV: IntoView + 'static,
{
    generate_route_list_with_exclusions_and_ssg(app_fn, None).0
}

/// Generates a list of all routes defined in Leptos's Router in your app. We can then use this to automatically
/// create routes in Viz's Router without having to use wildcard matching or fallbacks. Takes in your root app Element
/// as an argument so it can walk you app tree. This version is tailored to generate Viz compatible paths.
pub fn generate_route_list_with_ssg<IV>(
    app_fn: impl Fn() -> IV + 'static + Clone,
) -> (Vec<RouteListing>, StaticDataMap)
where
    IV: IntoView + 'static,
{
    generate_route_list_with_exclusions_and_ssg(app_fn, None)
}

/// Generates a list of all routes defined in Leptos's Router in your app. We can then use this to automatically
/// create routes in Viz's Router without having to use wildcard matching or fallbacks. Takes in your root app Element
/// as an argument so it can walk you app tree. This version is tailored to generate Viz compatible paths.
pub fn generate_route_list_with_exclusions<IV>(
    app_fn: impl Fn() -> IV + 'static + Clone,
    excluded_routes: Option<Vec<String>>,
) -> Vec<RouteListing>
where
    IV: IntoView + 'static,
{
    generate_route_list_with_exclusions_and_ssg(app_fn, excluded_routes).0
}

/// Generates a list of all routes defined in Leptos's Router in your app. We can then use this to automatically
/// create routes in Viz's Router without having to use wildcard matching or fallbacks. Takes in your root app Element
/// as an argument so it can walk you app tree. This version is tailored to generate Viz compatible paths.
pub fn generate_route_list_with_exclusions_and_ssg<IV>(
    app_fn: impl Fn() -> IV + 'static + Clone,
    excluded_routes: Option<Vec<String>>,
) -> (Vec<RouteListing>, StaticDataMap)
where
    IV: IntoView + 'static,
{
    generate_route_list_with_exclusions_and_ssg_and_context(
        app_fn,
        excluded_routes,
        || {},
    )
}

/// Generates a list of all routes defined in Leptos's Router in your app. We can then use this to automatically
/// create routes in Viz's Router without having to use wildcard matching or fallbacks. Takes in your root app Element
/// as an argument so it can walk you app tree. This version is tailored to generate Viz compatible paths.
/// Additional context will be provided to the app Element.
pub fn generate_route_list_with_exclusions_and_ssg_and_context<IV>(
    app_fn: impl Fn() -> IV + 'static + Clone,
    excluded_routes: Option<Vec<String>>,
    additional_context: impl Fn() + 'static + Clone,
) -> (Vec<RouteListing>, StaticDataMap)
where
    IV: IntoView + 'static,
{
    let (routes, static_data_map) =
        leptos_router::generate_route_list_inner_with_context(
            app_fn,
            additional_context,
        );
    // Viz's Router defines Root routes as "/" not ""
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
            if let Some(excluded_routes) = excluded_routes {
                routes
                    .retain(|p| !excluded_routes.iter().any(|e| e == p.path()))
            }
            routes
        },
        static_data_map,
    )
}

fn handle_static_response<IV>(
    path: String,
    options: LeptosOptions,
    app_fn: impl Fn() -> IV + Clone + Send + Sync + 'static,
    additional_context: impl Fn() + Clone + Send + Sync + 'static,
    res: StaticResponse,
) -> Pin<Box<dyn Future<Output = Result<Response>> + 'static>>
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
                let mut res = Response::html(body);
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
                Ok(res)
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

fn static_route<IV>(
    router: Router,
    path: &str,
    options: LeptosOptions,
    app_fn: impl Fn() -> IV + Clone + Send + Sync + 'static,
    additional_context: impl Fn() + Clone + Send + Sync + 'static,
    method: leptos_router::Method,
    mode: StaticMode,
) -> Router
where
    IV: IntoView + 'static,
{
    match mode {
        StaticMode::Incremental => {
            let handler = move |req: Request| {
                Box::pin({
                    let path = req.path().to_string();
                    let options = options.clone();
                    let app_fn = app_fn.clone();
                    let additional_context = additional_context.clone();

                    async move {
                        let (tx, rx) = futures::channel::oneshot::channel();
                        spawn_blocking(move || {
                            let path = path.clone();
                            let options = options.clone();
                            let app_fn = app_fn.clone();
                            let additional_context = additional_context.clone();
                            tokio::runtime::Runtime::new()
                                .expect("couldn't spawn runtime")
                                .block_on({
                                    let path = path.clone();
                                    let options = options.clone();
                                    let app_fn = app_fn.clone();
                                    let additional_context =
                                        additional_context.clone();
                                    async move {
                                        tokio::task::LocalSet::new().run_until(async {
                                            let res = incremental_static_route(
                                                tokio::fs::read_to_string(
                                                    static_file_path(
                                                        &options,
                                                    &path,
                                                    ),
                                                )
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
                                        }).await;
                                    }
                                })
                        });

                        rx.await.expect("to complete HTML rendering")
                    }
                })
            };
            match method {
                leptos_router::Method::Get => router.get(path, handler),
                leptos_router::Method::Post => router.post(path, handler),
                leptos_router::Method::Put => router.put(path, handler),
                leptos_router::Method::Delete => router.delete(path, handler),
                leptos_router::Method::Patch => router.patch(path, handler),
            }
        }
        StaticMode::Upfront => {
            let handler = move |req: Request| {
                Box::pin({
                    let path = req.path().to_string();
                    let options = options.clone();
                    let app_fn = app_fn.clone();
                    let additional_context = additional_context.clone();

                    async move {
                        let (tx, rx) = futures::channel::oneshot::channel();
                        spawn_blocking(move || {
                            let path = path.clone();
                            let options = options.clone();
                            let app_fn = app_fn.clone();
                            let additional_context = additional_context.clone();
                            tokio::runtime::Runtime::new()
                                .expect("couldn't spawn runtime")
                                .block_on({
                                    let path = path.clone();
                                    let options = options.clone();
                                    let app_fn = app_fn.clone();
                                    let additional_context =
                                        additional_context.clone();
                                    async move {
                                        tokio::task::LocalSet::new()
                                            .run_until(async {
                                                let res = upfront_static_route(
                                                    tokio::fs::read_to_string(
                                                        static_file_path(
                                                            &options, &path,
                                                        ),
                                                    )
                                                    .await,
                                                );
                                                let res =
                                                    handle_static_response(
                                                        path.clone(),
                                                        options,
                                                        app_fn,
                                                        additional_context,
                                                        res,
                                                    )
                                                    .await;

                                                let _ = tx.send(res);
                                            })
                                            .await;
                                    }
                                })
                        });

                        rx.await.expect("to complete HTML rendering")
                    }
                })
            };
            match method {
                leptos_router::Method::Get => router.get(path, handler),
                leptos_router::Method::Post => router.post(path, handler),
                leptos_router::Method::Put => router.put(path, handler),
                leptos_router::Method::Delete => router.delete(path, handler),
                leptos_router::Method::Patch => router.patch(path, handler),
            }
        }
    }
}

/// This trait allows one to pass a list of routes and a render function to Viz's router, letting us avoid
/// having to use wildcards or manually define all routes in multiple places.
pub trait LeptosRoutes {
    fn leptos_routes<IV>(
        self,
        options: LeptosOptions,
        paths: Vec<RouteListing>,
        app_fn: impl Fn() -> IV + Clone + Send + Sync + 'static,
    ) -> Self
    where
        IV: IntoView + 'static;

    fn leptos_routes_with_context<IV>(
        self,
        options: LeptosOptions,
        paths: Vec<RouteListing>,
        additional_context: impl Fn() + Clone + Send + Sync + 'static,
        app_fn: impl Fn() -> IV + Clone + Send + Sync + 'static,
    ) -> Self
    where
        IV: IntoView + 'static;

    fn leptos_routes_with_handler<H, O>(
        self,
        paths: Vec<RouteListing>,
        handler: H,
    ) -> Self
    where
        H: Handler<Request, Output = Result<O>> + Clone,
        O: IntoResponse + Send + Sync + 'static;
}
/// The default implementation of `LeptosRoutes` which takes in a list of paths, and dispatches GET requests
/// to those paths to Leptos's renderer.
impl LeptosRoutes for Router {
    fn leptos_routes<IV>(
        self,
        options: LeptosOptions,
        paths: Vec<RouteListing>,
        app_fn: impl Fn() -> IV + Clone + Send + Sync + 'static,
    ) -> Self
    where
        IV: IntoView + 'static,
    {
        self.leptos_routes_with_context(options, paths, || {}, app_fn)
    }

    fn leptos_routes_with_context<IV>(
        self,
        options: LeptosOptions,
        paths: Vec<RouteListing>,
        additional_context: impl Fn() + Clone + Send + Sync + 'static,
        app_fn: impl Fn() -> IV + Clone + Send + Sync + 'static,
    ) -> Self
    where
        IV: IntoView + 'static,
    {
        paths.iter().fold(self, |router, listing| {
            let path = listing.path();
            let mode = listing.mode();

            listing.methods().fold(router, |router, method| {
                if let Some(static_mode) = listing.static_mode() {
                    static_route(
                        router,
                        path,
                        options.clone(),
                        app_fn.clone(),
                        additional_context.clone(),
                        method,
                        static_mode,
                    )
                } else {
                    match mode {
                        SsrMode::OutOfOrder => {
                            let s = render_app_to_stream_with_context(
                                options.clone(),
                                additional_context.clone(),
                                app_fn.clone(),
                            );
                            match method {
                                leptos_router::Method::Get => {
                                    router.get(path, s)
                                }
                                leptos_router::Method::Post => {
                                    router.post(path, s)
                                }
                                leptos_router::Method::Put => {
                                    router.put(path, s)
                                }
                                leptos_router::Method::Delete => {
                                    router.delete(path, s)
                                }
                                leptos_router::Method::Patch => {
                                    router.patch(path, s)
                                }
                            }
                        }
                        SsrMode::PartiallyBlocked => {
                            let s =
                        render_app_to_stream_with_context_and_replace_blocks(
                            options.clone(),
                            additional_context.clone(),
                            app_fn.clone(),
                            true,
                        );
                            match method {
                                leptos_router::Method::Get => {
                                    router.get(path, s)
                                }
                                leptos_router::Method::Post => {
                                    router.post(path, s)
                                }
                                leptos_router::Method::Put => {
                                    router.put(path, s)
                                }
                                leptos_router::Method::Delete => {
                                    router.delete(path, s)
                                }
                                leptos_router::Method::Patch => {
                                    router.patch(path, s)
                                }
                            }
                        }
                        SsrMode::InOrder => {
                            let s = render_app_to_stream_in_order_with_context(
                                options.clone(),
                                additional_context.clone(),
                                app_fn.clone(),
                            );
                            match method {
                                leptos_router::Method::Get => {
                                    router.get(path, s)
                                }
                                leptos_router::Method::Post => {
                                    router.post(path, s)
                                }
                                leptos_router::Method::Put => {
                                    router.put(path, s)
                                }
                                leptos_router::Method::Delete => {
                                    router.delete(path, s)
                                }
                                leptos_router::Method::Patch => {
                                    router.patch(path, s)
                                }
                            }
                        }
                        SsrMode::Async => {
                            let s = render_app_async_with_context(
                                options.clone(),
                                additional_context.clone(),
                                app_fn.clone(),
                            );
                            match method {
                                leptos_router::Method::Get => {
                                    router.get(path, s)
                                }
                                leptos_router::Method::Post => {
                                    router.post(path, s)
                                }
                                leptos_router::Method::Put => {
                                    router.put(path, s)
                                }
                                leptos_router::Method::Delete => {
                                    router.delete(path, s)
                                }
                                leptos_router::Method::Patch => {
                                    router.patch(path, s)
                                }
                            }
                        }
                    }
                }
            })
        })
    }

    fn leptos_routes_with_handler<H, O>(
        self,
        paths: Vec<RouteListing>,
        handler: H,
    ) -> Self
    where
        H: Handler<Request, Output = Result<O>> + Clone,
        O: IntoResponse + Send + Sync + 'static,
    {
        paths.iter().fold(self, |router, listing| {
            listing
                .methods()
                .fold(router, |router, method| match method {
                    leptos_router::Method::Get => {
                        router.get(listing.path(), handler.clone())
                    }
                    leptos_router::Method::Post => {
                        router.post(listing.path(), handler.clone())
                    }
                    leptos_router::Method::Put => {
                        router.put(listing.path(), handler.clone())
                    }
                    leptos_router::Method::Delete => {
                        router.delete(listing.path(), handler.clone())
                    }
                    leptos_router::Method::Patch => {
                        router.patch(listing.path(), handler.clone())
                    }
                })
        })
    }
}
