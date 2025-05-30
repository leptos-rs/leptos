use axum::Router;
use axum_session::{SessionConfig, SessionLayer, SessionStore};
use axum_session_auth::{AuthConfig, AuthSessionLayer};
use axum_session_sqlx::SessionSqlitePool;
use leptos::{config::get_configuration, logging::log};
use leptos_axum::{generate_route_list, LeptosRoutes};
use session_auth_axum::{auth::User, state::AppState, todo::*};
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};

#[tokio::main]
async fn main() {
    simple_logger::init_with_level(log::Level::Info)
        .expect("couldn't initialize logging");

    let pool = SqlitePoolOptions::new()
        .connect("sqlite:Todos.db")
        .await
        .expect("Could not make pool.");

    // Auth section
    let session_config =
        SessionConfig::default().with_table_name("axum_sessions");
    let auth_config = AuthConfig::<i64>::default();
    let session_store = SessionStore::<SessionSqlitePool>::new(
        Some(SessionSqlitePool::from(pool.clone())),
        session_config,
    )
    .await
    .unwrap();

    if let Err(e) = sqlx::migrate!().run(&pool).await {
        eprintln!("{e:?}");
    }

    // Setting this to None means we'll be using cargo-leptos and its env vars
    let conf = get_configuration(None).unwrap();
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;
    let routes = generate_route_list(TodoApp);

    let app_state = AppState {
        leptos_options,
        pool: pool.clone(),
        routes: routes.clone(),
    };

    // build our application with a route
    let app = Router::new()
        .leptos_routes(&app_state, routes, {
            let options = app_state.leptos_options.clone();
            move || shell(options.clone())
        })
        .fallback(leptos_axum::file_and_error_handler::<AppState, _>(shell))
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
    log!("listening on http://{}", &addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}
