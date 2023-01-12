#![forbid(unsafe_code)]

use actix_web::{
    body::BoxBody,
    dev::{ServiceFactory, ServiceRequest},
    http::header,
    web::Bytes,
    *,
};
use futures::{Future, StreamExt};
use http::StatusCode;
use leptos::*;
use leptos_meta::*;
use leptos_router::*;
use regex::Regex;
use std::sync::Arc;
use tokio::sync::RwLock;

/// This struct lets you define headers and override the status of the Response from an Element or a Server Function
/// Typically contained inside of a ResponseOptions. Setting this is useful for cookies and custom responses.
#[derive(Debug, Clone, Default)]
pub struct ResponseParts {
    pub headers: header::HeaderMap,
    pub status: Option<StatusCode>,
}

impl ResponseParts {
    /// Insert a header, overwriting any previous value with the same key
    pub fn insert_header(&mut self, key: header::HeaderName, value: header::HeaderValue) {
        self.headers.insert(key, value);
    }
    /// Append a header, leaving any header with the same key intact
    pub fn append_header(&mut self, key: header::HeaderName, value: header::HeaderValue) {
        self.headers.append(key, value);
    }
}

/// Adding this Struct to your Scope inside of a Server Fn or Elements will allow you to override details of the Response
/// like StatusCode and add Headers/Cookies. Because Elements and Server Fns are lower in the tree than the Response generation
/// code, it needs to be wrapped in an `Arc<RwLock<>>` so that it can be surfaced
#[derive(Debug, Clone, Default)]
pub struct ResponseOptions(pub Arc<RwLock<ResponseParts>>);

impl ResponseOptions {
    /// A less boilerplatey way to overwrite the contents of `ResponseOptions` with a new `ResponseParts`
    pub async fn overwrite(&self, parts: ResponseParts) {
        let mut writable = self.0.write().await;
        *writable = parts
    }
    /// Set the status of the returned Response
    pub async fn set_status(&self, status: StatusCode) {
        let mut writeable = self.0.write().await;
        let res_parts = &mut *writeable;
        res_parts.status = Some(status);
    }
    /// Insert a header, overwriting any previous value with the same key
    pub async fn insert_header(&self, key: header::HeaderName, value: header::HeaderValue) {
        let mut writeable = self.0.write().await;
        let res_parts = &mut *writeable;
        res_parts.headers.insert(key, value);
    }
    /// Append a header, leaving any header with the same key intact
    pub async fn append_header(&self, key: header::HeaderName, value: header::HeaderValue) {
        let mut writeable = self.0.write().await;
        let res_parts = &mut *writeable;
        res_parts.headers.append(key, value);
    }
}

/// Provides an easy way to redirect the user from within a server function. Mimicing the Remix `redirect()`,
/// it sets a StatusCode of 302 and a LOCATION header with the provided value.
/// If looking to redirect from the client, `leptos_router::use_navigate()` should be used instead
pub async fn redirect(cx: leptos::Scope, path: &str) {
    let response_options = use_context::<ResponseOptions>(cx).unwrap();
    response_options.set_status(StatusCode::FOUND).await;
    response_options
        .insert_header(
            header::LOCATION,
            header::HeaderValue::from_str(path).expect("Failed to create HeaderValue"),
        )
        .await;
}

