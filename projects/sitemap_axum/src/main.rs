#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use axum::{routing::get, Router};
    use leptos::*;
    use leptos_axum::{generate_route_list, LeptosRoutes};
    use sitemap_axum::{
        app::*, fileserv::file_and_error_handler, sitemap::generate_sitemap,
    };
    use tower_http::services::ServeFile;

    // Setting get_configuration(None) means we'll be using cargo-leptos's env values
    // For deployment these variables are:
    // <https://github.com/leptos-rs/start-axum#executing-a-server-on-a-remote-machine-without-the-toolchain>
    // Alternately a file can be specified such as Some("Cargo.toml")
    // The file would need to be included with the executable when moved to deployment
    let conf = get_configuration(None).unwrap();
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;
    let routes = generate_route_list(App);

    // Build our application with a route
    let app = Router::new()
        // We can use Axum to mount a route that serves a sitemap file that we can generate with dynamic data
        .route("/sitemap-index.xml", get(generate_sitemap))
        // Using tower's serve file service, we can also serve a static sitemap file for relatively small sites too
        .route_service(
            "/sitemap-static.xml",
            ServeFile::new("sitemap-static.xml"),
        )
        .leptos_routes(&leptos_options, routes, App)
        .fallback(file_and_error_handler)
        .with_state(leptos_options);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    logging::log!("listening on http://{}", &addr);
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
    // no client-side main function
    // unless we want this to work with e.g., Trunk for a purely client-side app
    // see lib.rs for hydration function instead
}
