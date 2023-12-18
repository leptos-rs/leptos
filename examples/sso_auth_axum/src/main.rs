use cfg_if::cfg_if;

// boilerplate to run in different modes
cfg_if! {
if #[cfg(feature = "ssr")] {
    use axum::{
        response::{IntoResponse},
        routing::get,
        extract::{Path, State, RawQuery},
        http::{Request, header::HeaderMap},
        body::Body as AxumBody,
        Router,
    };
    use sso_auth_axum::auth::*;
    use sso_auth_axum::state::AppState;
    use sso_auth_axum::fallback::file_and_error_handler;
    use leptos_axum::{generate_route_list, handle_server_fns_with_context, LeptosRoutes};
    use leptos::{logging::log, view, provide_context, get_configuration};
    use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};
    use axum_session::{SessionConfig, SessionLayer, SessionStore,Key, SecurityMode};
    use axum_session_auth::{AuthSessionLayer, AuthConfig, SessionSqlitePool};

    async fn server_fn_handler(State(app_state): State<AppState>, auth_session: AuthSession, path: Path<String>, headers: HeaderMap, raw_query: RawQuery,
    request: Request<AxumBody>) -> impl IntoResponse {

        log!("{:?}", path);

        handle_server_fns_with_context(path, headers, raw_query, move || {
            provide_context(app_state.clone());
            provide_context(auth_session.clone());
            provide_context(app_state.pool.clone());
        }, request).await
    }

    pub async fn leptos_routes_handler(
        auth_session: AuthSession,
        State(app_state): State<AppState>,
        axum::extract::State(option): axum::extract::State<leptos::LeptosOptions>,
        request: Request<AxumBody>,
    ) -> axum::response::Response {
        let handler = leptos_axum::render_app_async_with_context(
            option.clone(),
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
        simple_logger::init_with_level(log::Level::Info).expect("couldn't initialize logging");

        let pool = SqlitePoolOptions::new()
            .connect("sqlite:sso.db")
            .await
            .expect("Could not make pool.");

        // Auth section
        let session_config = SessionConfig::default()
            .with_table_name("sessions_table")
            .with_key(Key::generate())
            .with_database_key(Key::generate())
            .with_security_mode(SecurityMode::PerSession);

        let auth_config = AuthConfig::<i64>::default();
        let session_store = SessionStore::<SessionSqlitePool>::new(Some(pool.clone().into()), session_config);
        session_store.initiate().await.unwrap();

        sqlx::migrate!()
            .run(&pool)
            .await
            .expect("could not run SQLx migrations");



        // Setting this to None means we'll be using cargo-leptos and its env vars
        let conf = get_configuration(None).await.unwrap();
        let leptos_options = conf.leptos_options;
        let addr = leptos_options.site_addr;
        let routes = generate_route_list(sso_auth_axum::App);

            // We create our client using provided environment variables.
    let client = oauth2::basic::BasicClient::new(
        oauth2::ClientId::new(std::env::var("G_AUTH_CLIENT_ID").expect("G_AUTH_CLIENT Env var to be set.")),
        Some(oauth2::ClientSecret::new(std::env::var("G_AUTH_SECRET").expect("G_AUTH_SECRET Env var to be set"))),
        oauth2::AuthUrl::new(
            "https://accounts.google.com/o/oauth2/v2/auth".to_string(),
        )
        .unwrap(),
        Some(
            oauth2::TokenUrl::new("https://oauth2.googleapis.com/token".to_string())
                .unwrap(),
        ),
    )
    .set_redirect_uri(oauth2::RedirectUrl::new(std::env::var("REDIRECT_URL").expect("REDIRECT_URL Env var to be set")).unwrap());


        let app_state = AppState{
            leptos_options,
            pool: pool.clone(),
            client,
        };

        // build our application with a route
        let app = Router::new()
            .route("/api/*fn_name", get(server_fn_handler).post(server_fn_handler))
            .leptos_routes_with_handler(routes, get(leptos_routes_handler) )
            .fallback(file_and_error_handler)
            .layer(AuthSessionLayer::<User, i64, SessionSqlitePool, SqlitePool>::new(Some(pool.clone()))
            .with_config(auth_config))
            .layer(SessionLayer::new(session_store))
            .with_state(app_state);

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