/// An Actix [Route](actix_web::Route) that listens for a `POST` request with
/// Leptos server function arguments in the body, runs the server function if found,
/// and returns the resulting [HttpResponse].
///
/// This provides the [HttpRequest] to the server [Scope](leptos::Scope).
///
/// This can then be set up at an appropriate route in your application:
///
/// ```
/// use actix_web::*;
///
/// fn register_server_functions() {
///   // call ServerFn::register() for each of the server functions you've defined
/// }
///
/// # if false { // don't actually try to run a server in a doctest...
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
/// # }
/// ```
pub fn handle_server_fns() -> Route {
    web::post().to(
        |req: HttpRequest, params: web::Path<String>, body: web::Bytes| async move {
            {
                let path = params.into_inner();
                let accept_header = req
                    .headers()
                    .get("Accept")
                    .and_then(|value| value.to_str().ok());

                if let Some(server_fn) = server_fn_by_path(path.as_str()) {
                    let body: &[u8] = &body;

                    let runtime = create_runtime();
                    let (cx, disposer) = raw_scope_and_disposer(runtime);
                    let res_options = ResponseOptions::default();

                    // provide HttpRequest as context in server scope
                    provide_context(cx, req.clone());
                    provide_context(cx, res_options.clone());

                    match server_fn(cx, body).await {
                        Ok(serialized) => {
                            let res_options = use_context::<ResponseOptions>(cx).unwrap();

                            // clean up the scope, which we only needed to run the server fn
                            disposer.dispose();
                            runtime.dispose();

                            let mut res: HttpResponseBuilder;
                            let mut res_parts = res_options.0.write().await;

                            if accept_header == Some("application/json")
                                || accept_header == Some("application/x-www-form-urlencoded")
                                || accept_header == Some("application/cbor")
                            {
                                res = HttpResponse::Ok();
                            }
                            // otherwise, it's probably a <form> submit or something: redirect back to the referrer
                            else {
                                let referer = req
                                    .headers()
                                    .get("Referer")
                                    .and_then(|value| value.to_str().ok())
                                    .unwrap_or("/");
                                res = HttpResponse::SeeOther();
                                res.insert_header(("Location", referer))
                                    .content_type("application/json");
                            };
                            // Override StatusCode if it was set in a Resource or Element
                            if let Some(status) = res_parts.status {
                                res.status(status);
                            }

                            // Use provided ResponseParts headers if they exist
                            let _count = res_parts
                                .headers
                                .drain()
                                .map(|(k, v)| {
                                    if let Some(k) = k {
                                        res.append_header((k, v));
                                    }
                                })
                                .count();

                            match serialized {
                                Payload::Binary(data) => {
                                    res.content_type("application/cbor");
                                    res.body(Bytes::from(data))
                                }
                                Payload::Url(data) => {
                                    res.content_type("application/x-www-form-urlencoded");
                                    res.body(data)
                                }
                                Payload::Json(data) => {
                                    res.content_type("application/json");
                                    res.body(data)
                                }
                            }
                        }
                        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
                    }
                } else {
                    HttpResponse::BadRequest().body(format!(
                        "Could not find a server function at the route {:?}. \
                        \n\nIt's likely that you need to call ServerFn::register() on the \
                        server function type, somewhere in your `main` function.",
                        req.path()
                    ))
                }
            }
        },
    )
}

/// Returns an Actix [Route](actix_web::Route) that listens for a `GET` request and tries
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
/// use actix_web::{HttpServer, App};
/// use leptos::*;
/// use std::{env,net::SocketAddr};
///
/// #[component]
/// fn MyApp(cx: Scope) -> impl IntoView {
///   view! { cx, <main>"Hello, world!"</main> }
/// }
///
/// # if false { // don't actually try to run a server in a doctest...
/// #[actix_web::main]
/// async fn main() -> std::io::Result<()> {
///     let conf = get_configuration(Some("Cargo.toml")).await.unwrap();
///     let addr = conf.leptos_options.site_address.clone();
///     HttpServer::new(move || {
///         let leptos_options = &conf.leptos_options;
///     
///         App::new()
///             // {tail:.*} passes the remainder of the URL as the route
///             // the actual routing will be handled by `leptos_router`
///             .route("/{tail:.*}", leptos_actix::render_app_to_stream(leptos_options.to_owned(), |cx| view! { cx, <MyApp/> }))
///     })
///     .bind(&addr)?
///     .run()
///     .await
/// }
/// # }
/// ```
pub fn render_app_to_stream<IV>(
    options: LeptosOptions,
    app_fn: impl Fn(leptos::Scope) -> IV + Clone + 'static,
) -> Route
where
    IV: IntoView,
{
    web::get().to(move |req: HttpRequest| {
        let options = options.clone();
        let app_fn = app_fn.clone();
        let res_options = ResponseOptions::default();

        async move {
            let app = {
                let app_fn = app_fn.clone();
                let res_options = res_options.clone();
                move |cx| {
                    provide_contexts(cx, &req, res_options);
                    (app_fn)(cx).into_view(cx)
                }
            };

            let (head, tail) = html_parts(&options);

            stream_app(app, head, tail, res_options).await
        }
    })
}

