use actix_web::*;
use leptos::*;

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
    web::post().to(handle_server_fn)
}

async fn handle_server_fn(
    req: HttpRequest,
    params: web::Path<String>,
    body: web::Bytes,
) -> impl Responder {
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
        HttpResponse::BadRequest().body(format!("Could not find a server function at that route."))
    }
}
