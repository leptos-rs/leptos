use crate::{app::*, fallback::file_and_error_handler};
use axum::Router;
use leptos::{get_configuration, logging};
use leptos_axum::{generate_route_list, LeptosRoutes};
use server_fns_axum::*;

#[tokio::main]
async fn main() {
    simple_logger::init_with_level(log::Level::Error)
        .expect("couldn't initialize logging");

    // Setting this to None means we'll be using cargo-leptos and its env vars
    let conf = get_configuration(None).await.unwrap();
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;
    let routes = generate_route_list(TodoApp);

    // build our application with a route
    let app = Router::new()
        .leptos_routes(&leptos_options, routes, TodoApp)
        .fallback(file_and_error_handler)
        .with_state(leptos_options);

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    logging::log!("listening on http://{}", &addr);
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}
