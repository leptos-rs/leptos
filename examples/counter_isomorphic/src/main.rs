mod counters;

use crate::counters::*;
use actix_files::Files;
use actix_web::*;
use leptos::*;
use leptos_actix::{generate_route_list, LeptosRoutes};

#[get("/api/events")]
async fn counter_events() -> impl Responder {
    use crate::counters::ssr_imports::*;
    use futures::StreamExt;

    let stream = futures::stream::once(async {
        crate::counters::get_server_count().await.unwrap_or(0)
    })
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
    // Setting this to None means we'll be using cargo-leptos and its env vars.
    // when not using cargo-leptos None must be replaced with Some("Cargo.toml")
    let conf = get_configuration(None).await.unwrap();

    let addr = conf.leptos_options.site_addr;
    let routes = generate_route_list(Counters);

    HttpServer::new(move || {
        let leptos_options = &conf.leptos_options;
        let site_root = &leptos_options.site_root;

        App::new()
            .service(counter_events)
            .leptos_routes(
                leptos_options.to_owned(),
                routes.to_owned(),
                Counters,
            )
            .service(Files::new("/", site_root))
        //.wrap(middleware::Compress::default())
    })
    .bind(&addr)?
    .run()
    .await
}
