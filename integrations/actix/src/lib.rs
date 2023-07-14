#![forbid(unsafe_code)]

//! Provides functions to easily integrate Leptos with Actix.
//!
//! For more details on how to use the integrations, see the
//! [`examples`](https://github.com/leptos-rs/leptos/tree/main/examples)
//! directory in the Leptos repository.

use actix_web::{
    body::BoxBody,
    dev::{ServiceFactory, ServiceRequest},
    http::header,
    web::Bytes,
    *,
};
use futures::{Stream, StreamExt};
use http::StatusCode;
use leptos::{
    leptos_server::{server_fn_by_path, Payload},
    server_fn::Encoding,
    ssr::render_to_stream_with_prefix_undisposed_with_context_and_block_replacement,
    *,
};
use leptos_integration_utils::{build_async_response, html_parts_separated};
use leptos_meta::*;
use leptos_router::*;
use parking_lot::RwLock;
use regex::Regex;
use std::{fmt::Display, future::Future, sync::Arc};
use tracing::instrument;
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
    pub fn set_status(&self, status: StatusCode) {
        let mut writeable = self.0.write();
        let res_parts = &mut *writeable;
        res_parts.status = Some(status);
    }
    /// Insert a header, overwriting any previous value with the same key
    pub fn insert_header(
        &self,
        key: header::HeaderName,
        value: header::HeaderValue,
    ) {
        let mut writeable = self.0.write();
        let res_parts = &mut *writeable;
        res_parts.headers.insert(key, value);
    }
    /// Append a header, leaving any header with the same key intact
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

/// Provides an easy way to redirect the user from within a server function. Mimicking the Remix `redirect()`,
/// it sets a [StatusCode] of 302 and a [LOCATION](header::LOCATION) header with the provided value.
/// If looking to redirect from the client, `leptos_router::use_navigate()` should be used instead.
#[tracing::instrument(level = "trace", fields(error), skip_all)]
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
///
/// ## Provided Context Types
/// This function always provides context values including the following types:
/// - [ResponseOptions]
/// - [HttpRequest](actix_web::HttpRequest)
#[tracing::instrument(level = "trace", fields(error), skip_all)]
pub fn handle_server_fns() -> Route {
    handle_server_fns_with_context(|_cx| {})
}

