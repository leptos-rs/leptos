#[cfg(feature = "ssr")]
mod ssr_imports {
    use axum::extract::State;
    pub use axum::{
        body::Body as AxumBody,
        extract::Path,
        http::Request,
        response::{IntoResponse, Response},
        routing::get,
        Router,
    };
    use errors_axum::landing::shell;
    pub use errors_axum::landing::App;
    use leptos::{config::LeptosOptions, context::provide_context};
    pub use leptos_axum::{generate_route_list, LeptosRoutes};

    // This custom handler lets us provide Axum State via context
    pub async fn custom_handler(
        Path(id): Path<String>,
        State(options): State<LeptosOptions>,
        req: Request<AxumBody>,
    ) -> Response {
        let handler = leptos_axum::render_app_to_stream_with_context(
            move || {
                provide_context(id.clone());
            },
            move || shell(options.clone()),
        );
        handler(req).await.into_response()
    }
}

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use errors_axum::landing::shell;
    use leptos::config::get_configuration;
    use ssr_imports::*;

    // Setting this to None means we'll be using cargo-leptos and its env vars
    let conf = get_configuration(None).unwrap();
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;
    let routes = generate_route_list(App);

    // build our application with a route
    let app = Router::new()
        .route("/special/{id}", get(custom_handler))
        .leptos_routes(&leptos_options, routes, {
            let leptos_options = leptos_options.clone();
            move || shell(leptos_options.clone())
        })
        .fallback(leptos_axum::file_and_error_handler(shell))
        .with_state(leptos_options);

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    println!("listening on http://{}", &addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

// this is if we were using client-only rending with Trunk
#[cfg(not(feature = "ssr"))]
pub fn main() {
    // This example cannot be built as a trunk standalone CSR-only app.
    // The server is needed to demonstrate the error statuses.
}
