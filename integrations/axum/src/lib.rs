use axum::{
    body::{Body, Bytes, Full, HttpBody, StreamBody},
    debug_handler,
    extract::Path,
    http::{HeaderMap, HeaderValue, Request, StatusCode},
    response::{IntoResponse, Response},
};
use futures::{Future, SinkExt, Stream, StreamExt};
use http::{method::Method, uri::Uri, version::Version};
use hyper::body;
use leptos::*;
use leptos_meta::MetaContext;
use leptos_router::*;
use std::{io, pin::Pin, sync::Arc};

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
/// If ResponseParts is inserted into context with `use_context()` during a server function, it will
/// let you set the status code and headers of the response. This is useful for cookies and custom responses.
/// Status is not set if the request does not have one of the supported body types, and Headers will be set
/// on any non Error response if provided
#[derive(Debug, Clone)]
pub struct ResponseParts {
    pub status: Option<StatusCode>,
    pub headers: HeaderMap,
}

pub async fn generate_request_parts(req: Request<Body>) -> RequestParts {
    // provide request headers as context in server scope
    let (parts, body) = req.into_parts();
    let body = body::to_bytes(body).await.unwrap_or_default();
    RequestParts {
        method: parts.method,
        uri: parts.uri,
        headers: parts.headers,
        version: parts.version,
        body: body.clone(),
    }
}

