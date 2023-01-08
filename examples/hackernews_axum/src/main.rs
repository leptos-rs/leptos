use cfg_if::cfg_if;
use leptos::*;

// boilerplate to run in different modes
cfg_if! {
if #[cfg(feature = "ssr")] {
    use axum::{
        Router,
        extract::Extension,
        error_handling::HandleError,
    };
    use http::StatusCode;
    use tower_http::services::ServeDir;
    use leptos_axum::{generate_route_list, LeptosRoutes};

    #[tokio::main]
    async fn main() {
        use hackernews_axum::*;

        let conf = get_configuration(Some("Cargo.toml")).await.unwrap();
        let leptos_options = conf.leptos_options;
        let addr = leptos_options.site_address.clone();
        let routes = generate_route_list(|cx| view! { cx, <App/> }).await;

        simple_logger::init_with_level(log::Level::Debug).expect("couldn't initialize logging");

        // build our application with a route
        let app = Router::new()
        .leptos_routes(leptos_options.clone(), routes, |cx| view! { cx, <App/> } )
        .route("/*file_path", get(file_handler))
        .layer(Extension(Arc::new(leptos_options)));

        // run our app with hyper
        // `axum::Server` is a re-export of `hyper::Server`
        log!("listening on {}", addr);
        axum::Server::bind(&addr)
            .serve(app.into_make_service())
            .await
            .unwrap();
    }
}

    // client-only stuff for Trunk
    else {
        use hackernews_axum::*;

        pub fn main() {
            console_error_panic_hook::set_once();
            _ = console_log::init_with_level(log::Level::Debug);
            console_error_panic_hook::set_once();
            mount_to_body(|cx| {
                view! { cx, <App/> }
            });
        }
    }
}
