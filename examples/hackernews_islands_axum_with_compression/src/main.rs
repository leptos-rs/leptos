#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    pub use axum::{routing::get, Router};
    pub use hackernews_islands::fallback::file_and_error_handler;
    use hackernews_islands::*;
    pub use leptos::get_configuration;
    pub use leptos_axum::{generate_route_list, LeptosRoutes};

    let conf = get_configuration(Some("Cargo.toml")).await.unwrap();
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;
    let routes = generate_route_list(App);

    // build our application with a route
    let app = Router::new()
        .route("/favicon.ico", get(file_and_error_handler))
        .leptos_routes(&leptos_options, routes, App)
        .fallback(file_and_error_handler)
        .with_state(leptos_options);

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    println!("listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

// client-only stuff for Trunk
#[cfg(not(feature = "ssr"))]
pub fn main() {
    use hackernews_islands::*;
    use leptos::*;
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();
    mount_to_body(App);
}