/// Returns an Actix [Route](actix_web::Route) that listens for a `GET` request and tries
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
/// use actix_web::{HttpServer, App};
/// use leptos::*;
/// use std::{env,net::SocketAddr};
/// use leptos_actix::DataResponse;
///
/// #[component]
/// fn MyApp(cx: Scope, data: &'static str) -> impl IntoView {
///   view! { cx, <main>"Hello, world!"</main> }
/// }
///
/// # if false { // don't actually try to run a server in a doctest...
/// #[actix_web::main]
/// async fn main() -> std::io::Result<()> {
///     let conf = get_configuration(Some("Cargo.toml")).await.unwrap();
///     let addr = conf.leptos_options.site_address.clone();
///     HttpServer::new(move || {
///         let leptos_options = &conf.leptos_options;
///     
///         App::new()
///             // {tail:.*} passes the remainder of the URL as the route
///             // the actual routing will be handled by `leptos_router`
///             .route("/{tail:.*}", leptos_actix::render_preloaded_data_app(
///                 leptos_options.to_owned(),
///                 |req| async move { Ok(DataResponse::Data("async func that can preload data")) },
///                 |cx, data| view! { cx, <MyApp data/> })
///             )
///     })
///     .bind(&addr)?
///     .run()
///     .await
/// }
/// # }
/// ```
pub fn render_preloaded_data_app<Data, Fut, IV>(
    options: LeptosOptions,
    data_fn: impl Fn(HttpRequest) -> Fut + Clone + 'static,
    app_fn: impl Fn(leptos::Scope, Data) -> IV + Clone + Send + 'static,
) -> Route
where
    Data: 'static,
    Fut: Future<Output = Result<DataResponse<Data>, actix_web::Error>>,
    IV: IntoView + 'static,
{
    web::get().to(move |req: HttpRequest| {
        let options = options.clone();
        let app_fn = app_fn.clone();
        let data_fn = data_fn.clone();
        let res_options = ResponseOptions::default();

        async move {
            let data = match data_fn(req.clone()).await {
                Err(e) => return HttpResponse::from_error(e),
                Ok(DataResponse::Response(r)) => return r.into(),
                Ok(DataResponse::Data(d)) => d,
            };

            let app = {
                let app_fn = app_fn.clone();
                let res_options = res_options.clone();
                move |cx| {
                    provide_contexts(cx, &req, res_options);
                    (app_fn)(cx, data).into_view(cx)
                }
            };

            let (head, tail) = html_parts(&options);

            stream_app(app, head, tail, res_options).await
        }
    })
}

