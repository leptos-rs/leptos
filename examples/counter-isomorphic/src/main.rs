use cfg_if::cfg_if;
use leptos::*;
use leptos_router::*;
mod counters;

// boilerplate to run in different modes
cfg_if! {
    // server-only stuff
    if #[cfg(feature = "ssr")] {
        use actix_files::{Files};
        use actix_web::*;
        use crate::counters::*;

        #[get("{tail:.*}")]
        async fn render(req: HttpRequest) -> impl Responder {
            let path = req.path();
            let path = "http://leptos".to_string() + path;
            println!("path = {path}");

            HttpResponse::Ok().content_type("text/html").body(format!(
                r#"<!DOCTYPE html>
                <html lang="en">
                    <head>
                        <meta charset="utf-8"/>
                        <meta name="viewport" content="width=device-width, initial-scale=1"/>
                        <title>Isomorphic Counter</title>
                    </head>
                    <body>
                        {}
                    </body>
                    <script type="module">import init, {{ hydrate }} from './pkg/leptos_counter_isomorphic.js'; init().then(hydrate);</script>
                </html>"#,
                run_scope({
                    move |cx| {
                        let integration = ServerIntegration { path: path.clone() };
                        provide_context(cx, RouterIntegrationContext::new(integration));

                        view! { cx, <Counters/>}
                    }
                })
            ))
        }

        #[post("/api/{tail:.*}")]
        async fn handle_server_fns(
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
                match server_fn(&body).await {
                    Ok(serialized) => {
                        // if this is Accept: application/json then send a serialized JSON response
                        if let Some("application/json") = accept_header {
                            HttpResponse::Ok().body(serialized)
                        }
                        // otherwise, it's probably a <form> submit or something: redirect back to the referrer
                        else {
                            HttpResponse::SeeOther()
                                .insert_header(("Location", "/"))
                                .content_type("application/json")
                                .body(serialized)
                        }
                    }
                    Err(e) => {
                        eprintln!("server function error: {e:#?}");
                        HttpResponse::InternalServerError().body(e.to_string())
                    }
                }
            } else {
                HttpResponse::BadRequest().body(format!("Could not find a server function at that route."))
            }
        }

        #[get("/api/events")]
        async fn counter_events() -> impl Responder {
            use futures::StreamExt;

            let stream =
                futures::stream::once(async { crate::counters::get_server_count().await.unwrap_or(0) })
                    .chain(COUNT_CHANNEL.clone())
                    .map(|value| {
                        Ok(web::Bytes::from(format!(
                            "event: message\ndata: {value}\n\n"
                        ))) as Result<web::Bytes>
                    });
            HttpResponse::Ok()
                .insert_header(("Content-Type", "text/event-stream"))
                .streaming(stream)
        }

        #[actix_web::main]
        async fn main() -> std::io::Result<()> {
            crate::counters::register_server_functions();

            HttpServer::new(|| {
                App::new()
                    .service(Files::new("/pkg", "./pkg"))
                    .service(counter_events)
                    .service(handle_server_fns)
                    .service(render)
                //.wrap(middleware::Compress::default())
            })
            .bind(("127.0.0.1", 8081))?
            .run()
            .await
        }
        }

    // client-only stuff for Trunk
    else {
        use leptos_counter_isomorphic::counters::*;

        pub fn main() {
            _ = console_log::init_with_level(log::Level::Debug);
            console_error_panic_hook::set_once();
            mount_to_body(|cx| view! { cx, <Counter/> });
        }
    }
}
