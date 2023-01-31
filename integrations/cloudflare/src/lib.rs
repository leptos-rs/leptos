use std::{sync::Arc, future::Future};

use futures::StreamExt;
use serde_json::json;
use worker::*;

mod utils;

fn log_request(req: &Request) {
    console_log!(
        "{} - [{}], located at: {:?}, within: {}",
        Date::now().to_string(),
        req.path(),
        req.cf().coordinates().unwrap_or_default(),
        req.cf().region().unwrap_or_else(|| "unknown region".into())
    );
}

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    log_request(&req);
    utils::set_panic_hook();
    
    let router = Router::new();
    router
        .leptos_routes(
            generate_route_list(|cx| view! { cx, <App/> }),
            |req, _| render_app_to_string("/pkg", "leptos_worker", req, |cx| view! { cx, <App/> })
        )
        .run(req, env)
        .await
}

// integration
use parking_lot::RwLock;

/// This struct lets you define headers and override the status of the Response from an Element or a Server Function
/// Typically contained inside of a ResponseOptions. Setting this is useful for cookies and custom responses.
#[derive(Debug, Clone, Default)]
pub struct ResponseParts {
    pub headers: worker::Headers,
    pub status: Option<u16>,
}

impl ResponseParts {
    /// Insert a header, overwriting any previous value with the same key
    pub fn set_header(&mut self, key: &str, value: &str) -> Result<()> {
        self.headers.set(key, value)
    }
    /// Append a header, leaving any header with the same key intact
    pub fn append_header(&mut self, key: &str, value: &str) -> Result<()> {
        self.headers.append(key, value)
    }
}

/// Adding this Struct to your Scope inside of a Server Fn or Elements will allow you to override details of the Response
/// like StatusCode and add Headers/Cookies. Because Elements and Server Fns are lower in the tree than the Response generation
/// code, it needs to be wrapped in an `Arc<RwLock<>>` so that it can be surfaced
#[derive(Debug, Clone, Default)]
pub struct ResponseOptions(pub Arc<RwLock<ResponseParts>>);

impl ResponseOptions {
    /// A less boilerplatey way to overwrite the contents of `ResponseOptions` with a new `ResponseParts`
    pub fn overwrite(&self, parts: ResponseParts) {
        let mut writable = self.0.write();
        *writable = parts
    }
    /// Set the status of the returned Response
    pub fn set_status(&self, status: u16) {
        let mut writeable = self.0.write();
        let res_parts = &mut *writeable;
        res_parts.status = Some(status);
    }
    /// Set a header, overwriting any previous value with the same key
    pub fn set_header(&self, key: &str, value: &str) {
        let mut writeable = self.0.write();
        let res_parts = &mut *writeable;
        res_parts.headers.set(key, value);
    }
    /// Append a header, leaving any header with the same key intact
    pub fn append_header(&self, key: &str, value: &str) {
        let mut writeable = self.0.write();
        let res_parts = &mut *writeable;
        res_parts.headers.append(key, value);
    }
}

