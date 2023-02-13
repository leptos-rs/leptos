use cfg_if::cfg_if;
use leptos::*;
// boilerplate to run in different modes
cfg_if! {
if #[cfg(feature = "ssr")] {
    use axum::{
        routing::{post, get},
        extract::{Extension, Path},
        http::Request,
        response::{IntoResponse, Response},
        Router,
    };
    use axum::body::Body as AxumBody;
    use crate::app::*;
    use in_order_streaming_axum::*;
    use leptos_axum::*;
    use std::sync::Arc;

    #[tokio::main]
    async fn main() {
        simple_logger::init_with_level(log::Level::Debug).expect("couldn't initialize logging");

        // Setting this to None means we'll be using cargo-leptos and its env vars
        let conf = get_configuration(None).await.unwrap();
        let leptos_options = conf.leptos_options;
        let addr = leptos_options.site_addr;
        let routes = generate_route_list(|cx| view! { cx, <App/> }).await;

        // build our application with a route
        let app = Router::new()
            .route("/", get(render_app_to_stream_in_order(leptos_options.to_owned(), |cx| view! { cx, <App/> })))
            .route("/ooo", get(render_app_to_stream(leptos_options.to_owned(), |cx| view! { cx, <App/> })))
            .route("/async", get(render_app_async(leptos_options.to_owned(), |cx| view! { cx, <App/> })))
            //.leptos_async_routes(leptos_options.clone(), routes, |cx| view! { cx, <App/> } )
            .fallback(fileserv::file_and_error_handler)
            .layer(Extension(Arc::new(leptos_options)));

        // run our app with hyper
        // `axum::Server` is a re-export of `hyper::Server`
        log!("listening on http://{}", &addr);
        axum::Server::bind(&addr)
            .serve(app.into_make_service())
            .await
            .unwrap();
    }
}

    // client-only stuff for Trunk
    else {
        pub fn main() {
            // This example cannot be built as a trunk standalone CSR-only app.
            // Only the server may directly connect to the database.
        }
    }
}
