#![forbid(unsafe_code)]

//! Provides functions to easily integrate Leptos with Actix.
//!
//! For more details on how to use the integrations, see the
//! [`examples`](https://github.com/leptos-rs/leptos/tree/main/examples)
//! directory in the Leptos repository.

use actix_http::header::{HeaderName, HeaderValue, ACCEPT, LOCATION, REFERER};
use actix_web::{
    body::BoxBody,
    dev::{ServiceFactory, ServiceRequest},
    http::header,
    web::{Payload, ServiceConfig},
    *,
};
use futures::{stream::once, Stream, StreamExt};
use http::StatusCode;
use leptos::{
    context::{provide_context, use_context},
    reactive_graph::{computed::ScopedFuture, owner::Owner},
    IntoView, *,
};
use leptos_integration_utils::{
    BoxedFnOnce, ExtendResponse, PinnedFuture, PinnedStream,
};
use leptos_meta::ServerMetaContext;
use leptos_router::{
    components::provide_server_redirect, location::RequestUrl, PathSegment,
    RouteList, RouteListing, SsrMode, StaticDataMap, StaticMode, *,
};
use parking_lot::RwLock;
use send_wrapper::SendWrapper;
use server_fn::{
    redirect::REDIRECT_HEADER, request::actix::ActixRequest, ServerFnError,
};
use std::{
    fmt::{Debug, Display},
    ops::{Deref, DerefMut},
    sync::Arc,
};

/// This struct lets you define headers and override the status of the Response from an Element or a Server Function
/// Typically contained inside of a ResponseOptions. Setting this is useful for cookies and custom responses.
#[derive(Debug, Clone, Default)]
pub struct ResponseParts {
    pub headers: header::HeaderMap,
    pub status: Option<StatusCode>,
}

impl ResponseParts {
    /// Insert a header, overwriting any previous value with the same key
    pub fn insert_header(
        &mut self,
        key: header::HeaderName,
        value: header::HeaderValue,
    ) {
        self.headers.insert(key, value);
    }

    /// Append a header, leaving any header with the same key intact
    pub fn append_header(
        &mut self,
        key: header::HeaderName,
        value: header::HeaderValue,
    ) {
        self.headers.append(key, value);
    }
}

/// A wrapper for an Actix [`HttpRequest`](actix_web::HttpRequest) that allows it to be used in an
/// `Send`/`Sync` setting like Leptos's Context API.
#[derive(Debug, Clone)]
pub struct Request(SendWrapper<HttpRequest>);

impl Request {
    pub fn new(req: &HttpRequest) -> Self {
        Self(SendWrapper::new(req.clone()))
    }

    pub fn into_inner(self) -> HttpRequest {
        self.0.take()
    }
}

impl Deref for Request {
    type Target = HttpRequest;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Request {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
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
    pub fn insert_header(
        &self,
        key: header::HeaderName,
        value: header::HeaderValue,
    ) {
        let mut writeable = self.0.write();
        let res_parts = &mut *writeable;
        res_parts.headers.insert(key, value);
    }
    /// Append a header, leaving any header with the same key intact.
    pub fn append_header(
        &self,
        key: header::HeaderName,
        value: header::HeaderValue,
    ) {
        let mut writeable = self.0.write();
        let res_parts = &mut *writeable;
        res_parts.headers.append(key, value);
    }
}

struct ActixResponse(HttpResponse);

impl ExtendResponse for ActixResponse {
    type ResponseOptions = ResponseOptions;

    fn from_stream(
        stream: impl Stream<Item = String> + Send + 'static,
    ) -> Self {
        ActixResponse(
            HttpResponse::Ok()
                .content_type("text/html")
                .streaming(stream.map(|chunk| {
                    Ok(web::Bytes::from(chunk)) as Result<web::Bytes>
                })),
        )
    }

