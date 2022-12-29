use axum::{
    body::{Body, Bytes, Full, StreamBody},
    extract::Path,
    http::{HeaderMap, HeaderValue, Request, StatusCode},
    response::IntoResponse,
};
use futures::{Future, SinkExt, Stream, StreamExt};
use http::{method::Method, uri::Uri, version::Version, Response};
use hyper::body;
use leptos::*;
use leptos_meta::MetaContext;
use leptos_router::*;
use std::{io, pin::Pin, sync::Arc};
use tokio::{sync::RwLock, task::spawn_blocking};

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
/// This struct lets you define headers and override the status of the Response from an Element or a Server Function
/// Typically contained inside of a ResponseOptions. Setting this is useful for cookies and custom responses.
#[derive(Debug, Clone, Default)]
pub struct ResponseParts {
    pub status: Option<StatusCode>,
    pub headers: HeaderMap,
}

/// Adding this Struct to your Scope inside of a Server Fn or Element will allow you to override details of the Response
/// like status and add Headers/Cookies. Because Elements and Server Fns are lower in the tree than the Response generation
/// code, it needs to be wrapped in an `Arc<RwLock<>>` so that it can be surfaced.
#[derive(Debug, Clone, Default)]
pub struct ResponseOptions(pub Arc<RwLock<ResponseParts>>);

