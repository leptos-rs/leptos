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
    use crate::landing::*;
    use errors_axum::*;
    use crate::fallback::file_and_error_handler;
    use leptos_axum::{generate_route_list, LeptosRoutes};
    use std::sync::Arc;

    //Define a handler to test extractor with state
    async fn custom_handler(Path(id): Path<String>, Extension(options): Extension<Arc<LeptosOptions>>, req: Request<AxumBody>) -> Response{
            let handler = leptos_axum::render_app_to_stream_with_context((*options).clone(),
            move |cx| {
                provide_context(cx, id.clone());
            },
            |cx| view! { cx, <App/> }
        );
            handler(req).await.into_response()
    }

    #[tokio::main]
    async fn main() {
        simple_logger::init_with_level(log::Level::Debug).expect("couldn't initialize logging");

        crate::landing::register_server_functions();

        // Setting this to None means we'll be using cargo-leptos and its env vars
        let conf = get_configuration(None).await.unwrap();
        let leptos_options = conf.leptos_options;
        let addr = leptos_options.site_address.clone();
        let routes = generate_route_list(|cx| view! { cx, <App/> }).await;

        // build our application with a route
        let app = Router::new()
        .route("/api/*fn_name", post(leptos_axum::handle_server_fns))
        .route("/special/:id", get(custom_handler))
        .leptos_routes(leptos_options.clone(), routes, |cx| view! { cx, <App/> } )
        .fallback(file_and_error_handler)
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
        use todo_app_sqlite_axum::landing::*;

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
