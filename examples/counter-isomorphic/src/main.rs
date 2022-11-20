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
                    .route("/api/{tail:.*}", leptos_actix::handle_server_fns())
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