/// An Actix [Route](actix_web::Route) that listens for `GET` or `POST` requests with
/// Leptos server function arguments in the URL (`GET`) or body (`POST`),
/// runs the server function if found, and returns the resulting [HttpResponse].
///
/// This provides the [HttpRequest] to the server [Scope](leptos::Scope).
///
/// This can then be set up at an appropriate route in your application:
///
/// This version allows you to pass in a closure that adds additional route data to the
/// context, allowing you to pass in info about the route or user from Actix, or other info.
///
/// ## Provided Context Types
/// This function always provides context values including the following types:
/// - [ResponseOptions]
/// - [HttpRequest](actix_web::HttpRequest)
#[tracing::instrument(level = "trace", fields(error), skip_all)]
pub fn handle_server_fns_with_context(
    additional_context: impl Fn(leptos::Scope) + 'static + Clone + Send,
) -> Route {
    web::to(
        move |req: HttpRequest, params: web::Path<String>, body: web::Bytes| {
            let additional_context = additional_context.clone();
            async move {
                let additional_context = additional_context.clone();

                let path = params.into_inner();
                let accept_header = req
                    .headers()
                    .get("Accept")
                    .and_then(|value| value.to_str().ok());

                if let Some(server_fn) = server_fn_by_path(path.as_str()) {
                    let body_ref: &[u8] = &body;

                    let runtime = create_runtime();
                    let (cx, disposer) = raw_scope_and_disposer(runtime);

                    // Add additional info to the context of the server function
                    additional_context(cx);
                    let res_options = ResponseOptions::default();

                    // provide HttpRequest as context in server scope
                    provide_context(cx, req.clone());
                    provide_context(cx, res_options.clone());

                    // we consume the body here (using the web::Bytes extractor), but it is required for things
                    // like MultipartForm
                    if req
                        .headers()
                        .get("Content-Type")
                        .and_then(|value| value.to_str().ok())
                        .map(|value| {
                            value.starts_with("multipart/form-data; boundary=")
                        })
                        == Some(true)
                    {
                        provide_context(cx, body.clone());
                    }

                    let query = req.query_string().as_bytes();

                    let data = match &server_fn.encoding() {
                        Encoding::Url | Encoding::Cbor => body_ref,
                        Encoding::GetJSON | Encoding::GetCBOR => query,
                    };
                    let res = match server_fn.call(cx, data).await {
                        Ok(serialized) => {
                            let res_options =
                                use_context::<ResponseOptions>(cx).unwrap();

                            let mut res: HttpResponseBuilder =
                                HttpResponse::Ok();
                            let res_parts = res_options.0.write();

                            // if accept_header isn't set to one of these, it's a form submit
                            // redirect back to the referrer if not redirect has been set
                            if accept_header != Some("application/json")
                                && accept_header
                                    != Some("application/x-www-form-urlencoded")
                                && accept_header != Some("application/cbor")
                            {
                                // Location will already be set if redirect() has been used
                                let has_location_set =
                                    res_parts.headers.get("Location").is_some();
                                if !has_location_set {
                                    let referer = req
                                        .headers()
                                        .get("Referer")
                                        .and_then(|value| value.to_str().ok())
                                        .unwrap_or("/");
                                    res = HttpResponse::SeeOther();
                                    res.insert_header(("Location", referer))
                                        .content_type("application/json");
                                }
                            };
                            // Override StatusCode if it was set in a Resource or Element
                            if let Some(status) = res_parts.status {
                                res.status(status);
                            }

                            // Use provided ResponseParts headers if they exist
                            let _count = res_parts
                                .headers
                                .clone()
                                .into_iter()
                                .map(|(k, v)| {
                                    res.append_header((k, v));
                                })
                                .count();

                            match serialized {
                                Payload::Binary(data) => {
                                    res.content_type("application/cbor");
                                    res.body(Bytes::from(data))
                                }
                                Payload::Url(data) => {
                                    res.content_type(
                                        "application/x-www-form-urlencoded",
                                    );
                                    res.body(data)
                                }
                                Payload::Json(data) => {
                                    res.content_type("application/json");
                                    res.body(data)
                                }
                            }
                        }
                        Err(e) => HttpResponse::InternalServerError().body(
                            serde_json::to_string(&e)
                                .unwrap_or_else(|_| e.to_string()),
                        ),
                    };
                    // clean up the scope
                    disposer.dispose();
                    runtime.dispose();
                    res
                } else {
                    HttpResponse::BadRequest().body(format!(
                        "Could not find a server function at the route {:?}. \
                         \n\nIt's likely that either 
                         1. The API prefix you specify in the `#[server]` \
                         macro doesn't match the prefix at which your server \
                         function handler is mounted, or \n2. You are on a \
                         platform that doesn't support automatic server \
                         function registration and you need to call \
                         ServerFn::register_explicit() on the server function \
                         type, somewhere in your `main` function.",
                        req.path()
                    ))
                }
            }
        },
    )
}

/// Returns an Actix [Route](actix_web::Route) that listens for a `GET` request and tries
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
/// fn MyApp(cx: Scope) -> impl IntoView {
///     view! { cx, <main>"Hello, world!"</main> }
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
///                     |cx| view! { cx, <MyApp/> },
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
    options: LeptosOptions,
    app_fn: impl Fn(leptos::Scope) -> IV + Clone + 'static,
    method: Method,
) -> Route
where
    IV: IntoView,
{
    render_app_to_stream_with_context(options, |_cx| {}, app_fn, method)
}

/// Returns an Actix [Route](actix_web::Route) that listens for a `GET` request and tries
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
/// fn MyApp(cx: Scope) -> impl IntoView {
///     view! { cx, <main>"Hello, world!"</main> }
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
///                     |cx| view! { cx, <MyApp/> },
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
    options: LeptosOptions,
    app_fn: impl Fn(leptos::Scope) -> IV + Clone + 'static,
    method: Method,
) -> Route
where
    IV: IntoView,
{
    render_app_to_stream_in_order_with_context(
        options,
        |_cx| {},
        app_fn,
        method,
    )
}

/// Returns an Actix [Route](actix_web::Route) that listens for a `GET` request and tries
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
/// fn MyApp(cx: Scope) -> impl IntoView {
///     view! { cx, <main>"Hello, world!"</main> }
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
///                     |cx| view! { cx, <MyApp/> },
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
    options: LeptosOptions,
    app_fn: impl Fn(leptos::Scope) -> IV + Clone + 'static,
    method: Method,
) -> Route
where
    IV: IntoView,
{
    render_app_async_with_context(options, |_cx| {}, app_fn, method)
}