impl ResponseOptions {
    /// A less boilerplatey way to overwrite the default contents of `ResponseOptions` with a new `ResponseParts`
    pub async fn overwrite(&self, parts: ResponseParts) {
        let mut writable = self.0.write().await;
        *writable = parts
    }
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
    spawn_blocking({
        move || {
            tokio::runtime::Runtime::new()
                .expect("couldn't spawn runtime")
                .block_on({
                    async move {
                        let res = if let Some(server_fn) = server_fn_by_path(fn_name.as_str()) {
                            let runtime = create_runtime();
                            let (cx, disposer) = raw_scope_and_disposer(runtime);

                            let req_parts = generate_request_parts(req).await;
                            // Add this so we can get details about the Request
                            provide_context(cx, req_parts.clone());
                            // Add this so that we can set headers and status of the response
                            provide_context(cx, ResponseOptions::default());

                            match server_fn(cx, &req_parts.body).await {
                                Ok(serialized) => {
                                    // If ResponseParts are set, add the headers and extension to the request
                                    let res_options = use_context::<ResponseOptions>(cx);

                                    // clean up the scope, which we only needed to run the server fn
                                    disposer.dispose();
                                    runtime.dispose();

                                    // if this is Accept: application/json then send a serialized JSON response
                                    let accept_header =
                                        headers.get("Accept").and_then(|value| value.to_str().ok());
                                    let mut res = Response::builder();

                                    // Add headers from ResponseParts if they exist. These should be added as long
                                    // as the server function returns an OK response
                                    let res_options_outer = res_options.unwrap().0;
                                    let res_options_inner = res_options_outer.read().await;
                                    let (status, mut res_headers) = (
                                        res_options_inner.status.clone(),
                                        res_options_inner.headers.clone(),
                                    );

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
/// use leptos_config::get_configuration;
///
/// #[component]
/// fn MyApp(cx: Scope) -> impl IntoView {
///   view! { cx, <main>"Hello, world!"</main> }
/// }
///
/// # if false { // don't actually try to run a server in a doctest...
/// #[tokio::main]
/// async fn main() {
///     
///     let conf = get_configuration(Some("Cargo.toml")).await.unwrap();
///     let leptos_options = conf.leptos_options;
///     let addr = leptos_options.site_address.clone();
///     
///     // build our application with a route
///     let app = Router::new()
///     .fallback(leptos_axum::render_app_to_stream(leptos_options, |cx| view! { cx, <MyApp/> }));
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
pub fn render_app_to_stream<IV>(
    options: LeptosOptions,
    app_fn: impl Fn(leptos::Scope) -> IV + Clone + Send + 'static,
) -> impl Fn(
    Request<Body>,
) -> Pin<Box<dyn Future<Output = Response<StreamBody<PinnedHtmlStream>>> + Send + 'static>>
       + Clone
       + Send
       + 'static
where IV: IntoView
{
    move |req: Request<Body>| {
        Box::pin({
            let options = options.clone();
            let app_fn = app_fn.clone();
            let default_res_options = ResponseOptions::default();
            let res_options2 = default_res_options.clone();
            let res_options3 = default_res_options.clone();

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

                let site_root = &options.site_root;
                let output_name = &options.output_name;

                // Because wasm-pack adds _bg to the end of the WASM filename, and we want to mantain compatibility with it's default options
                // we add _bg to the wasm files if cargo-leptos doesn't set the env var OUTPUT_NAME
                // Otherwise we need to add _bg because wasm_pack always does. This is not the same as options.output_name, which is set regardless
                let mut wasm_output_name = output_name.clone();
                if std::env::var("OUTPUT_NAME").is_err() {
                    wasm_output_name.push_str("_bg");
                }

                let site_ip = &options.site_address.ip().to_string();
                let reload_port = options.reload_port;

                let leptos_autoreload = match std::env::var("LEPTOS_WATCH").is_ok() {
                    true => format!(
                        r#"
                        <script crossorigin="">(function () {{
                            var ws = new WebSocket('ws://{site_ip}:{reload_port}/live_reload');
                            ws.onmessage = (ev) => {{
                                let msg = JSON.parse(event.data);
                                if (msg.all) window.location.reload();
                                if (msg.css) {{
                                    const link = document.querySelector("link#leptos");
                                    if (link) {{
                                        let href = link.getAttribute('href').split('?')[0];
                                        let newHref = href + '?version=' + new Date().getMilliseconds();
                                        link.setAttribute('href', newHref);
                                    }} else {{
                                        console.warn("Could not find link#leptos");
                                    }}
                                }};
                            }};
                            ws.onclose = () => console.warn('Live-reload stopped. Manual reload necessary.');
                        }})()
                        </script>
                        "#
                    ),
                    false => "".to_string(),
                };

                let head = format!(
                    r#"<!DOCTYPE html>
                    <html lang="en">
                        <head>
                            <meta charset="utf-8"/>
                            <meta name="viewport" content="width=device-width, initial-scale=1"/>
                            <link rel="modulepreload" href="{site_root}/{output_name}.js">
                            <link rel="preload" href="{site_root}/{wasm_output_name}.wasm" as="fetch" type="application/wasm" crossorigin="">
                            <script type="module">import init, {{ hydrate }} from '{site_root}/{output_name}.js'; init('{site_root}/{wasm_output_name}.wasm').then(hydrate);</script>
                            {leptos_autoreload}
                            "#
                );
                let tail = "</body></html>";

                let (mut tx, rx) = futures::channel::mpsc::channel(8);

                spawn_blocking({
                    let app_fn = app_fn.clone();
                    move || {
                        tokio::runtime::Runtime::new()
                            .expect("couldn't spawn runtime")
                            .block_on({
                                let app_fn = app_fn.clone();
                                async move {
                                    tokio::task::LocalSet::new()
                                        .run_until(async {
                                            let app = {
                                                let full_path = full_path.clone();
                                                let req_parts =
                                                    generate_request_parts(req).await;
                                                move |cx| {
                                                    let integration = ServerIntegration {
                                                        path: full_path.clone(),
                                                    };
                                                    provide_context(
                                                        cx,
                                                        RouterIntegrationContext::new(
                                                            integration,
                                                        ),
                                                    );
                                                    provide_context(cx, MetaContext::new());
                                                    provide_context(cx, req_parts);
                                                    provide_context(cx, default_res_options);
                                                    app_fn(cx).into_view(cx)
                                                }
                                            };

                                            let (bundle, runtime, scope) =
                                                render_to_stream_with_prefix_undisposed(
                                                    app,
                                                |cx| {
                                                        let head = use_context::<MetaContext>(cx)
                                                            .map(|meta| meta.dehydrate())
                                                            .unwrap_or_default();
                                                        format!("{head}</head><body>").into()
                                                    }
                                            );
                                            let mut shell = Box::pin(bundle);
                                            while let Some(fragment) = shell.next().await {
                                                _ = tx.send(fragment).await;
                                            }

                                            // Extract the value of ResponseOptions from here
                                            let cx = Scope {
                                                runtime,
                                                id: scope
                                            };
                                            let res_options =
                                                use_context::<ResponseOptions>(cx).unwrap();

                                            let new_res_parts = res_options.0.read().await.clone();

                                            let mut writable = res_options2.0.write().await;
                                            *writable = new_res_parts;

                                            runtime.dispose();

                                            tx.close_channel();
                                        })
                                        .await;
                                }
                            });
                    }
                });

                let mut stream = Box::pin(
                    futures::stream::once(async move { head.clone() })
                        .chain(rx)
                        .chain(futures::stream::once(async { tail.to_string() }))
                        .map(|html| Ok(Bytes::from(html))),
                );

                // Get the first, second, and third chunks in the stream, which renders the app shell, and thus allows Resources to run
                let first_chunk = stream.next().await;
                let second_chunk = stream.next().await;
                let third_chunk = stream.next().await;

                // Extract the resources now that they've been rendered
                let res_options = res_options3.0.read().await;

                let complete_stream = futures::stream::iter([
                    first_chunk.unwrap(),
                    second_chunk.unwrap(),
                    third_chunk.unwrap(),
                ])
                .chain(stream);

                let mut res = Response::new(StreamBody::new(
                    Box::pin(complete_stream) as PinnedHtmlStream
                ));

                match res_options.status {
                    Some(status) => *res.status_mut() = status,
                    None => (),
                };
                let mut res_headers = res_options.headers.clone();
                res.headers_mut().extend(res_headers.drain());

                res
            }
        })
    }
}
