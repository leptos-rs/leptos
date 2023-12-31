#[cfg(feature = "ssr")]
#[allow(unused)]
mod ssr_imports {
    pub use axum::{
        body::Body as AxumBody,
        extract::{Path, State},
        http::Request,
        response::{Html, IntoResponse, Response},
        routing::{get, post},
        Router,
    };
    pub use leptos::*;
    pub use leptos_axum::{generate_route_list, LeptosRoutes};
    pub use todo_app_sqlite_csr::{
        fallback::file_or_index_handler, todo::*, *,
    };
}

#[cfg(feature = "ssr")]
#[cfg_attr(feature = "ssr", tokio::main)]
async fn main() {
    use ssr_imports::*;
    simple_logger::init_with_level(log::Level::Error)
        .expect("couldn't initialize logging");

    let _conn = db().await.expect("couldn't connect to DB");

    // Setting this to None means we'll be using cargo-leptos and its env vars
    let conf = get_configuration(None).await.unwrap();
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;

    // build our application with a route
    let app = Router::new()
        .route("/api/*fn_name", post(leptos_axum::handle_server_fns))
        .fallback(file_or_index_handler)
        .with_state(leptos_options);

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    logging::log!("listening on http://{}", &addr);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("couldn't bind to address");
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
    // This example cannot be built as a trunk standalone CSR-only app.
    // Only the server may directly connect to the database.
}
