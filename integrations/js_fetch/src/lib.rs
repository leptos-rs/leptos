use futures::{channel::mpsc::Receiver, Future, Stream, StreamExt};
use gloo_net::http::{Request, Response, ResponseBuilder};
use js_sys::Uint8Array;
use leptos::{
    provide_context, ssr::render_to_stream_with_prefix_undisposed_with_context,
    use_context, IntoView, LeptosOptions, RuntimeId, ScopeId, View,
};
use leptos_integration_utils::{build_async_response, html_parts_separated};
use leptos_meta::{generate_head_metadata_separated, MetaContext};
use leptos_router::{
    provide_server_redirect, RouteListing, RouterIntegrationContext,
    ServerIntegration, SsrMode,
};
use std::{pin::Pin, sync::{Arc, RwLock}};
use url::Url;
use matchit::{Router, InsertError};
use wasm_bindgen::prelude::*;

type HandlerResponse = Pin<Box<dyn Future<Output = web_sys::Response>>>;

pub trait Handler: HandlerClone {
    fn call(self: Box<Self>, req: web_sys::Request) -> HandlerResponse;
}


pub trait HandlerClone {
    fn clone_box(&self) -> Box<dyn Handler>;
}

impl <T> HandlerClone for T
    where 
    T: 'static + Handler + Clone
{
    fn clone_box(&self) -> Box<dyn Handler> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn Handler> {
    fn clone(&self) -> Box<dyn Handler> {
        self.clone_box()
    }
}


impl<F, Fut> Handler for F
where
    F: Clone + FnOnce(Request) -> Fut + 'static,
    Fut: Future<Output = Response>,
{
    fn call(self: Box<Self>, req: web_sys::Request) -> HandlerResponse {
        Box::pin(async move {
            let req = Request::from(req);
            let res = self(req).await;
            res.into()
        })
    }
}

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

}

#[derive(Hash, PartialEq, Eq, Clone)]
struct RouteIdentifier {
    method: gloo_net::http::Method,
    pathname: String,
}



#[derive(Debug, Clone, Default)]
pub struct ResponseOptions(pub Arc<RwLock<gloo_net::http::ResponseBuilder>>);

#[derive(Clone)]
pub struct DefaultRouter {
    get_router: Router<Box<dyn Handler>>,
    post_router: Router<Box<dyn Handler>>,
    not_found: Box<dyn Handler>
}

impl DefaultRouter {
    pub fn new(not_found: impl Handler + 'static) -> Self {
        let get_router: Router<Box<dyn Handler>> = Router::new();
        let post_router: Router<Box<dyn Handler>> = Router::new();

        Self { get_router, post_router, not_found: Box::new(not_found) }
    }

    fn add_route(
        &mut self,
        path: RouteIdentifier,
        route: Box<dyn Handler>,
    ) -> Result<(), InsertError> {
        match path.method {
            gloo_net::http::Method::GET => self.get_router.insert(path.pathname, route),
            gloo_net::http::Method::POST => self.post_router.insert(path.pathname, route),
            _ => Err(InsertError::Conflict { with: "This integration only routes POST and GET requests".to_owned() })
        }
        
    }

    pub async fn serve(&self, req: web_sys::Request) -> Result<web_sys::Response, web_sys::Response> {
        let url_str = req.url();
        let url = web_sys::Url::new(&url_str).unwrap();
        let pathname = url.pathname();
        let req = Request::from(req);

        let out = match req.method() {
            gloo_net::http::Method::GET => self.get_router.at(&pathname),
            gloo_net::http::Method::POST => self.post_router.at(&pathname),
            _ => Err(matchit::MatchError::NotFound)
        };

        match out {
            Ok(m) => Ok(m.value.clone().call(req.into()).await),
            Err(_) => Err(self.not_found.clone().call(req.into()).await)
        }
    }

}

impl Handler for DefaultRouter {
    fn call(self: Box<Self>, req: web_sys::Request) -> HandlerResponse {
        Box::pin(async move {

            let req = Request::from(req);

            let router = match req.method() {
                gloo_net::http::Method::GET => self.get_router,
                gloo_net::http::Method::POST => self.post_router,
                _ => panic!("invalid method received")
            };

            let url = req.url();
            let pathname = web_sys::Url::new(&url).unwrap().pathname();
            let route = router.at(&pathname).ok();
            match route {
                Some(route) => route.value.clone().call(req.into()).await,
                None => self
                    .not_found
                    .call(req.into()).await,
            }
        })
    }
}

struct GlooMethod(gloo_net::http::Method);

