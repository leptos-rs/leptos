use crate::{fallback::file_and_error_handler, todo::*};
use axum::{
    body::Body,
    extract::{Path, State},
    http::Request,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use leptos::*;
use leptos_axum::{generate_route_list, LeptosRoutes};
use todo_app_sqlite_axum::*;

//Define a handler to test extractor with state
async fn custom_handler(
    Path(id): Path<String>,
    State(options): State<LeptosOptions>,
    req: Request<Body>,
) -> Response {
    let handler = leptos_axum::render_app_to_stream_with_context(
        options,
        move || {
            provide_context(id.clone());
        },
        TodoApp,
    );
    handler(req).await.into_response()
}

#[tokio::main]
async fn main() {
    use crate::todo::ssr::db;

    simple_logger::init_with_level(log::Level::Error)
        .expect("couldn't initialize logging");

    let mut conn = db().await.expect("couldn't connect to DB");
    if let Err(e) = sqlx::migrate!().run(&mut conn).await {
        eprintln!("{e:?}");
    }

    // Setting this to None means we'll be using cargo-leptos and its env vars
    let conf = get_configuration(None).await.unwrap();
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;
    let routes = generate_route_list(TodoApp);

    // build our application with a route
    let app = Router::new()
        .route("/special/:id", get(custom_handler))
        .leptos_routes(&leptos_options, routes, || view! { <TodoApp/> })
        .fallback(file_and_error_handler)
        .with_state(leptos_options);

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    logging::log!("listening on http://{}", &addr);
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}
