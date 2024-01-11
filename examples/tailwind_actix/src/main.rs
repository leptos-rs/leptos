mod app;

use crate::app::*;
use actix_files::Files;
use actix_web::*;
use leptos::*;
use leptos_actix::{generate_route_list, LeptosRoutes};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Setting this to None means we'll be using cargo-leptos and its env vars.
    let conf = get_configuration(None).await.unwrap();

    let addr = conf.leptos_options.site_addr;

    // Generate the list of routes in your Leptos App
    let routes = generate_route_list(|| view! { <App/> });

    HttpServer::new(move || {
        let leptos_options = &conf.leptos_options;
        let site_root = &leptos_options.site_root;
        let routes = &routes;
        App::new()
            .leptos_routes(
                leptos_options.to_owned(),
                routes.to_owned(),
                || view! { <App/> },
            )
            .service(Files::new("/", site_root))
            .wrap(middleware::Compress::default())
    })
    .bind(&addr)?
    .run()
    .await
}
