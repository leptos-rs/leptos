use cfg_if::cfg_if;
use leptos::*;
mod counters;

// boilerplate to run in different modes
cfg_if! {
    // server-only stuff
    if #[cfg(feature = "ssr")] {
        use actix_files::{Files};
        use actix_web::*;
        use crate::counters::*;
        use std::{net::SocketAddr, env};

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
            let addr = SocketAddr::from(([127,0,0,1],3000));
            crate::counters::register_server_functions();

            HttpServer::new(move || {
                let render_options: RenderOptions = RenderOptions::builder().pkg_path("/pkg/leptos_counter_isomorphic").reload_port(3001).socket_address(addr.clone()).environment(&env::var("RUST_ENV")).build();
                render_options.write_to_file();
                App::new()
                    .service(Files::new("/pkg", "./pkg"))
                    .service(counter_events)
                    .route("/api/{tail:.*}", leptos_actix::handle_server_fns())
                    .route("/{tail:.*}", leptos_actix::render_app_to_stream(render_options, |cx| view! { cx, <Counters/> }))
                //.wrap(middleware::Compress::default())
            })
            .bind(&addr)?
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
