//! Server entry point for the automatic mode detection example

use automatic_mode_detection::App;
use leptos::*;
use leptos_axum::{generate_route_list, LeptosRoutes};
use leptos_compile_validator::validate_with_context;

#[tokio::main]
async fn main() {
    // Perform validation at startup
    let _validation = validate_with_context();
    
    // Get the leptos configuration
    let conf = get_configuration(None).await.unwrap();
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;
    let routes = generate_route_list(App);

    // Build the application with routes
    let app = Router::new()
        .leptos_routes(&leptos_options, routes, App)
        .fallback_file_and_image(&leptos_options.site_root, &leptos_options.site_pkg_dir, None)
        .with_state(leptos_options);

    // Start the server
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    println!("listening on http://{}", &addr);
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}