impl From<&leptos_router::Method> for GlooMethod {
    fn from(value: &leptos_router::Method) -> Self {
        let val = match value {
            leptos_router::Method::Get => gloo_net::http::Method::GET,
            leptos_router::Method::Post => gloo_net::http::Method::POST,
            leptos_router::Method::Put => gloo_net::http::Method::PUT,
            leptos_router::Method::Delete => gloo_net::http::Method::DELETE,
            leptos_router::Method::Patch => gloo_net::http::Method::PATCH,
        };
        GlooMethod(val)
    }
}

// fn server_error(msg: impl Error) -> web_sys::Response {
//     let formatted = format!("{{ \"error\": \"{}\" }}", msg);
//     let body = Some(formatted.as_str());
//     Response::builder()
//         .status(500)
//         .header("Content-Type", "application/json")
//         .body(body)
//         .unwrap()
//         .into()
// }

// #[derive(Debug)]
// enum ServerError {
//     CouldNotParseUrl,
// }

// impl Display for ServerError {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             Self::CouldNotParseUrl => {
//                 f.write_str("Failed to parse the url for the server")
//             }
//         }
//     }
// }
// impl Error for ServerError {}

pub fn render_app_async_with_context<IV>(
    options: LeptosOptions,
    additional_context: impl Fn(leptos::Scope) + 'static + Clone + Send,
    app_fn: impl Fn(leptos::Scope) -> IV + Clone + Send + 'static,
) -> impl Handler
where
    IV: IntoView,
{
    move |req: Request| {
        let options = options.clone();
        let app_fn = app_fn.clone();
        // let add_context = additional_context.clone();
        let res_options = gloo_net::http::ResponseBuilder::default();

        async move {
            let app = {
                let app_fn = app_fn.clone();
                let res_options = res_options.clone();
                move |cx| {
                    let req_copy = req.try_clone().unwrap();
                    provide_contexts(cx, req_copy, res_options);
                    (app_fn)(cx).into_view(cx)
                }
            };
            let out = render_app_async_helper(
                &options,
                app,
                res_options,
                additional_context,
            )
            .await;
            out.into()
        }
    }
}

fn render_app_to_stream_with_context<IV>(
    options: LeptosOptions,
    additional_context: impl Fn(leptos::Scope) + 'static + Clone + Send,
    app_fn: impl Fn(leptos::Scope) -> IV + Clone + 'static,
) -> impl Handler
where
    IV: IntoView,
{
    
    move |req: Request| {
        let options = options.clone();
        let app_fn = app_fn.clone();
        let additional_context = additional_context.clone();
        let res_options = ResponseBuilder::default();

        Box::pin(async move {
            let app = {
                let app_fn = app_fn.clone();
                let res_options = res_options.clone();
                move |cx| {
                    provide_contexts(cx, req, res_options);
                    (app_fn)(cx).into_view(cx)
                }
            };
            stream_app(&options, app, res_options, additional_context).await
        })
    }
}

pub fn render_app_to_stream_in_order_with_context<IV>(
    options: LeptosOptions,
    additional_context: impl Fn(leptos::Scope) + 'static + Clone + Send,
    app_fn: impl Fn(leptos::Scope) -> IV + Clone + 'static,
) -> impl Handler
where
    IV: IntoView,
{
    move |req: Request| {
        let options = options.clone();
        let app_fn = app_fn.clone();
        let additional_context = additional_context.clone();
        let res_options = ResponseBuilder::default();

        async move {
            let app = {
                let app_fn = app_fn.clone();
                let res_options = res_options.clone();
                move |cx| {
                    provide_contexts(cx, req, res_options);
                    (app_fn)(cx).into_view(cx)
                }
            };

            stream_app_in_order(&options, app, res_options, additional_context)
                .await
        }
    }
}

async fn stream_app_in_order(
    options: &LeptosOptions,
    app: impl FnOnce(leptos::Scope) -> View + 'static,
    res_options: ResponseBuilder,
    additional_context: impl Fn(leptos::Scope) + 'static + Clone + Send,
) -> Response {
    let (stream, runtime, scope) = leptos::ssr::render_to_stream_in_order_with_prefix_undisposed_with_context(
        app,
        move |cx| {
            generate_head_metadata_separated(cx).1.into()
        },
        additional_context
    );

    build_stream_response(options, res_options, stream, runtime, scope).await
}

async fn stream_app(
    options: &LeptosOptions,
    app: impl FnOnce(leptos::Scope) -> View + 'static,
    res_options: ResponseBuilder,
    additional_context: impl Fn(leptos::Scope) + 'static + Clone + Send,
) -> Response {
    let (stream, runtime, scope) =
        render_to_stream_with_prefix_undisposed_with_context(
            app,
            move |cx| generate_head_metadata_separated(cx).1.into(),
            additional_context,
        );
    build_stream_response(options, res_options, stream, runtime, scope).await
}

