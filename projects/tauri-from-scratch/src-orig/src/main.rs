#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use axum::{
        body::Body,
        extract::{Request, State},
        response::IntoResponse,
        routing::get,
        Router,
    };
    use leptos::logging::log;
    use leptos::prelude::*;
    use leptos_axum::{generate_route_list, LeptosRoutes};
    use leptos_tauri_from_scratch::{
        app::{shell, App},
        fallback::file_and_error_handler,
    };
    use tower_http::cors::CorsLayer;

    let conf = get_configuration(None).unwrap();
    let addr = conf.leptos_options.site_addr;
    let leptos_options = conf.leptos_options;
    // Generate the list of routes in your Leptos App
    let routes = generate_route_list(App);

    #[derive(Clone, Debug, axum_macros::FromRef)]
    pub struct ServerState {
        pub options: LeptosOptions,
        pub routes: Vec<leptos_axum::AxumRouteListing>,
    }

    let state = ServerState {
        options: leptos_options,
        routes: routes.clone(),
    };

    pub async fn server_fn_handler(
        State(state): State<ServerState>,
        request: Request<Body>,
    ) -> impl IntoResponse {
        leptos_axum::handle_server_fns_with_context(
            move || {
                provide_context(state.clone());
            },
            request,
        )
        .await
        .into_response()
    }

    let cors = CorsLayer::new()
        .allow_methods([axum::http::Method::GET, axum::http::Method::POST])
        .allow_origin(
            "tauri://localhost"
                .parse::<axum::http::HeaderValue>()
                .unwrap(),
        )
        .allow_headers(vec![axum::http::header::CONTENT_TYPE]);

    pub async fn leptos_routes_handler(
        State(state): State<ServerState>,
        req: Request<Body>,
    ) -> axum::response::Response {
        let leptos_options = state.options.clone();
        let handler = leptos_axum::render_route_with_context(
            state.routes.clone(),
            move || {
                provide_context("...");
            },
            move || shell(leptos_options.clone()),
        );
        handler(axum::extract::State(state), req)
            .await
            .into_response()
    }

    let app = Router::new()
        .route(
            "/api/{*fn_name}",
            get(server_fn_handler).post(server_fn_handler),
        )
        .layer(cors)
        .leptos_routes_with_handler(routes, get(leptos_routes_handler))
        .fallback(file_and_error_handler)
        .with_state(state);

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    log!("listening on http://{}", &addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

#[cfg(feature = "csr")]
pub fn main() {
    server_fn::client::set_server_url("http://127.0.0.1:3000");
    leptos::mount::mount_to_body(leptos_tauri_from_scratch::app::App);
}
