#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use axum::Router;
    use axum::routing::post;
    // In production you wouldn't want to use a hardcoded address like this.
    let addr = "127.0.0.1:3003";
    // build our application with a route
    let app = Router::new()
        .route("/api_shared2/*fn_name", post(leptos_axum::handle_server_fns))
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .layer(axum::Extension(shared_server_2::SharedServerState2));

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    println!("shared server listening on http://{}", addr);
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
    // no client-side main function
    // our server is SSR only, we have no client pair.
    // We'll only ever run this with cargo run --features ssr
}
