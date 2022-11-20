use actix_web::*;
use futures::StreamExt;
use leptos::*;
use leptos_meta::*;
use leptos_router::*;

/// An Actix [Route](actix_web::Route) that listens for a `POST` request with
/// Leptos server function arguments in the body, runs the server function if found,
/// and returns the resulting [HttpResponse].
///
/// The provides the [HttpRequest] to the server [Scope](leptos_reactive::Scope).
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
                    let (cx, disposer) = raw_scope_and_disposer();

                    // provide HttpRequest as context in server scope
                    provide_context(cx, req.clone());

                    match server_fn(cx, body).await {
                        Ok(serialized) => {
                            // clean up the scope, which we only needed to run the server fn
                            disposer.dispose();

                            // if this is Accept: application/json then send a serialized JSON response
                            if let Some("application/json") = accept_header {
                                HttpResponse::Ok().body(serialized)
                            }
                            // otherwise, it's probably a <form> submit or something: redirect back to the referrer
                            else {
                                let referer = req
                                    .headers()
                                    .get("Referer")
                                    .and_then(|value| value.to_str().ok())
                                    .unwrap_or("/");
                                HttpResponse::SeeOther()
                                    .insert_header(("Location", referer))
                                    .content_type("application/json")
                                    .body(serialized)
                            }
                        }
                        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
                    }
                } else {
                    HttpResponse::BadRequest()
                        .body(format!("Could not find a server function at that route."))
                }
            }
        },
    )
}

/// An Actix [Route](actix_web::Route) that listens for a `GET` request and tries
/// to route it using [leptos_router], serving an HTML stream of your application.
///
/// The provides a [MetaContext] and a [RouterIntegrationContext] to app’s context before
/// rendering it, and includes any meta tags injected using [leptos_meta].
///
/// The HTML stream is rendered using [render_to_stream], and also everything described in
/// the documentation for that function.
///
/// This can then be set up at an appropriate route in your application:
/// ```
/// use actix_web::{HttpServer, App};
/// use leptos::*;
///
/// #[component]
/// fn MyApp(cx: Scope) -> Element {
///   view! { cx, <main>"Hello, world!"</main> }
/// }
///
/// # if false { // don't actually try to run a server in a doctest...
/// #[actix_web::main]
/// async fn main() -> std::io::Result<()> {
///     HttpServer::new(|| {
///         App::new()
///             // {tail:.*} passes the remainder of the URL as the route
///             // the actual routing will be handled by `leptos_router`
///             .route("/{tail:.*}", leptos_actix::render_app_to_stream("leptos_example", |cx| view! { cx, <MyApp/> }))
///     })
///     .bind(("127.0.0.1", 8080))?
///     .run()
///     .await
/// }
/// # }
/// ```
pub fn render_app_to_stream(
    client_pkg_name: &'static str,
    app_fn: impl Fn(leptos::Scope) -> Element + Clone + 'static,
) -> Route {
    web::get().to(move |req: HttpRequest| {
        let app_fn = app_fn.clone();
        async move {
            let path = req.path();

            let query = req.query_string();
            let path = if query.is_empty() {
                "http://leptos".to_string() + path
            } else {
                "http://leptos".to_string() + path + "?" + query
            };

            let app = {
                let app_fn = app_fn.clone();
                move |cx| {
                    let integration = ServerIntegration { path: path.clone() };
                    provide_context(cx, RouterIntegrationContext::new(integration));
                    provide_context(cx, MetaContext::new());
                    provide_context(cx, req.clone());

                    (app_fn)(cx)
                }
            };

            let head = format!(r#"<!DOCTYPE html>
                <html>
                    <head>
                        <meta charset="utf-8"/>
                        <meta name="viewport" content="width=device-width, initial-scale=1"/>
                        <script type="module">import init, {{ hydrate }} from '/pkg/{client_pkg_name}.js'; init().then(hydrate);</script>"#);
            let tail = "</body></html>";

            HttpResponse::Ok().content_type("text/html").streaming(
                futures::stream::once(async move { head.clone() })
                    .chain(render_to_stream(move |cx| {
                        let app = app(cx);
                        let head = use_context::<MetaContext>(cx)
                            .map(|meta| meta.dehydrate())
                            .unwrap_or_default();
                        format!("{head}</head><body>{app}")
                    }))
                    .chain(futures::stream::once(async { tail.to_string() }))
                    .map(|html| Ok(web::Bytes::from(html)) as Result<web::Bytes>),
            )
        }
    })
}
