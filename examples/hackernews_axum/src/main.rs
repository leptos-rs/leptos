use cfg_if::cfg_if;
use leptos::{logging::log, *};

// boilerplate to run in different modes
cfg_if! {
if #[cfg(feature = "ssr")] {
    use axum::{
        Router,
        routing::get,
    };
    use leptos_axum::{generate_route_list, LeptosRoutes};
    use hackernews_axum::fallback::file_and_error_handler;

    #[tokio::main]
    async fn main() {
        use hackernews_axum::*;

        let conf = get_configuration(Some("Cargo.toml")).await.unwrap();
        let leptos_options = conf.leptos_options;
        let addr = leptos_options.site_addr;
        let routes = generate_route_list(App);

        simple_logger::init_with_level(log::Level::Debug).expect("couldn't initialize logging");

        // build our application with a route
        let app = Router::new()
        .route("/favicon.ico", get(file_and_error_handler))
        .leptos_routes(&leptos_options, routes, || view! {  <App/> } )
        .fallback(file_and_error_handler)
        .with_state(leptos_options);

        // run our app with hyper
        // `axum::Server` is a re-export of `hyper::Server`
        log!("listening on {}", addr);
        let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
        axum::serve(listener, app.into_make_service())
            .await
            .unwrap();
    }
}

    // client-only stuff for Trunk
    else {
        use hackernews_axum::*;

        pub fn main() {
            _ = console_log::init_with_level(log::Level::Debug);
            console_error_panic_hook::set_once();
            mount_to_body(|| {
                view! {  <App/> }
            });
        }
    }
}
