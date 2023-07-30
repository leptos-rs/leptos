#[cfg(feature = "ssr")]
mod ssr_imports {
    pub use axum::Router;
    pub use leptos::*;
    pub use leptos_axum::{generate_route_list, LeptosRoutes};
    pub use tower_http::{compression::CompressionLayer, services::ServeFile};
}

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use hackernews::*;
    use ssr_imports::*;

    let conf = get_configuration(Some("Cargo.toml")).await.unwrap();
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;
    let routes = generate_route_list(|| view! {  <App/> }).await;

    // build our application with a route
    let app = Router::new()
        .route_service("/favicon.ico", ServeFile::new("./public/favicon.ico"))
        .route_service(
            "/style.css",
            ServeFile::new("./pkg/style.css").precompressed_br(),
        )
        .route_service(
            "/pkg/hackernews.js",
            ServeFile::new("./pkg/hackernews.js").precompressed_br(),
        )
        .route_service(
            "/pkg/hackernews_bg.wasm",
            ServeFile::new("./pkg/hackernews_bg.wasm").precompressed_br(),
        )
        .leptos_routes(&leptos_options, routes, App)
        //.layer(CompressionLayer::new())
        .with_state(leptos_options);

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    log!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
