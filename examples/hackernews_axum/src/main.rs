#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use axum::{routing::get, Router};
    use hackernews_axum::{shell, App};
    use leptos::config::get_configuration;
    use leptos_axum::{generate_route_list, LeptosRoutes};

    let conf = get_configuration(Some("Cargo.toml")).unwrap();
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;
    let routes = generate_route_list(App);

    // build our application with a route
    let app = Router::new()
        .route(
            "/favicon.ico",
            get(|| async {
                (
                    [("content-type", "image/x-icon")],
                    include_bytes!("../public/favicon.ico"),
                )
            }),
        )
        .leptos_routes(&leptos_options, routes, {
            let leptos_options = leptos_options.clone();
            move || shell(leptos_options.clone())
        })
        .fallback(leptos_axum::file_and_error_handler(shell))
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
    use hackernews_axum::*;

    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();
    mount_to_body(App);
}