/// An Axum handlers to listens for a request with Leptos server function arguments in the body,
/// run the server function if found, and return the resulting [Response].
///
/// This provides an `Arc<[Request<Body>](axum::http::Request)>` [Scope](leptos::Scope).
///
/// This can then be set up at an appropriate route in your application:
///
/// ```
/// use axum::{handler::Handler, routing::post, Router};
/// use std::net::SocketAddr;
/// use leptos::*;
///
/// # if false { // don't actually try to run a server in a doctest...
/// #[tokio::main]
/// async fn main() {
///     let addr = SocketAddr::from(([127, 0, 0, 1], 8082));
///
///     // build our application with a route
///     let app = Router::new()
///       .route("/api/*fn_name", post(leptos_axum::handle_server_fns));
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
#[axum::debug_handler]
pub async fn handle_server_fns(
    Path(fn_name): Path<String>,
    headers: HeaderMap,
    req: Request<Body>,
) -> impl IntoResponse {
    // Axum Path extractor doesn't remove the first slash from the path, while Actix does
    let fn_name: String = match fn_name.strip_prefix("/") {
        Some(path) => path.to_string(),
        None => fn_name,
    };

    let (tx, rx) = futures::channel::oneshot::channel();
    std::thread::spawn({
        move || {
            tokio::runtime::Runtime::new()
                .expect("couldn't spawn runtime")
                .block_on({
                    async move {
                        let res = if let Some(server_fn) = server_fn_by_path(fn_name.as_str()) {
                            let runtime = create_runtime();
                            let (cx, disposer) = raw_scope_and_disposer(runtime);

                            let req_parts = generate_request_parts(req).await;

                            provide_context(cx, req_parts.clone());

                            match server_fn(cx, &req_parts.body).await {
                                Ok(serialized) => {
                                    // If ResponseParts are set, add the headers and extension to the request
                                    let response_parts = use_context::<ResponseParts>(cx);

                                    // clean up the scope, which we only needed to run the server fn
                                    disposer.dispose();
                                    runtime.dispose();

                                    // if this is Accept: application/json then send a serialized JSON response
                                    let accept_header =
                                        headers.get("Accept").and_then(|value| value.to_str().ok());
                                    let mut res = Response::builder();

                                    // Add headers from ResponseParts if they exist. These should be added as long
                                    // as the server function returns an OK response
                                    // Use provided headers if they exist
                                    let (status, mut res_headers) = match response_parts {
                                        Some(parts) => (parts.status, parts.headers),
                                        None => (None, HeaderMap::new()),
                                    };
                                    match res.headers_mut() {
                                        Some(header_ref) => {
                                            header_ref.extend(res_headers.drain());
                                        }
                                        None => (),
                                    };

                                    if accept_header == Some("application/json")
                                        || accept_header
                                            == Some("application/x-www-form-urlencoded")
                                        || accept_header == Some("application/cbor")
                                    {
                                        res = res.status(StatusCode::OK);

                                        // Override Status if Status is set in ResponseParts and
                                        // We're not trying to do a form submit
                                        res = match status {
                                            Some(status) => res.status(status),
                                            None => res,
                                        }
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
                                    .body(Full::from(e.to_string())),
                            }
                        } else {
                            Response::builder()
                                .status(StatusCode::BAD_REQUEST)
                                .body(Full::from(
                                    "Could not find a server function at that route.".to_string(),
                                ))
                        }
                        .expect("could not build Response");

                        _ = tx.send(res);
                    }
                })
        }
    });

    rx.await.unwrap()
}

pub type PinnedHtmlStream = Pin<Box<dyn Stream<Item = io::Result<Bytes>> + Send>>;

/// Returns an Axum [Handler](axum::handler::Handler) that listens for a `GET` request and tries
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
/// use axum::handler::Handler;
/// use axum::Router;
/// use std::{net::SocketAddr, env};
/// use leptos::*;
///
/// #[component]
/// fn MyApp(cx: Scope) -> Element {
///   view! { cx, <main>"Hello, world!"</main> }
/// }
///
/// # if false { // don't actually try to run a server in a doctest...
/// #[tokio::main]
/// async fn main() {
///     let addr = SocketAddr::from(([127, 0, 0, 1], 8082));
/// let render_options: RenderOptions = RenderOptions::builder()
///     .pkg_path("/pkg/leptos_example")
///     .socket_address(addr)
///     .reload_port(3001)
///     .environment(&env::var("RUST_ENV")).build();
///
///     // build our application with a route
///     let app = Router::new()
///     .fallback(leptos_axum::render_app_to_stream(render_options, |cx| view! { cx, <MyApp/> }));
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
pub fn render_app_to_stream(
    options: RenderOptions,
    app_fn: impl Fn(leptos::Scope) -> Element + Clone + Send + 'static,
) -> impl Fn(
    Request<Body>,
) -> Pin<Box<dyn Future<Output = StreamBody<PinnedHtmlStream>> + Send + 'static>>
       + Clone
       + Send
       + 'static {
    move |req: Request<Body>| {
        Box::pin({
            let options = options.clone();
            let app_fn = app_fn.clone();
            async move {
                // Need to get the path and query string of the Request
                let path = req.uri();
                let query = path.query();

                let full_path;
                if let Some(query) = query {
                    full_path = "http://leptos".to_string() + &path.to_string() + "?" + query
                } else {
                    full_path = "http://leptos".to_string() + &path.to_string()
                }

                let pkg_path = &options.pkg_path;
                let socket_ip = &options.socket_address.ip().to_string();
                let reload_port = options.reload_port;

                let leptos_autoreload = match options.environment {
                    RustEnv::DEV => format!(
                        r#"
                            <script crossorigin="">(function () {{
                                var ws = new WebSocket('ws://{socket_ip}:{reload_port}/autoreload');
                                ws.onmessage = (ev) => {{
                                    console.log(`Reload message: `);
                                    if (ev.data === 'reload') window.location.reload();
                                }};
                                ws.onclose = () => console.warn('Autoreload stopped. Manual reload necessary.');
                            }})()
                            </script>
                        "#
                    ),
                    RustEnv::PROD => "".to_string(),
                };

                let head = format!(
                    r#"<!DOCTYPE html>
                    <html lang="en">
                        <head>
                            <meta charset="utf-8"/>
                            <meta name="viewport" content="width=device-width, initial-scale=1"/>
                            <link rel="modulepreload" href="{pkg_path}.js">
                            <link rel="preload" href="{pkg_path}_bg.wasm" as="fetch" type="application/wasm" crossorigin="">
                            <script type="module">import init, {{ hydrate }} from '{pkg_path}.js'; init('{pkg_path}_bg.wasm').then(hydrate);</script>
                            {leptos_autoreload}
                            "#
                );
                let tail = "</body></html>";

                let (mut tx, rx) = futures::channel::mpsc::channel(8);

                std::thread::spawn({
                    let app_fn = app_fn.clone();
                    move || {
                        tokio::runtime::Runtime::new()
                            .expect("couldn't spawn runtime")
                            .block_on({
                                let app_fn = app_fn.clone();
                                async move {
                                    tokio::task::LocalSet::new()
                                        .run_until(async {
                                            let mut shell = Box::pin(render_to_stream({
                                                let full_path = full_path.clone();
                                                let req_parts = generate_request_parts(req).await;
                                                move |cx| {
                                                    let integration = ServerIntegration {
                                                        path: full_path.clone(),
                                                    };
                                                    provide_context(
                                                        cx,
                                                        RouterIntegrationContext::new(integration),
                                                    );
                                                    provide_context(cx, MetaContext::new());
                                                    provide_context(cx, req_parts);
                                                    let app = app_fn(cx);
                                                    let head = use_context::<MetaContext>(cx)
                                                        .map(|meta| meta.dehydrate())
                                                        .unwrap_or_default();
                                                    format!("{head}</head><body>{app}")
                                                }
                                            }));
                                            while let Some(fragment) = shell.next().await {
                                                _ = tx.send(fragment).await;
                                            }
                                            tx.close_channel();
                                        })
                                        .await;
                                }
                            });
                    }
                });

                let stream = futures::stream::once(async move { head.clone() })
                    .chain(rx)
                    .chain(futures::stream::once(async { tail.to_string() }))
                    .map(|html| Ok(Bytes::from(html)));
                StreamBody::new(Box::pin(stream) as PinnedHtmlStream)
            }
        })
    }
}