pub fn generate_route_list<IV>(app_fn: impl FnOnce(leptos::Scope) -> IV + 'static) -> Vec<String>
where
    IV: IntoView + 'static,
{
    let mut routes = leptos_router::generate_route_list_inner(app_fn);

    // replace empty paths with "/"
    // otherwise, CF router works very similar to Leptos 
    // with :params and *blobs
    routes = routes
        .iter()
        .map(|s| {
            if s.is_empty() {
                return "/".to_string();
            }
            s.to_string()
        })
        .collect();

    if routes.is_empty() {
        vec!["/".to_string()]
    } else {
        routes
    }
}

async fn stream_app(
    pkg_url: &str,
    crate_name: &str,
    app: impl FnOnce(leptos::Scope) -> leptos::View + 'static,
    res_options: ResponseOptions,
    additional_context: impl Fn(leptos::Scope) + 'static + Clone + Send,
) -> Response {
    let (stream, runtime, scope) = render_to_stream_with_prefix_undisposed_with_context(
        app,
        move |cx| {
            let meta = use_context::<MetaContext>(cx);
            let head = meta
                .as_ref()
                .map(|meta| meta.dehydrate())
                .unwrap_or_default();
            let body_meta = meta
                .as_ref()
                .and_then(|meta| meta.body.as_string())
                .unwrap_or_default();
            format!("{head}</head><body{body_meta}>").into()
        },
        additional_context,
    );

    let cx = leptos::Scope { runtime, id: scope };
    let meta = use_context::<MetaContext>(cx);

        let html_meta = meta.as_ref().and_then(|mc| mc.html.as_string()).unwrap_or_default();
    let head = format!(r#"
                <!DOCTYPE html>
                <html{html_meta}>
                    <head>
                        <link rel="modulepreload" href="/{pkg_url}/{crate_name}.js">
                        <link rel="preload" href="/{pkg_url}/{crate_name}_bg.wasm" as="fetch" type="application/wasm" crossorigin="">
                        <script type="module">import init, {{ hydrate }} from '/{pkg_url}/{crate_name}.js'; init('/{pkg_url}/{crate_name}_bg.wasm').then(hydrate);</script>"#);
    let tail = "</body></html>";

    let mut stream = Box::pin(
        futures::stream::once(async move { head.clone() })
            .chain(stream)
            .chain(futures::stream::once(async move {
                runtime.dispose();
                tail.to_string()
            }))
    );

    // Get the first, second, and third chunks in the stream, which renders the app shell, and thus allows Resources to run
    let first_chunk = stream.next().await;
    let second_chunk = stream.next().await;
    let third_chunk = stream.next().await;

    let res_options = res_options.0.read();

    let (status, mut headers) = (res_options.status, res_options.headers.clone());
    let status = status.unwrap_or(200);

    let complete_stream = futures::stream::iter([
        first_chunk.unwrap(),
        second_chunk.unwrap(),
        third_chunk.unwrap(),
    ])
    .chain(stream)
    .map(|html| Ok(html));
    Response::from_stream(stream)
        .expect("to create Response from Stream")
        .with_status(status)
        .with_headers(headers)
}

pub fn render_app_to_stream_with_additional_context<IV>(
    pkg_url: &str,
    crate_name: &str,
    req: Request,
    app_fn: impl FnOnce(Scope) -> IV + Clone + 'static,
    additional_context: impl FnOnce(Scope) + Clone + Send + 'static
)-> Result<Response>
where IV: IntoView
{
    let pkg_url = pkg_url.to_owned();
    let crate_name = crate_name.to_owned();
    let runtime = create_runtime();
    let (html, _, _) = run_scope_undisposed(runtime, move |cx| {
        let integration = RouterIntegrationContext::new(ServerIntegration { path: format!("https://leptos.dev{}", req.path()) });
        provide_context(cx, integration);
        provide_context(cx, MetaContext::new());
        provide_context(cx, Arc::new(req.clone()));
        let html = app_fn(cx).into_view(cx).render_to_string(cx);

        let meta = use_context::<MetaContext>(cx);
        let html_meta = meta.as_ref().and_then(|mc| mc.html.as_string()).unwrap_or_default();
        let body_meta = meta.as_ref().and_then(|mc| mc.body.as_string()).unwrap_or_default();
        let meta_tags = meta.map(|mc| mc.dehydrate()).unwrap_or_default();
        format!(r#"
                <!DOCTYPE html>
                <html{html_meta}>
                    <head>
                        <link rel="modulepreload" href="/{pkg_url}/{crate_name}.js">
                        <link rel="preload" href="/{pkg_url}/{crate_name}_bg.wasm" as="fetch" type="application/wasm" crossorigin="">
                        <script type="module">import init, {{ hydrate }} from '/{pkg_url}/{crate_name}.js'; init('/{pkg_url}/{crate_name}_bg.wasm').then(hydrate);</script>
                        {meta_tags}
                    </head>
                    <body{body_meta}>
                        {html}
                    </body>
                </html>"#)
    });
    runtime.dispose();
    Response::from_html(html)
}

pub fn render_app_to_string<IV>(
    pkg_url: &str,
    crate_name: &str,
    req: Request,
    app_fn: impl FnOnce(Scope) -> IV + Clone + 'static
)-> Result<Response>
where IV: IntoView
{
    let pkg_url = pkg_url.to_owned();
    let crate_name = crate_name.to_owned();
    let runtime = create_runtime();
    let (html, _, _) = run_scope_undisposed(runtime, move |cx| {
        let integration = RouterIntegrationContext::new(ServerIntegration { path: format!("https://leptos.dev{}", req.path()) });
        provide_context(cx, integration);
        provide_context(cx, MetaContext::new());
        provide_context(cx, Arc::new(req.clone()));
        let html = app_fn(cx).into_view(cx).render_to_string(cx);

        let meta = use_context::<MetaContext>(cx);
        let html_meta = meta.as_ref().and_then(|mc| mc.html.as_string()).unwrap_or_default();
        let body_meta = meta.as_ref().and_then(|mc| mc.body.as_string()).unwrap_or_default();
        let meta_tags = meta.map(|mc| mc.dehydrate()).unwrap_or_default();
        format!(r#"
                <!DOCTYPE html>
                <html{html_meta}>
                    <head>
                        <link rel="modulepreload" href="/{pkg_url}/{crate_name}.js">
                        <link rel="preload" href="/{pkg_url}/{crate_name}_bg.wasm" as="fetch" type="application/wasm" crossorigin="">
                        <script type="module">import init, {{ hydrate }} from '/{pkg_url}/{crate_name}.js'; init('/{pkg_url}/{crate_name}_bg.wasm').then(hydrate);</script>
                        {meta_tags}
                    </head>
                    <body{body_meta}>
                        {html}
                    </body>
                </html>"#)
    });
    runtime.dispose();
    Response::from_html(html)
}

pub async fn render_preloaded_data_app_to_string<Data, Fut, IV>(
    req: Request,
    data_fn: impl Fn(&Request) -> Fut + Clone + 'static,
    app_fn: impl FnOnce(Scope, Data) -> IV + Clone + 'static
) -> Result<Response>
where
    Data: 'static,
    Fut: Future<Output = Result<DataResponse<Data>>>,
    IV: IntoView + 'static,
{
    let data = data_fn(&req).await;
    let data = match data {
        Err(e) => return Response::error(e.to_string(), 500),
        Ok(DataResponse::Response(r)) => return Ok(r),
        Ok(DataResponse::Data(d)) => d
    };

    let runtime = create_runtime();
    let (html, _, _) = run_scope_undisposed(runtime, move |cx| {
        let integration = RouterIntegrationContext::new(ServerIntegration { path: format!("https://leptos.dev{}", req.path()) });
        provide_context(cx, integration);
        provide_context(cx, MetaContext::new());
        provide_context(cx, Arc::new(req.clone()));
        let html = app_fn(cx, data).into_view(cx).render_to_string(cx);

        let meta = use_context::<MetaContext>(cx);
        let html_meta = meta.as_ref().and_then(|mc| mc.html.as_string()).unwrap_or_default();
        let body_meta = meta.as_ref().and_then(|mc| mc.body.as_string()).unwrap_or_default();
        let meta_tags = meta.map(|mc| mc.dehydrate()).unwrap_or_default();
        format!(r#"
                <!DOCTYPE html>
                <html{html_meta}>
                    <head>
                        {meta_tags}
                    </head>
                    <body{body_meta}>
                        {html}
                    </body>
                </html>"#)
    });
    runtime.dispose();
    Response::from_html(html)
}

pub enum DataResponse<T> {
    Data(T),
    Response(worker::Response),
}

/// This trait allows one to pass a list of routes and a render function to Cloudflare's router, letting us avoid
/// having to use wildcards or manually define all routes in multiple places.
pub trait LeptosRoutes<T> {
    fn leptos_routes(
        self,
        paths: Vec<String>,
        app_fn: fn(Request, worker::RouteContext<T>) -> Result<Response>
    ) -> Self;

    fn leptos_preloaded_data_routes<Data, Fut, IV>(
        self,
        paths: Vec<String>,
        data_fn: impl Fn(Request) -> Fut + Clone + 'static,
        app_fn: impl Fn(leptos::Scope, Data) -> IV + Clone + Send + 'static,
    ) -> Self
    where
        Data: 'static,
        Fut: Future<Output = Result<DataResponse<Data>>>,
        IV: IntoView + 'static;
}

/// The default implementation of `LeptosRoutes` which takes in a list of paths, and dispatches GET requests
/// to those paths to Leptos's renderer.
impl<T> LeptosRoutes<T> for worker::Router<'_, T>
where T: 'static
{
    fn leptos_routes(
        self,
        paths: Vec<String>,
        app_fn: fn(Request, worker::RouteContext<T>) -> Result<Response>
    ) -> Self 
    {
        let mut router = self;
        for path in paths.iter() {
            router = router.get(path, app_fn);
        }
        router
    }

    fn leptos_preloaded_data_routes<Data, Fut, IV>(
        self,
        paths: Vec<String>,
        data_fn: impl Fn(Request) -> Fut + Clone + 'static,
        app_fn: impl Fn(leptos::Scope, Data) -> IV + Clone + Send + 'static,
    ) -> Self
    where
        Data: 'static,
        Fut: Future<Output = Result<DataResponse<Data>>>,
        IV: IntoView + 'static,
    {
        let mut router = self;

        for path in paths.iter() {
            router = router.get(
                path,
                |req, _| todo!() //render_preloaded_data_app_to_string(req, data_fn.clone(), app_fn.clone()),
            );
        }
        router
    }
}

// This app

use leptos::{component, Scope, IntoView, create_signal, view, render_to_string, provide_context, LeptosOptions, use_context, create_runtime, run_scope, run_scope_undisposed, get_configuration, render_to_stream_with_prefix_undisposed_with_context};
use leptos_router::*;
use leptos_meta::*;

#[component]
pub fn App(cx: Scope) -> impl IntoView {

    view! { cx,
        <Router>
            <Meta name="color-scheme" content="dark"/>
            <Title text="Hello from Leptos Cloudflare"/>
            <nav>
                <a href="/">"Home"</a>
                <a href="/about">"About"</a>
            </nav>
            <main>
                <Routes>
                    <Route path="/" view=|cx| view! { cx, <HomePage/> }/>
                    <Route path="/about" view=|cx| view! { cx, <About/> }/>
                </Routes>
            </main>
        </Router>
    }
}

#[component]
pub fn HomePage(cx: Scope) -> impl IntoView {
    let (count, set_count) = create_signal(cx, 0);
    view! { cx,
        <h1>"Hello, Leptos Cloudflare!"</h1>
        <button on:click=move |_| set_count.update(|n| *n += 1)>
            "Click me: " {count}
        </button>
    }
}

#[component]
pub fn About(cx: Scope) -> impl IntoView {
    view! { cx,
        <h1>"About"</h1>
    }
}
