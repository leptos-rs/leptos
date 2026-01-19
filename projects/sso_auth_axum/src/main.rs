use crate::ssr_imports::*;
use axum::{
    body::Body as AxumBody,
    extract::{Path, State},
    http::Request,
    response::IntoResponse,
    routing::get,
    Router,
};
use axum_session::{Key, SessionConfig, SessionLayer, SessionStore};
use axum_session_auth::{AuthConfig, AuthSessionLayer};
use leptos::prelude::*;
use leptos_axum::{
    file_and_error_handler, generate_route_list,
    handle_server_fns_with_context, LeptosRoutes,
};
use leptos_meta::{HashedStylesheet, MetaTags};
use sqlx::sqlite::SqlitePoolOptions;
use sso_auth_axum::{auth::*, state::AppState};

async fn server_fn_handler(
    State(app_state): State<AppState>,
    auth_session: AuthSession,
    path: Path<String>,
    request: Request<AxumBody>,
) -> impl IntoResponse {
    leptos::logging::log!("{:?}", path);

    handle_server_fns_with_context(
        move || {
            provide_context(app_state.clone());
            provide_context(auth_session.clone());
            provide_context(app_state.pool.clone());
        },
        request,
    )
    .await
}

pub async fn leptos_routes_handler(
    auth_session: AuthSession,
    State(app_state): State<AppState>,
    request: Request<AxumBody>,
) -> axum::response::Response {
    let handler = leptos_axum::render_app_async_with_context(
        move || {
            provide_context(app_state.clone());
            provide_context(auth_session.clone());
            provide_context(app_state.pool.clone());
        },
        move || view! {  <sso_auth_axum::App/> },
    );

    handler(request).await.into_response()
}

#[tokio::main]
async fn main() {
    simple_logger::init_with_level(log::Level::Info)
        .expect("couldn't initialize logging");

    let pool = SqlitePoolOptions::new()
        .connect("sqlite:sso.db")
        .await
        .expect("Could not make pool.");

    // Auth section
    let session_config = SessionConfig::default()
        .with_table_name("sessions_table")
        .with_key(Key::generate())
        .with_database_key(Key::generate());
    // .with_security_mode(SecurityMode::PerSession); // FIXME did this disappear?

    let auth_config = AuthConfig::<i64>::default();
    let session_store = SessionStore::<SessionSqlitePool>::new(
        Some(pool.clone().into()),
        session_config,
    )
    .await
    .unwrap();

    sqlx::migrate!()
        .run(&pool)
        .await
        .expect("could not run SQLx migrations");

    // Setting this to None means we'll be using cargo-leptos and its env vars
    let conf = get_configuration(None).unwrap();
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;
    let routes = generate_route_list(sso_auth_axum::App);

    // We create our client using provided environment variables.
    let client = oauth2::basic::BasicClient::new(
        oauth2::ClientId::new(
            std::env::var("G_AUTH_CLIENT_ID")
                .expect("G_AUTH_CLIENT_ID Env var to be set."),
        ),
        Some(oauth2::ClientSecret::new(
            std::env::var("G_AUTH_SECRET")
                .expect("G_AUTH_SECRET Env var to be set"),
        )),
        oauth2::AuthUrl::new(
            "https://accounts.google.com/o/oauth2/v2/auth".to_string(),
        )
        .unwrap(),
        Some(
            oauth2::TokenUrl::new(
                "https://oauth2.googleapis.com/token".to_string(),
            )
            .unwrap(),
        ),
    )
    .set_redirect_uri(
        oauth2::RedirectUrl::new(
            std::env::var("REDIRECT_URL")
                .expect("REDIRECT_URL Env var to be set"),
        )
        .unwrap(),
    );

    let app_state = AppState {
        leptos_options,
        pool: pool.clone(),
        client,
    };

    // build our application with a route
    let app = Router::new()
        .route(
            "/api/*fn_name",
            get(server_fn_handler).post(server_fn_handler),
        )
        .leptos_routes_with_handler(routes, get(leptos_routes_handler))
        .fallback(file_and_error_handler::<AppState, _>(shell))
        .layer(
            AuthSessionLayer::<User, i64, SessionSqlitePool, SqlitePool>::new(
                Some(pool.clone()),
            )
            .with_config(auth_config),
        )
        .layer(SessionLayer::new(session_store))
        .with_state(app_state);

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    leptos::logging::log!("listening on http://{}", &addr);
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />

                <AutoReload options=options.clone() />
                <HydrationScripts options=options.clone() />
                <MetaTags />

                <link rel="preconnect" href="/fonts/inter" />
                <link rel="stylesheet" href="/fonts/inter/inter.css" />
                <link rel="preconnect" href="/fonts/jetbrains-mono" />
                <link rel="stylesheet" href="/fonts/jetbrains-mono/jetbrains-mono.css" />

                <HashedStylesheet options />

                <title>"SSO Auth Axum"</title>
            </head>

            <body>
                <sso_auth_axum::App />
            </body>
        </html>
    }
    .into_any()
}