/// Returns an Actix [Route](actix_web::Route) that listens for a `GET` request and tries
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
    options: LeptosOptions,
    additional_context: impl Fn(leptos::Scope) + 'static + Clone + Send,
    app_fn: impl Fn(leptos::Scope) -> IV + Clone + 'static,
    method: Method,
) -> Route
where
    IV: IntoView,
{
    render_app_to_stream_with_context_and_replace_blocks(
        options,
        additional_context,
        app_fn,
        method,
        false,
    )
}

/// Returns an Actix [Route](actix_web::Route) that listens for a `GET` request and tries
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
    options: LeptosOptions,
    additional_context: impl Fn(leptos::Scope) + 'static + Clone + Send,
    app_fn: impl Fn(leptos::Scope) -> IV + Clone + 'static,
    method: Method,
    replace_blocks: bool,
) -> Route
where
    IV: IntoView,
{
    let handler = move |req: HttpRequest| {
        let options = options.clone();
        let app_fn = app_fn.clone();
        let additional_context = additional_context.clone();
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

            stream_app(
                &options,
                app,
                res_options,
                additional_context,
                replace_blocks,
            )
            .await
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

/// Returns an Actix [Route](actix_web::Route) that listens for a `GET` request and tries
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
    options: LeptosOptions,
    additional_context: impl Fn(leptos::Scope) + 'static + Clone + Send,
    app_fn: impl Fn(leptos::Scope) -> IV + Clone + 'static,
    method: Method,
) -> Route
where
    IV: IntoView,
{
    let handler = move |req: HttpRequest| {
        let options = options.clone();
        let app_fn = app_fn.clone();
        let additional_context = additional_context.clone();
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

            stream_app_in_order(&options, app, res_options, additional_context)
                .await
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

/// Returns an Actix [Route](actix_web::Route) that listens for a `GET` request and tries
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
    options: LeptosOptions,
    additional_context: impl Fn(leptos::Scope) + 'static + Clone + Send,
    app_fn: impl Fn(leptos::Scope) -> IV + Clone + 'static,
    method: Method,
) -> Route
where
    IV: IntoView,
{
    let handler = move |req: HttpRequest| {
        let options = options.clone();
        let app_fn = app_fn.clone();
        let additional_context = additional_context.clone();
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

            render_app_async_helper(
                &options,
                app,
                res_options,
                additional_context,
            )
            .await
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

#[tracing::instrument(level = "trace", fields(error), skip_all)]
fn provide_contexts(
    cx: leptos::Scope,
    req: &HttpRequest,
    res_options: ResponseOptions,
) {
    let path = leptos_corrected_path(req);

    let integration = ServerIntegration { path };
    provide_context(cx, RouterIntegrationContext::new(integration));
    provide_context(cx, MetaContext::new());
    provide_context(cx, res_options);
    provide_context(cx, req.clone());
    provide_server_redirect(cx, move |path| redirect(cx, path));
    #[cfg(feature = "nonce")]
    leptos::nonce::provide_nonce(cx);
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
#[tracing::instrument(level = "trace", fields(error), skip_all)]
async fn stream_app(
    options: &LeptosOptions,
    app: impl FnOnce(leptos::Scope) -> View + 'static,
    res_options: ResponseOptions,
    additional_context: impl Fn(leptos::Scope) + 'static + Clone + Send,
    replace_blocks: bool,
) -> HttpResponse<BoxBody> {
    let (stream, runtime, scope) =
        render_to_stream_with_prefix_undisposed_with_context_and_block_replacement(
            app,
            move |cx| generate_head_metadata_separated(cx).1.into(),
            additional_context,
            replace_blocks
        );

    build_stream_response(options, res_options, stream, runtime, scope).await
}
#[cfg_attr(
    any(debug_assertions, feature = "ssr"),
    instrument(level = "trace", skip_all,)
)]
async fn stream_app_in_order(
    options: &LeptosOptions,
    app: impl FnOnce(leptos::Scope) -> View + 'static,
    res_options: ResponseOptions,
    additional_context: impl Fn(leptos::Scope) + 'static + Clone + Send,
) -> HttpResponse<BoxBody> {
    let (stream, runtime, scope) =
        leptos::ssr::render_to_stream_in_order_with_prefix_undisposed_with_context(
            app,
            move |cx| {
                generate_head_metadata_separated(cx).1.into()
            },
            additional_context,
        );

    build_stream_response(options, res_options, stream, runtime, scope).await
}
#[tracing::instrument(level = "trace", fields(error), skip_all)]
async fn build_stream_response(
    options: &LeptosOptions,
    res_options: ResponseOptions,
    stream: impl Stream<Item = String> + 'static,
    runtime: RuntimeId,
    scope: ScopeId,
) -> HttpResponse {
    let cx = leptos::Scope { runtime, id: scope };
    let mut stream = Box::pin(stream);

    // wait for any blocking resources to load before pulling metadata
    let first_app_chunk = stream.next().await.unwrap_or_default();

    let (head, tail) = html_parts_separated(
        cx,
        options,
        use_context::<MetaContext>(cx).as_ref(),
    );

    let mut stream = Box::pin(
        futures::stream::once(async move { head.clone() })
            .chain(
                futures::stream::once(async move { first_app_chunk })
                    .chain(stream),
            )
            .chain(futures::stream::once(async move {
                runtime.dispose();
                tail.to_string()
            }))
            .map(|html| Ok(web::Bytes::from(html)) as Result<web::Bytes>),
    );

    // Get the first and second in the stream, which renders the app shell, and thus allows Resources to run
    let first_chunk = stream.next().await;
    let second_chunk = stream.next().await;

    let res_options = res_options.0.read();

    let (status, headers) = (res_options.status, res_options.headers.clone());
    let status = status.unwrap_or_default();

    let complete_stream =
        futures::stream::iter([first_chunk.unwrap(), second_chunk.unwrap()])
            .chain(stream);
    let mut res = HttpResponse::Ok()
        .content_type("text/html")
        .streaming(complete_stream);

    // Add headers manipulated in the response
    for (key, value) in headers.into_iter() {
        res.headers_mut().append(key, value);
    }

    // Set status to what is returned in the function
    let res_status = res.status_mut();
    *res_status = status;
    // Return the response
    res
}
#[tracing::instrument(level = "trace", fields(error), skip_all)]
async fn render_app_async_helper(
    options: &LeptosOptions,
    app: impl FnOnce(leptos::Scope) -> View + 'static,
    res_options: ResponseOptions,
    additional_context: impl Fn(leptos::Scope) + 'static + Clone + Send,
) -> HttpResponse<BoxBody> {
    let (stream, runtime, scope) =
        leptos::ssr::render_to_stream_in_order_with_prefix_undisposed_with_context(
            app,
            move |_| "".into(),
            additional_context,
        );

    let html = build_async_response(stream, options, runtime, scope).await;

    let res_options = res_options.0.read();

    let (status, headers) = (res_options.status, res_options.headers.clone());
    let status = status.unwrap_or_default();

    let mut res = HttpResponse::Ok().content_type("text/html").body(html);

    // Add headers manipulated in the response
    for (key, value) in headers.into_iter() {
        res.headers_mut().append(key, value);
    }

    // Set status to what is returned in the function
    let res_status = res.status_mut();
    *res_status = status;
    // Return the response
    res
}

/// Generates a list of all routes defined in Leptos's Router in your app. We can then use this to automatically
/// create routes in Actix's App without having to use wildcard matching or fallbacks. Takes in your root app Element
/// as an argument so it can walk you app tree. This version is tailored to generated Actix compatible paths.
pub fn generate_route_list<IV>(
    app_fn: impl FnOnce(leptos::Scope) -> IV + 'static,
) -> Vec<RouteListing>
where
    IV: IntoView + 'static,
{
    generate_route_list_with_exclusions(app_fn, None)
}

/// Generates a list of all routes defined in Leptos's Router in your app. We can then use this to automatically
/// create routes in Actix's App without having to use wildcard matching or fallbacks. Takes in your root app Element
/// as an argument so it can walk you app tree. This version is tailored to generated Actix compatible paths. Adding excluded_routes
/// to this function will stop `.leptos_routes()` from generating a route for it, allowing a custom handler. These need to be in Actix path format
pub fn generate_route_list_with_exclusions<IV>(
    app_fn: impl FnOnce(leptos::Scope) -> IV + 'static,
    excluded_routes: Option<Vec<String>>,
) -> Vec<RouteListing>
where
    IV: IntoView + 'static,
{
    let mut routes = leptos_router::generate_route_list_inner(app_fn);

    // Actix's Router doesn't follow Leptos's
    // Match `*` or `*someword` to replace with replace it with "/{tail.*}
    let wildcard_re = Regex::new(r"\*.*").unwrap();
    // Match `:some_word` but only capture `some_word` in the groups to replace with `{some_word}`
    let capture_re = Regex::new(r":((?:[^.,/]+)+)[^/]?").unwrap();

    // Empty strings screw with Actix pathing, they need to be "/"
    routes = routes
        .into_iter()
        .map(|listing| {
            let path = listing.path();
            if path.is_empty() {
                return RouteListing::new(
                    "/".to_string(),
                    listing.mode(),
                    listing.methods(),
                );
            }
            RouteListing::new(listing.path(), listing.mode(), listing.methods())
        })
        .map(|listing| {
            let path = wildcard_re
                .replace_all(listing.path(), "{tail:.*}")
                .to_string();
            let path = capture_re.replace_all(&path, "{$1}").to_string();
            RouteListing::new(path, listing.mode(), listing.methods())
        })
        .collect::<Vec<_>>();

    if routes.is_empty() {
        vec![RouteListing::new("/", Default::default(), [Method::Get])]
    } else {
        // Routes to exclude from auto generation
        if let Some(excluded_routes) = excluded_routes {
            routes.retain(|p| !excluded_routes.iter().any(|e| e == p.path()))
        }
        routes
    }
}

pub enum DataResponse<T> {
    Data(T),
    Response(actix_web::dev::Response<BoxBody>),
}

/// This trait allows one to pass a list of routes and a render function to Actix's router, letting us avoid
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
            let mode = listing.mode();

            for method in listing.methods() {
                router = router.route(
                    path,
                    match mode {
                        SsrMode::OutOfOrder => {
                            render_app_to_stream_with_context(
                                options.clone(),
                                additional_context.clone(),
                                app_fn.clone(),
                                method,
                            )
                        }
                        SsrMode::PartiallyBlocked => {
                            render_app_to_stream_with_context_and_replace_blocks(
                                options.clone(),
                                additional_context.clone(),
                                app_fn.clone(),
                                method,
                                true,
                            )
                        }
                        SsrMode::InOrder => {
                            render_app_to_stream_in_order_with_context(
                                options.clone(),
                                additional_context.clone(),
                                app_fn.clone(),
                                method,
                            )
                        }
                        SsrMode::Async => render_app_async_with_context(
                            options.clone(),
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

/// A helper to make it easier to use Actix extractors in server functions. This takes
/// a handler function as its argument. The handler follows similar rules to an Actix
/// [Handler](actix_web::Handler): it is an async function that receives arguments that
/// will be extracted from the request and returns some value.
///
/// ```rust,ignore
/// use leptos::*;
/// use serde::Deserialize;
/// #[derive(Deserialize)]
/// struct Search {
///     q: String,
/// }
///
/// #[server(ExtractoServerFn, "/api")]
/// pub async fn extractor_server_fn(cx: Scope) -> Result<String, ServerFnError> {
///     use actix_web::dev::ConnectionInfo;
///     use actix_web::web::{Data, Query};
///
///     extract(
///         cx,
///         |data: Data<String>, search: Query<Search>, connection: ConnectionInfo| async move {
///             format!(
///                 "data = {}\nsearch = {}\nconnection = {:?}",
///                 data.into_inner(),
///                 search.q,
///                 connection
///             )
///         },
///     )
///     .await
/// }
/// ```
pub async fn extract<F, E>(
    cx: leptos::Scope,
    f: F,
) -> Result<<<F as Extractor<E>>::Future as Future>::Output, ServerFnError>
where
    F: Extractor<E>,
    E: actix_web::FromRequest,
    <E as actix_web::FromRequest>::Error: Display,
    <F as Extractor<E>>::Future: Future,
{
    let req = use_context::<actix_web::HttpRequest>(cx)
        .expect("HttpRequest should have been provided via context");

    let input = if let Some(body) = use_context::<Bytes>(cx) {
        let (_, mut payload) = actix_http::h1::Payload::create(false);
        payload.unread_data(body);
        E::from_request(&req, &mut dev::Payload::from(payload))
    } else {
        E::extract(&req)
    }
    .await
    .map_err(|e| ServerFnError::ServerError(e.to_string()))?;

    Ok(f.call(input).await)
}

// Drawn from the Actix Handler implementation
// https://github.com/actix/actix-web/blob/19c9d858f25e8262e14546f430d713addb397e96/actix-web/src/handler.rs#L124
pub trait Extractor<T> {
    type Future;

    fn call(self, args: T) -> Self::Future;
}
macro_rules! factory_tuple ({ $($param:ident)* } => {
    impl<Func, Fut, $($param,)*> Extractor<($($param,)*)> for Func
    where
        Func: FnOnce($($param),*) -> Fut + Clone + 'static,
        Fut: Future,
    {
        type Future = Fut;

        #[inline]
        #[allow(non_snake_case)]
        fn call(self, ($($param,)*): ($($param,)*)) -> Self::Future {
            (self)($($param,)*)
        }
    }
});

factory_tuple! {}
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