    fn extend_response(&mut self, res_options: &Self::ResponseOptions) {
        let mut res_options = res_options.0.write();

        let headers = self.0.headers_mut();
        for (key, value) in std::mem::take(&mut res_options.headers) {
            headers.append(key, value);
        }

        // Set status to what is returned in the function
        if let Some(status) = res_options.status {
            *self.0.status_mut() = status;
        }
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
#[tracing::instrument(level = "trace", fields(error), skip_all)]
pub fn redirect(path: &str) {
    if let (Some(req), Some(res)) =
        (use_context::<Request>(), use_context::<ResponseOptions>())
    {
        // insert the Location header in any case
        res.insert_header(
            header::LOCATION,
            header::HeaderValue::from_str(path)
                .expect("Failed to create HeaderValue"),
        );

        let accepts_html = req
            .headers()
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

/// An Actix [struct@Route](actix_web::Route) that listens for a `POST` request with
/// Leptos server function arguments in the body, runs the server function if found,
/// and returns the resulting [HttpResponse].
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
///
/// ## Provided Context Types
/// This function always provides context values including the following types:
/// - [ResponseOptions]
/// - [HttpRequest](actix_web::HttpRequest)
#[tracing::instrument(level = "trace", fields(error), skip_all)]
pub fn handle_server_fns() -> Route {
    handle_server_fns_with_context(|| {})
}

/// An Actix [struct@Route](actix_web::Route) that listens for `GET` or `POST` requests with
/// Leptos server function arguments in the URL (`GET`) or body (`POST`),
/// runs the server function if found, and returns the resulting [HttpResponse].
///
/// This can then be set up at an appropriate route in your application:
///
/// This version allows you to pass in a closure that adds additional route data to the
/// context, allowing you to pass in info about the route or user from Actix, or other info.
///
/// **NOTE**: If your server functions expect a context, make sure to provide it both in
/// [`handle_server_fns_with_context`] **and** in [`LeptosRoutes::leptos_routes_with_context`] (or whatever
/// rendering method you are using). During SSR, server functions are called by the rendering
/// method, while subsequent calls from the client are handled by the server function handler.
/// The same context needs to be provided to both handlers.
///
/// ## Provided Context Types
/// This function always provides context values including the following types:
/// - [ResponseOptions]
/// - [HttpRequest](actix_web::HttpRequest)
#[tracing::instrument(level = "trace", fields(error), skip_all)]
pub fn handle_server_fns_with_context(
    additional_context: impl Fn() + 'static + Clone + Send,
) -> Route {
    web::to(move |req: HttpRequest, payload: Payload| {
        let additional_context = additional_context.clone();
        async move {
            let additional_context = additional_context.clone();

            let path = req.path();
            if let Some(mut service) =
                server_fn::actix::get_server_fn_service(path)
            {
                let owner = Owner::new();
                owner
                    .with(|| {
                        ScopedFuture::new(async move {
                            additional_context();
                            provide_context(Request::new(&req));
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
                            let mut res = ActixResponse(
                                service
                                    .0
                                    .run(ActixRequest::from((req, payload)))
                                    .await
                                    .take(),
                            );

                            // it it accepts text/html (i.e., is a plain form post) and doesn't already have a
                            // Location set, then redirect to to Referer
                            if accepts_html {
                                if let Some(referrer) = referrer {
                                    let has_location =
                                        res.0.headers().get(LOCATION).is_some();
                                    if !has_location {
                                        *res.0.status_mut() = StatusCode::FOUND;
                                        res.0
                                            .headers_mut()
                                            .insert(LOCATION, referrer);
                                    }
                                }
                            }

                            // apply status code and headers if used changed them
                            res.extend_response(&res_options);
                            res.0
                        })
                    })
                    .await
            } else {
                HttpResponse::BadRequest().body(format!(
                    "Could not find a server function at the route {:?}. \
                     \n\nIt's likely that either
                         1. The API prefix you specify in the `#[server]` \
                     macro doesn't match the prefix at which your server \
                     function handler is mounted, or \n2. You are on a \
                     platform that doesn't support automatic server function \
                     registration and you need to call \
                     ServerFn::register_explicit() on the server function \
                     type, somewhere in your `main` function.",
                    req.path()
                ))
            }
        }
    })
}

/// Returns an Actix [struct@Route](actix_web::Route) that listens for a `GET` request and tries
/// to route it using [leptos_router], serving an HTML stream of your application. The stream
/// will include fallback content for any `<Suspense/>` nodes, and be immediately interactive,
/// but requires some client-side JavaScript.
///
/// The provides a [MetaContext] and a [RouterIntegrationContext] to app’s context before
/// rendering it, and includes any meta tags injected using [leptos_meta].
///
/// The HTML stream is rendered using [render_to_stream](leptos::ssr::render_to_stream), and
/// includes everything described in the documentation for that function.
///
/// This can then be set up at an appropriate route in your application:
/// ```
/// use actix_web::{App, HttpServer};
/// use leptos::*;
/// use leptos_router::Method;
/// use std::{env, net::SocketAddr};
///
/// #[component]
/// fn MyApp() -> impl IntoView {
///     view! { <main>"Hello, world!"</main> }
/// }
///
/// # if false { // don't actually try to run a server in a doctest...
/// #[actix_web::main]
/// async fn main() -> std::io::Result<()> {
///     let conf = get_configuration(Some("Cargo.toml")).await.unwrap();
///     let addr = conf.leptos_options.site_addr.clone();
///     HttpServer::new(move || {
///         let leptos_options = &conf.leptos_options;
///
///         App::new()
///             // {tail:.*} passes the remainder of the URL as the route
///             // the actual routing will be handled by `leptos_router`
///             .route(
///                 "/{tail:.*}",
///                 leptos_actix::render_app_to_stream(
///                     leptos_options.to_owned(),
///                     || view! { <MyApp/> },
///                     Method::Get,
///                 ),
///             )
///     })
///     .bind(&addr)?
///     .run()
///     .await
/// }
/// # }
/// ```
///
/// ## Provided Context Types
/// This function always provides context values including the following types:
/// - [ResponseOptions]
/// - [HttpRequest](actix_web::HttpRequest)
/// - [MetaContext](leptos_meta::MetaContext)
/// - [RouterIntegrationContext](leptos_router::RouterIntegrationContext)
#[tracing::instrument(level = "trace", fields(error), skip_all)]
pub fn render_app_to_stream<IV>(
    app_fn: impl Fn() -> IV + Clone + Send + 'static,
    method: Method,
) -> Route
where
    IV: IntoView + 'static,
{
    render_app_to_stream_with_context(|| {}, app_fn, method)
}

/// Returns an Actix [struct@Route](actix_web::Route) that listens for a `GET` request and tries
/// to route it using [leptos_router], serving an in-order HTML stream of your application.
/// This stream will pause at each `<Suspense/>` node and wait for it to resolve before
/// sending down its HTML. The app will become interactive once it has fully loaded.
///
/// The provides a [MetaContext] and a [RouterIntegrationContext] to app’s context before
/// rendering it, and includes any meta tags injected using [leptos_meta].
///
/// The HTML stream is rendered using
/// [render_to_stream_in_order](leptos::ssr::render_to_stream_in_order),
/// and includes everything described in the documentation for that function.
///
/// This can then be set up at an appropriate route in your application:
/// ```
/// use actix_web::{App, HttpServer};
/// use leptos::*;
/// use leptos_router::Method;
/// use std::{env, net::SocketAddr};
///
/// #[component]
/// fn MyApp() -> impl IntoView {
///     view! { <main>"Hello, world!"</main> }
/// }
///
/// # if false { // don't actually try to run a server in a doctest...
/// #[actix_web::main]
/// async fn main() -> std::io::Result<()> {
///     let conf = get_configuration(Some("Cargo.toml")).await.unwrap();
///     let addr = conf.leptos_options.site_addr.clone();
///     HttpServer::new(move || {
///         let leptos_options = &conf.leptos_options;
///
///         App::new()
///             // {tail:.*} passes the remainder of the URL as the route
///             // the actual routing will be handled by `leptos_router`
///             .route(
///                 "/{tail:.*}",
///                 leptos_actix::render_app_to_stream_in_order(
///                     leptos_options.to_owned(),
///                     || view! { <MyApp/> },
///                     Method::Get,
///                 ),
///             )
///     })
///     .bind(&addr)?
///     .run()
///     .await
/// }
/// # }
/// ```
///
/// ## Provided Context Types
/// This function always provides context values including the following types:
/// - [ResponseOptions]
/// - [HttpRequest](actix_web::HttpRequest)
/// - [MetaContext](leptos_meta::MetaContext)
/// - [RouterIntegrationContext](leptos_router::RouterIntegrationContext)
#[tracing::instrument(level = "trace", fields(error), skip_all)]
pub fn render_app_to_stream_in_order<IV>(
    app_fn: impl Fn() -> IV + Clone + Send + 'static,
    method: Method,
) -> Route
where
    IV: IntoView + 'static,
{
    render_app_to_stream_in_order_with_context(|| {}, app_fn, method)
}

/// Returns an Actix [struct@Route](actix_web::Route) that listens for a `GET` request and tries
/// to route it using [leptos_router], asynchronously rendering an HTML page after all
/// `async` [Resource](leptos::Resource)s have loaded.
///
/// The provides a [MetaContext] and a [RouterIntegrationContext] to the app’s context before
/// rendering it, and includes any meta tags injected using [leptos_meta].
///
/// The HTML stream is rendered using [render_to_string_async](leptos::ssr::render_to_string_async), and
/// includes everything described in the documentation for that function.
///
/// This can then be set up at an appropriate route in your application:
/// ```
/// use actix_web::{App, HttpServer};
/// use leptos::*;
/// use leptos_router::Method;
/// use std::{env, net::SocketAddr};
///
/// #[component]
/// fn MyApp() -> impl IntoView {
///     view! { <main>"Hello, world!"</main> }
/// }
///
/// # if false { // don't actually try to run a server in a doctest...
/// #[actix_web::main]
/// async fn main() -> std::io::Result<()> {
///     let conf = get_configuration(Some("Cargo.toml")).await.unwrap();
///     let addr = conf.leptos_options.site_addr.clone();
///     HttpServer::new(move || {
///         let leptos_options = &conf.leptos_options;
///
///         App::new()
///             // {tail:.*} passes the remainder of the URL as the route
///             // the actual routing will be handled by `leptos_router`
///             .route(
///                 "/{tail:.*}",
///                 leptos_actix::render_app_async(
///                     leptos_options.to_owned(),
///                     || view! { <MyApp/> },
///                     Method::Get,
///                 ),
///             )
///     })
///     .bind(&addr)?
///     .run()
///     .await
/// }
/// # }
/// ```
///
/// ## Provided Context Types
/// This function always provides context values including the following types:
/// - [ResponseOptions]
/// - [HttpRequest](actix_web::HttpRequest)
/// - [MetaContext](leptos_meta::MetaContext)
/// - [RouterIntegrationContext](leptos_router::RouterIntegrationContext)
#[tracing::instrument(level = "trace", fields(error), skip_all)]
pub fn render_app_async<IV>(
    app_fn: impl Fn() -> IV + Clone + Send + 'static,
    method: Method,
) -> Route
where
    IV: IntoView + 'static,
{
    render_app_async_with_context(|| {}, app_fn, method)
}

/// Returns an Actix [struct@Route] that listens for a `GET` request and tries
/// to route it using [leptos_router], serving an HTML stream of your application.
///
/// This function allows you to provide additional information to Leptos for your route.
/// It could be used to pass in Path Info, Connection Info, or anything your heart desires.
///
/// ## Provided Context Types
/// This function always provides context values including the following types:
/// - [ResponseOptions]
/// - [HttpRequest](actix_web::HttpRequest)
/// - [MetaContext](leptos_meta::MetaContext)
/// - [RouterIntegrationContext](leptos_router::RouterIntegrationContext)
#[tracing::instrument(level = "trace", fields(error), skip_all)]
pub fn render_app_to_stream_with_context<IV>(
    additional_context: impl Fn() + 'static + Clone + Send,
    app_fn: impl Fn() -> IV + Clone + Send + 'static,
    method: Method,
) -> Route
where
    IV: IntoView + 'static,
{
    render_app_to_stream_with_context_and_replace_blocks(
        additional_context,
        app_fn,
        method,
        false,
    )
}

/// Returns an Actix [struct@Route](actix_web::Route) that listens for a `GET` request and tries
/// to route it using [leptos_router], serving an HTML stream of your application.
///
/// This function allows you to provide additional information to Leptos for your route.
/// It could be used to pass in Path Info, Connection Info, or anything your heart desires.
///
/// `replace_blocks` additionally lets you specify whether `<Suspense/>` fragments that read
/// from blocking resources should be retrojected into the HTML that's initially served, rather
/// than dynamically inserting them with JavaScript on the client. This means you will have
/// better support if JavaScript is not enabled, in exchange for a marginally slower response time.
///
/// ## Provided Context Types
/// This function always provides context values including the following types:
/// - [ResponseOptions]
/// - [HttpRequest](actix_web::HttpRequest)
/// - [MetaContext](leptos_meta::MetaContext)
/// - [RouterIntegrationContext](leptos_router::RouterIntegrationContext)
#[tracing::instrument(level = "trace", fields(error), skip_all)]
pub fn render_app_to_stream_with_context_and_replace_blocks<IV>(
    additional_context: impl Fn() + 'static + Clone + Send,
    app_fn: impl Fn() -> IV + Clone + Send + 'static,
    method: Method,
    replace_blocks: bool,
) -> Route
where
    IV: IntoView + 'static,
{
    _ = replace_blocks; // TODO
    handle_response(method, additional_context, app_fn, |app, chunks| {
        Box::pin(async move {
            Box::pin(app.to_html_stream_out_of_order().chain(chunks()))
                as PinnedStream<String>
        })
    })
}

/// Returns an Actix [struct@Route](actix_web::Route) that listens for a `GET` request and tries
/// to route it using [leptos_router], serving an in-order HTML stream of your application.
///
/// This function allows you to provide additional information to Leptos for your route.
/// It could be used to pass in Path Info, Connection Info, or anything your heart desires.
///
/// ## Provided Context Types
/// This function always provides context values including the following types:
/// - [ResponseOptions]
/// - [HttpRequest](actix_web::HttpRequest)
/// - [MetaContext](leptos_meta::MetaContext)
/// - [RouterIntegrationContext](leptos_router::RouterIntegrationContext)
#[tracing::instrument(level = "trace", fields(error), skip_all)]
pub fn render_app_to_stream_in_order_with_context<IV>(
    additional_context: impl Fn() + 'static + Clone + Send,
    app_fn: impl Fn() -> IV + Clone + Send + 'static,
    method: Method,
) -> Route
where
    IV: IntoView + 'static,
{
    handle_response(method, additional_context, app_fn, |app, chunks| {
        Box::pin(async move {
            Box::pin(app.to_html_stream_in_order().chain(chunks()))
                as PinnedStream<String>
        })
    })
}

/// Returns an Actix [struct@Route](actix_web::Route) that listens for a `GET` request and tries
/// to route it using [leptos_router], asynchronously serving the page once all `async`
/// [Resource](leptos::Resource)s have loaded.
///
/// This function allows you to provide additional information to Leptos for your route.
/// It could be used to pass in Path Info, Connection Info, or anything your heart desires.
///
/// ## Provided Context Types
/// This function always provides context values including the following types:
/// - [ResponseOptions]
/// - [HttpRequest](actix_web::HttpRequest)
/// - [MetaContext](leptos_meta::MetaContext)
/// - [RouterIntegrationContext](leptos_router::RouterIntegrationContext)
#[tracing::instrument(level = "trace", fields(error), skip_all)]
pub fn render_app_async_with_context<IV>(
    additional_context: impl Fn() + 'static + Clone + Send,
    app_fn: impl Fn() -> IV + Clone + Send + 'static,
    method: Method,
) -> Route
where
    IV: IntoView + 'static,
{
    handle_response(method, additional_context, app_fn, |app, chunks| {
        Box::pin(async move {
            let app = app.to_html_stream_in_order().collect::<String>().await;
            let chunks = chunks();
            Box::pin(once(async move { app }).chain(chunks))
                as PinnedStream<String>
        })
    })
}

#[tracing::instrument(level = "trace", fields(error), skip_all)]
fn provide_contexts(
    req: Request,
    meta_context: &ServerMetaContext,
    res_options: &ResponseOptions,
) {
    let path = leptos_corrected_path(&req);

    provide_context(RequestUrl::new(&path));
    provide_context(meta_context.clone());
    provide_context(res_options.clone());
    provide_context(req);
    provide_server_redirect(redirect);
    #[cfg(feature = "nonce")]
    leptos::nonce::provide_nonce();
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

fn handle_response<IV>(
    method: Method,
    additional_context: impl Fn() + 'static + Clone + Send,
    app_fn: impl Fn() -> IV + Clone + Send + 'static,
    stream_builder: fn(
        IV,
        BoxedFnOnce<PinnedStream<String>>,
    ) -> PinnedFuture<PinnedStream<String>>,
) -> Route
where
    IV: IntoView + 'static,
{
    let handler = move |req: HttpRequest| {
        let app_fn = app_fn.clone();
        let add_context = additional_context.clone();

        async move {
            let res_options = ResponseOptions::default();
            let meta_context = ServerMetaContext::new();

            let additional_context = {
                let meta_context = meta_context.clone();
                let res_options = res_options.clone();
                let req = Request::new(&req);
                move || {
                    provide_contexts(req, &meta_context, &res_options);
                    add_context();
                }
            };

            let res = ActixResponse::from_app(
                app_fn,
                meta_context,
                additional_context,
                res_options,
                stream_builder,
            )
            .await;

            res.0
        }
    };
    match method {
        Method::Get => web::get().to(handler),
        Method::Post => web::post().to(handler),
        Method::Put => web::put().to(handler),
        Method::Delete => web::delete().to(handler),
        Method::Patch => web::patch().to(handler),
    }
}

/// Generates a list of all routes defined in Leptos's Router in your app. We can then use this to automatically
/// create routes in Actix's App without having to use wildcard matching or fallbacks. Takes in your root app Element
/// as an argument so it can walk you app tree. This version is tailored to generated Actix compatible paths.
pub fn generate_route_list<IV>(
    app_fn: impl Fn() -> IV + 'static + Clone,
) -> Vec<ActixRouteListing>
where
    IV: IntoView + 'static,
{
    generate_route_list_with_exclusions_and_ssg(app_fn, None).0
}

/// Generates a list of all routes defined in Leptos's Router in your app. We can then use this to automatically
/// create routes in Actix's App without having to use wildcard matching or fallbacks. Takes in your root app Element
/// as an argument so it can walk you app tree. This version is tailored to generated Actix compatible paths.
pub fn generate_route_list_with_ssg<IV>(
    app_fn: impl Fn() -> IV + 'static + Clone,
) -> (Vec<ActixRouteListing>, StaticDataMap)
where
    IV: IntoView + 'static,
{
    generate_route_list_with_exclusions_and_ssg(app_fn, None)
}

/// Generates a list of all routes defined in Leptos's Router in your app. We can then use this to automatically
/// create routes in Actix's App without having to use wildcard matching or fallbacks. Takes in your root app Element
/// as an argument so it can walk you app tree. This version is tailored to generated Actix compatible paths. Adding excluded_routes
/// to this function will stop `.leptos_routes()` from generating a route for it, allowing a custom handler. These need to be in Actix path format
pub fn generate_route_list_with_exclusions<IV>(
    app_fn: impl Fn() -> IV + 'static + Clone,
    excluded_routes: Option<Vec<String>>,
) -> Vec<ActixRouteListing>
where
    IV: IntoView + 'static,
{
    generate_route_list_with_exclusions_and_ssg(app_fn, excluded_routes).0
}

/// Generates a list of all routes defined in Leptos's Router in your app. We can then use this to automatically
/// create routes in Actix's App without having to use wildcard matching or fallbacks. Takes in your root app Element
/// as an argument so it can walk you app tree. This version is tailored to generated Actix compatible paths. Adding excluded_routes
/// to this function will stop `.leptos_routes()` from generating a route for it, allowing a custom handler. These need to be in Actix path format
pub fn generate_route_list_with_exclusions_and_ssg<IV>(
    app_fn: impl Fn() -> IV + 'static + Clone,
    excluded_routes: Option<Vec<String>>,
) -> (Vec<ActixRouteListing>, StaticDataMap)
where
    IV: IntoView + 'static,
{
    generate_route_list_with_exclusions_and_ssg_and_context(
        app_fn,
        excluded_routes,
        || {},
    )
}

trait ActixPath {
    fn to_actix_path(&self) -> String;
}

impl ActixPath for &[PathSegment] {
    fn to_actix_path(&self) -> String {
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
                    path.push('{');
                    path.push_str(s);
                    path.push('}');
                }
                PathSegment::Splat(s) => {
                    path.push('{');
                    path.push_str(s);
                    path.push_str(":.*}");
                }
                PathSegment::Unit => {}
            }
        }
        path
    }
}

#[derive(Clone, Debug, Default)]
/// A route that this application can serve.
pub struct ActixRouteListing {
    path: String,
    mode: SsrMode,
    methods: Vec<leptos_router::Method>,
    static_mode: Option<(StaticMode, StaticDataMap)>,
}

impl From<RouteListing> for ActixRouteListing {
    fn from(value: RouteListing) -> Self {
        let path = value.path().to_actix_path();
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

impl ActixRouteListing {
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
/// create routes in Actix's App without having to use wildcard matching or fallbacks. Takes in your root app Element
/// as an argument so it can walk you app tree. This version is tailored to generated Actix compatible paths. Adding excluded_routes
/// to this function will stop `.leptos_routes()` from generating a route for it, allowing a custom handler. These need to be in Actix path format.
/// Additional context will be provided to the app Element.
pub fn generate_route_list_with_exclusions_and_ssg_and_context<IV>(
    app_fn: impl Fn() -> IV + 'static + Clone,
    excluded_routes: Option<Vec<String>>,
    additional_context: impl Fn() + 'static + Clone,
) -> (Vec<ActixRouteListing>, StaticDataMap)
where
    IV: IntoView + 'static,
{
    let _ = any_spawner::Executor::init_tokio();

    let owner = Owner::new_root(None);
    let routes = owner
        .with(|| {
            // stub out a path for now
            provide_context(RequestUrl::new(""));
            provide_context(ResponseOptions::default());
            provide_context(ServerMetaContext::new());
            additional_context();
            RouteList::generate(&app_fn)
        })
        .unwrap_or_default();

    // Axum's Router defines Root routes as "/" not ""
    let mut routes = routes
        .into_inner()
        .into_iter()
        .map(ActixRouteListing::from)
        .collect::<Vec<_>>();

    (
        if routes.is_empty() {
            vec![ActixRouteListing::new(
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

pub enum DataResponse<T> {
    Data(T),
    Response(actix_web::dev::Response<BoxBody>),
}

// TODO static response
/*
fn handle_static_response<'a, IV>(
    path: &'a str,
    options: &'a LeptosOptions,
    app_fn: &'a (impl Fn() -> IV + Clone + Send + 'static),
    additional_context: &'a (impl Fn() + 'static + Clone + Send),
    res: StaticResponse,
) -> Pin<Box<dyn Future<Output = HttpResponse<String>> + 'a>>
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
                let mut res = HttpResponse::new(match status {
                    StaticStatusCode::Ok => StatusCode::OK,
                    StaticStatusCode::NotFound => StatusCode::NOT_FOUND,
                    StaticStatusCode::InternalServerError => {
                        StatusCode::INTERNAL_SERVER_ERROR
                    }
                });
                if let Some(v) = content_type {
                    res.headers_mut().insert(
                        HeaderName::from_static("content-type"),
                        HeaderValue::from_static(v),
                    );
                }
                res.set_body(body)
            }
            StaticResponse::RenderDynamic => {
                handle_static_response(
                    path,
                    options,
                    app_fn,
                    additional_context,
                    render_dynamic(
                        path,
                        options,
                        app_fn.clone(),
                        additional_context.clone(),
                    )
                    .await,
                )
                .await
            }
            StaticResponse::RenderNotFound => {
                handle_static_response(
                    path,
                    options,
                    app_fn,
                    additional_context,
                    not_found_page(
                        tokio::fs::read_to_string(not_found_path(options))
                            .await,
                    ),
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
                    path.to_str().unwrap(),
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
    options: LeptosOptions,
    app_fn: impl Fn() -> IV + Clone + Send + 'static,
    additional_context: impl Fn() + 'static + Clone + Send,
    method: Method,
    mode: StaticMode,
) -> Route
where
    IV: IntoView + 'static,
{
    match mode {
        StaticMode::Incremental => {
            let handler = move |req: HttpRequest| {
                Box::pin({
                    let options = options.clone();
                    let app_fn = app_fn.clone();
                    let additional_context = additional_context.clone();
                    async move {
                        handle_static_response(
                            req.path(),
                            &options,
                            &app_fn,
                            &additional_context,
                            incremental_static_route(
                                tokio::fs::read_to_string(static_file_path(
                                    &options,
                                    req.path(),
                                ))
                                .await,
                            ),
                        )
                        .await
                    }
                })
            };
            match method {
                Method::Get => web::get().to(handler),
                Method::Post => web::post().to(handler),
                Method::Put => web::put().to(handler),
                Method::Delete => web::delete().to(handler),
                Method::Patch => web::patch().to(handler),
            }
        }
        StaticMode::Upfront => {
            let handler = move |req: HttpRequest| {
                Box::pin({
                    let options = options.clone();
                    let app_fn = app_fn.clone();
                    let additional_context = additional_context.clone();
                    async move {
                        handle_static_response(
                            req.path(),
                            &options,
                            &app_fn,
                            &additional_context,
                            upfront_static_route(
                                tokio::fs::read_to_string(static_file_path(
                                    &options,
                                    req.path(),
                                ))
                                .await,
                            ),
                        )
                        .await
                    }
                })
            };
            match method {
                Method::Get => web::get().to(handler),
                Method::Post => web::post().to(handler),
                Method::Put => web::put().to(handler),
                Method::Delete => web::delete().to(handler),
                Method::Patch => web::patch().to(handler),
            }
        }
    }
}
*/

/// This trait allows one to pass a list of routes and a render function to Actix's router, letting us avoid
/// having to use wildcards or manually define all routes in multiple places.
pub trait LeptosRoutes {
    fn leptos_routes<IV>(
        self,
        paths: Vec<ActixRouteListing>,
        app_fn: impl Fn() -> IV + Clone + Send + 'static,
    ) -> Self
    where
        IV: IntoView + 'static;

    fn leptos_routes_with_context<IV>(
        self,
        paths: Vec<ActixRouteListing>,
        additional_context: impl Fn() + 'static + Clone + Send,
        app_fn: impl Fn() -> IV + Clone + Send + 'static,
    ) -> Self
    where
        IV: IntoView + 'static;
}

/// The default implementation of `LeptosRoutes` which takes in a list of paths, and dispatches GET requests
/// to those paths to Leptos's renderer.
impl<T> LeptosRoutes for actix_web::App<T>
where
    T: ServiceFactory<
        ServiceRequest,
        Config = (),
        Error = Error,
        InitError = (),
    >,
{
    #[tracing::instrument(level = "trace", fields(error), skip_all)]
    fn leptos_routes<IV>(
        self,
        paths: Vec<ActixRouteListing>,
        app_fn: impl Fn() -> IV + Clone + Send + 'static,
    ) -> Self
    where
        IV: IntoView + 'static,
    {
        self.leptos_routes_with_context(paths, || {}, app_fn)
    }

    #[tracing::instrument(level = "trace", fields(error), skip_all)]
    fn leptos_routes_with_context<IV>(
        self,
        paths: Vec<ActixRouteListing>,
        additional_context: impl Fn() + 'static + Clone + Send,
        app_fn: impl Fn() -> IV + Clone + Send + 'static,
    ) -> Self
    where
        IV: IntoView + 'static,
    {
        let mut router = self;

        // register server functions first to allow for wildcard route in Leptos's Router
        for (path, _) in server_fn::actix::server_fn_paths() {
            let additional_context = additional_context.clone();
            let handler = handle_server_fns_with_context(additional_context);
            router = router.route(path, handler);
        }

        // register routes defined in Leptos's Router
        for listing in paths.iter() {
            let path = listing.path();
            let mode = listing.mode();

            for method in listing.methods() {
                let additional_context = additional_context.clone();
                let additional_context_and_method = move || {
                    provide_context(method);
                    additional_context();
                };
                router = if let Some(static_mode) = listing.static_mode() {
                    _ = static_mode;
                    todo!() /*
                            router.route(
                                path,
                                static_route(
                                    app_fn.clone(),
                                    additional_context_and_method.clone(),
                                    method,
                                    static_mode,
                                ),
                            )
                            */
                } else {
                    router.route(
                    path,
                    match mode {
                        SsrMode::OutOfOrder => {
                            render_app_to_stream_with_context(
                                additional_context_and_method.clone(),
                                app_fn.clone(),
                                method,
                            )
                        }
                        SsrMode::PartiallyBlocked => {
                            render_app_to_stream_with_context_and_replace_blocks(
                                additional_context_and_method.clone(),
                                app_fn.clone(),
                                method,
                                true,
                            )
                        }
                        SsrMode::InOrder => {
                            render_app_to_stream_in_order_with_context(
                                additional_context_and_method.clone(),
                                app_fn.clone(),
                                method,
                            )
                        }
                        SsrMode::Async => render_app_async_with_context(
                            additional_context_and_method.clone(),
                            app_fn.clone(),
                            method,
                        ),
                    },
                )
                };
            }
        }

        router
    }
}

/// The default implementation of `LeptosRoutes` which takes in a list of paths, and dispatches GET requests
/// to those paths to Leptos's renderer.
impl LeptosRoutes for &mut ServiceConfig {
    #[tracing::instrument(level = "trace", fields(error), skip_all)]
    fn leptos_routes<IV>(
        self,
        paths: Vec<ActixRouteListing>,
        app_fn: impl Fn() -> IV + Clone + Send + 'static,
    ) -> Self
    where
        IV: IntoView + 'static,
    {
        self.leptos_routes_with_context(paths, || {}, app_fn)
    }

    #[tracing::instrument(level = "trace", fields(error), skip_all)]
    fn leptos_routes_with_context<IV>(
        self,
        paths: Vec<ActixRouteListing>,
        additional_context: impl Fn() + 'static + Clone + Send,
        app_fn: impl Fn() -> IV + Clone + Send + 'static,
    ) -> Self
    where
        IV: IntoView + 'static,
    {
        let mut router = self;

        // register server functions first to allow for wildcard route in Leptos's Router
        for (path, _) in server_fn::actix::server_fn_paths() {
            let additional_context = additional_context.clone();
            let handler = handle_server_fns_with_context(additional_context);
            router = router.route(path, handler);
        }

        // register routes defined in Leptos's Router
        for listing in paths.iter() {
            let path = listing.path();
            let mode = listing.mode();

            for method in listing.methods() {
                router = router.route(
                            path,
                            match mode {
                                SsrMode::OutOfOrder => {
                                    render_app_to_stream_with_context(
                                        additional_context.clone(),
                                        app_fn.clone(),
                                        method,
                                    )
                                }
                                SsrMode::PartiallyBlocked => {
                                    render_app_to_stream_with_context_and_replace_blocks(
                                        additional_context.clone(),
                                        app_fn.clone(),
                                        method,
                                        true,
                                    )
                                }
                                SsrMode::InOrder => {
                                    render_app_to_stream_in_order_with_context(
                                        additional_context.clone(),
                                        app_fn.clone(),
                                        method,
                                    )
                                }
                                SsrMode::Async => render_app_async_with_context(
                                    additional_context.clone(),
                                    app_fn.clone(),
                                    method,
                                ),
                            },
                        );
            }
        }

        router
    }
}

/// A helper to make it easier to use Actix extractors in server functions.
///
/// It is generic over some type `T` that implements [`FromRequest`] and can
/// therefore be used in an extractor. The compiler can often infer this type.
///
/// Any error that occurs during extraction is converted to a [`ServerFnError`].
///
/// ```rust,ignore
/// // MyQuery is some type that implements `Deserialize + Serialize`
/// #[server]
/// pub async fn query_extract() -> Result<MyQuery, ServerFnError> {
///     use actix_web::web::Query;
///     use leptos_actix::*;
///
///     let Query(data) = extract().await?;
///
///     // do something with the data
///
///     Ok(data)
/// }
/// ```
pub async fn extract<T>() -> Result<T, ServerFnError>
where
    T: actix_web::FromRequest,
    <T as FromRequest>::Error: Display,
{
    let req = use_context::<HttpRequest>().ok_or_else(|| {
        ServerFnError::new("HttpRequest should have been provided via context")
    })?;

    T::extract(&req)
        .await
        .map_err(|e| ServerFnError::ServerError(e.to_string()))
}