async fn build_stream_response(
    options: &LeptosOptions,
    res_options: ResponseBuilder,
    stream: impl Stream<Item = String> + 'static,
    runtime: RuntimeId,
    scope: ScopeId,
) -> Response {
    let cx = leptos::Scope { runtime, id: scope };
    let (head, tail) =
        html_parts_separated(options, use_context::<MetaContext>(cx).as_ref());

    let mut stream = Box::pin(
        futures::stream::once(async move { head.clone() })
            .chain(stream)
            .chain(futures::stream::once(async move {
                runtime.dispose();
                tail.to_string()
            }))
            .map(|html| -> Result<JsValue, JsValue> {
                let bytes = html.into_bytes();
                let arr = Uint8Array::new_with_length(bytes.len() as _);
                arr.copy_from(&bytes);
                Ok(arr.into())
            }),
    );

    let first_chunk = stream.next().await;
    let second_chunk = stream.next().await;

    let complete_stream =
        futures::stream::iter([first_chunk.unwrap(), second_chunk.unwrap()])
            .chain(stream);

    let js_stream: web_sys::ReadableStream =
        wasm_streams::ReadableStream::from_stream(complete_stream)
            .into_raw()
            .unchecked_into();

    res_options.body(Some(&js_stream)).unwrap()
}

pub fn redirect(cx: leptos::Scope, path: &str) {
    if let Some(response_options) = use_context::<ResponseBuilder>(cx) {
        response_options.status(302)
            .status_text("Found")
            .header("Location", path);
    }
}

async fn render_app_async_helper(
    options: &LeptosOptions,
    app: impl FnOnce(leptos::Scope) -> View + 'static,
    res_options: ResponseBuilder,
    additional_context: impl Fn(leptos::Scope) + 'static + Clone + Send,
) -> web_sys::Response {
    let (stream, runtime, scope) =
        leptos::ssr::render_to_stream_in_order_with_prefix_undisposed_with_context(
            app,
            move |_| "".into(),
            additional_context,
        );

    let html = build_async_response(stream, options, runtime, scope).await;

    let res = res_options
        .header("Content-Type", "text/html; charset=utf-8")
        .status(200)
        .body(Some(html.as_str()))
        .unwrap();

    res.into()
}

async fn generate_response(
    res_options: ResponseBuilder,
    rx: Receiver<String>,
) -> web_sys::Response {
    let mut stream = Box::pin(rx.map(|html| -> Result<JsValue, JsValue> {
        let bytes = html.into_bytes();
        let arr = Uint8Array::new_with_length(bytes.len() as _);
        arr.copy_from(&bytes);
        Ok(arr.into())
    }));

    let first_chunk = stream.next().await;
    let second_chunk = stream.next().await;

    let complete_stream =
        futures::stream::iter([first_chunk.unwrap(), second_chunk.unwrap()])
            .chain(stream);

    let js_stream: web_sys::ReadableStream =
        wasm_streams::ReadableStream::from_stream(complete_stream)
            .into_raw()
            .unchecked_into();

    res_options.body(Some(&js_stream)).unwrap().into()
}

struct RequestWrapper(pub Option<Request>);

impl Clone for RequestWrapper {
    fn clone(&self) -> Self {
        match &self.0 {
            Some(req) => {
                let req = req.try_clone().ok();
                RequestWrapper(req)
            }
            None => RequestWrapper(None),
        }
    }
}

impl RequestWrapper {
    fn new(req: Request) -> Self {
        RequestWrapper(Some(req))
    }
}

fn provide_contexts(
    cx: leptos::Scope,
    req: Request,
    default_res_options: ResponseBuilder,
) {
    let path = Url::parse(&req.url()).unwrap().path().to_owned();
    let integration = ServerIntegration { path };
    provide_context(cx, RouterIntegrationContext::new(integration));
    provide_context(cx, MetaContext::new());
    provide_context(cx, RequestWrapper::new(req));
    provide_context(cx, default_res_options);
    provide_server_redirect(cx, move |path| redirect(cx, path))
}

impl LeptosRoutes for DefaultRouter {
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
                let method = GlooMethod::from(&method).0;
                let id = RouteIdentifier {
                    method,
                    pathname: path.to_string()
                };

                router.add_route(
                    id,
                    match listing.mode() {
                        SsrMode::OutOfOrder => {
                            Box::new(render_app_to_stream_with_context(
                                options.clone(),
                                additional_context.clone(),
                                app_fn.clone(),
                            ))
                        }
                        SsrMode::InOrder => {
                            Box::new(render_app_to_stream_in_order_with_context(
                                options.clone(),
                                additional_context.clone(),
                                app_fn.clone(),
                            ))
                        }
                        SsrMode::Async => Box::new(render_app_async_with_context(
                            options.clone(),
                            additional_context.clone(),
                            app_fn.clone(),
                        )),
                    },
                ).unwrap();
            }
        }
        router
    }

}