fn provide_contexts(cx: leptos::Scope, req: &HttpRequest, res_options: ResponseOptions) {
    let path = leptos_corrected_path(req);

    let integration = ServerIntegration { path };
    provide_context(cx, RouterIntegrationContext::new(integration));
    provide_context(cx, MetaContext::new());
    provide_context(cx, res_options);
    provide_context(cx, req.clone());
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

async fn stream_app(
    app: impl FnOnce(leptos::Scope) -> View + 'static,
    head: String,
    tail: String,
    res_options: ResponseOptions,
) -> HttpResponse<BoxBody> {
    let (stream, runtime, _) = render_to_stream_with_prefix_undisposed(app, move |cx| {
        let head = use_context::<MetaContext>(cx)
            .map(|meta| meta.dehydrate())
            .unwrap_or_default();
        format!("{head}</head><body>").into()
    });

    let mut stream = Box::pin(
        futures::stream::once(async move { head.clone() })
            .chain(stream)
            .chain(futures::stream::once(async move {
                runtime.dispose();
                tail.to_string()
            }))
            .map(|html| Ok(web::Bytes::from(html)) as Result<web::Bytes>),
    );

    // Get the first, second, and third chunks in the stream, which renders the app shell, and thus allows Resources to run
    let first_chunk = stream.next().await;
    let second_chunk = stream.next().await;
    let third_chunk = stream.next().await;

    let res_options = res_options.0.read().await;

    let (status, mut headers) = (res_options.status, res_options.headers.clone());
    let status = status.unwrap_or_default();

    let complete_stream = futures::stream::iter([
        first_chunk.unwrap(),
        second_chunk.unwrap(),
        third_chunk.unwrap(),
    ])
    .chain(stream);
    let mut res = HttpResponse::Ok()
        .content_type("text/html")
        .streaming(complete_stream);
    // Add headers manipulated in the response
    for (key, value) in headers.drain() {
        if let Some(key) = key {
            res.headers_mut().append(key, value);
        }
    }
    // Set status to what is returned in the function
    let res_status = res.status_mut();
    *res_status = status;
    // Return the response
    res
}

fn html_parts(options: &LeptosOptions) -> (String, String) {
    // Because wasm-pack adds _bg to the end of the WASM filename, and we want to mantain compatibility with it's default options
    // we add _bg to the wasm files if cargo-leptos doesn't set the env var LEPTOS_OUTPUT_NAME
    // Otherwise we need to add _bg because wasm_pack always does. This is not the same as options.output_name, which is set regardless
    let output_name = &options.output_name;
    let mut wasm_output_name = output_name.clone();
    if std::env::var("LEPTOS_OUTPUT_NAME").is_err() {
        wasm_output_name.push_str("_bg");
    }

    let site_ip = &options.site_address.ip().to_string();
    let reload_port = options.reload_port;
    let pkg_path = &options.site_pkg_dir;

    let leptos_autoreload = match std::env::var("LEPTOS_WATCH").is_ok() {
        true => format!(
            r#"
            <script crossorigin="">(function () {{
                var ws = new WebSocket('ws://{site_ip}:{reload_port}/live_reload');
                ws.onmessage = (ev) => {{
                    let msg = JSON.parse(ev.data);
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
                <link rel="modulepreload" href="/{pkg_path}/{output_name}.js">
                <link rel="preload" href="/{pkg_path}/{wasm_output_name}.wasm" as="fetch" type="application/wasm" crossorigin="">
                <script type="module">import init, {{ hydrate }} from '/{pkg_path}/{output_name}.js'; init('/{pkg_path}/{wasm_output_name}.wasm').then(hydrate);</script>
                {leptos_autoreload}
                "#
    );
    let tail = "</body></html>".to_string();

    (head, tail)
}

/// Generates a list of all routes defined in Leptos's Router in your app. We can then use this to automatically
/// create routes in Actix's App without having to use wildcard matching or fallbacks. Takes in your root app Element
/// as an argument so it can walk you app tree. This version is tailored to generated Actix compatible paths.
pub fn generate_route_list<IV>(app_fn: impl FnOnce(leptos::Scope) -> IV + 'static) -> Vec<String>
where
    IV: IntoView + 'static,
{
    let mut routes = leptos_router::generate_route_list_inner(app_fn);

    // Empty strings screw with Actix pathing, they need to be "/"
    routes = routes
        .iter()
        .map(|s| {
            if s.is_empty() {
                return "/".to_string();
            }
            s.to_string()
        })
        .collect();

    // Actix's Router doesn't follow Leptos's
    // Match `*` or `*someword` to replace with replace it with "/{tail.*}
    let wildcard_re = Regex::new(r"\*.*").unwrap();
    // Match `:some_word` but only capture `some_word` in the groups to replace with `{some_word}`
    let capture_re = Regex::new(r":((?:[^.,/]+)+)[^/]?").unwrap();

    let routes: Vec<String> = routes
        .iter()
        .map(|s| wildcard_re.replace_all(s, "{tail:.*}").to_string())
        .map(|s| capture_re.replace_all(&s, "{$1}").to_string())
        .collect();

    if routes.is_empty() {
        vec!["/".to_string()]
    } else {
        routes
    }
}

pub enum DataResponse<T> {
    Data(T),
    Response(actix_web::dev::Response<BoxBody>),
}

/// This trait allows one to pass a list of routes and a render function to Axum's router, letting us avoid
/// having to use wildcards or manually define all routes in multiple places.
pub trait LeptosRoutes {
    fn leptos_routes<IV>(
        self,
        options: LeptosOptions,
        paths: Vec<String>,
        app_fn: impl Fn(leptos::Scope) -> IV + Clone + Send + 'static,
    ) -> Self
    where
        IV: IntoView + 'static;

    fn leptos_preloaded_data_routes<Data, Fut, IV>(
        self,
        options: LeptosOptions,
        paths: Vec<String>,
        data_fn: impl Fn(HttpRequest) -> Fut + Clone + 'static,
        app_fn: impl Fn(leptos::Scope, Data) -> IV + Clone + Send + 'static,
    ) -> Self
    where
        Data: 'static,
        Fut: Future<Output = Result<DataResponse<Data>, actix_web::Error>>,
        IV: IntoView + 'static;
}

/// The default implementation of `LeptosRoutes` which takes in a list of paths, and dispatches GET requests
/// to those paths to Leptos's renderer.
impl<T> LeptosRoutes for actix_web::App<T>
where
    T: ServiceFactory<ServiceRequest, Config = (), Error = Error, InitError = ()>,
{
    fn leptos_routes<IV>(
        self,
        options: LeptosOptions,
        paths: Vec<String>,
        app_fn: impl Fn(leptos::Scope) -> IV + Clone + Send + 'static,
    ) -> Self
    where
        IV: IntoView + 'static,
    {
        let mut router = self;
        for path in paths.iter() {
            router = router.route(path, render_app_to_stream(options.clone(), app_fn.clone()));
        }
        router
    }

    fn leptos_preloaded_data_routes<Data, Fut, IV>(
        self,
        options: LeptosOptions,
        paths: Vec<String>,
        data_fn: impl Fn(HttpRequest) -> Fut + Clone + 'static,
        app_fn: impl Fn(leptos::Scope, Data) -> IV + Clone + Send + 'static,
    ) -> Self
    where
        Data: 'static,
        Fut: Future<Output = Result<DataResponse<Data>, actix_web::Error>>,
        IV: IntoView + 'static,
    {
        let mut router = self;

        for path in paths.iter() {
            router = router.route(
                path,
                render_preloaded_data_app(options.clone(), data_fn.clone(), app_fn.clone()),
            );
        }
        router
    }
}
