use cfg_if::cfg_if;

// boilerplate to run in different modes
cfg_if! {
if #[cfg(feature = "ssr")] {
    use axum::{
        response::{Response, IntoResponse},
        routing::get,
        extract::{Path, State, RawQuery},
        http::{Request, header::HeaderMap},
        body::Body as AxumBody,
        Router,
    };
    use session_auth_axum::todo::*;
    use session_auth_axum::auth::*;
    use session_auth_axum::state::AppState;
    use session_auth_axum::fallback::file_and_error_handler;
    use leptos_axum::{generate_route_list, LeptosRoutes, handle_server_fns_with_context};
    use leptos::{logging::log, provide_context, get_configuration};
    use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};
    use axum_session::{SessionConfig, SessionLayer, SessionStore};
    use axum_session_auth::{AuthSessionLayer, AuthConfig, SessionSqlitePool};

    async fn server_fn_handler(State(app_state): State<AppState>, auth_session: AuthSession, path: Path<String>, headers: HeaderMap, raw_query: RawQuery,
    request: Request<AxumBody>) -> impl IntoResponse {

        log!("{:?}", path);

        handle_server_fns_with_context(path, headers, raw_query, move || {
            provide_context(auth_session.clone());
            provide_context(app_state.pool.clone());
        }, request).await
    }

    async fn leptos_routes_handler(auth_session: AuthSession, State(app_state): State<AppState>, req: Request<AxumBody>) -> Response{
            let handler = leptos_axum::render_route_with_context(app_state.leptos_options.clone(),
            app_state.routes.clone(),
            move || {
                provide_context(auth_session.clone());
                provide_context(app_state.pool.clone());
            },
            TodoApp
        );
        handler(req).await.into_response()
    }

    #[tokio::main]
    async fn main() {
        simple_logger::init_with_level(log::Level::Info).expect("couldn't initialize logging");

        let pool = SqlitePoolOptions::new()
            .connect("sqlite:Todos.db")
            .await
            .expect("Could not make pool.");

        // Auth section
        let session_config = SessionConfig::default().with_table_name("axum_sessions");
        let auth_config = AuthConfig::<i64>::default();
        let session_store = SessionStore::<SessionSqlitePool>::new(Some(pool.clone().into()), session_config).await.unwrap();

        sqlx::migrate!()
            .run(&pool)
            .await
            .expect("could not run SQLx migrations");

        // Explicit server function registration is no longer required
        // on the main branch. On 0.3.0 and earlier, uncomment the lines
        // below to register the server functions.
        // _ = GetTodos::register();
        // _ = AddTodo::register();
        // _ = DeleteTodo::register();
        // _ = Login::register();
        // _ = Logout::register();
        // _ = Signup::register();
        // _ = GetUser::register();
        // _ = Foo::register();

        // Setting this to None means we'll be using cargo-leptos and its env vars
        let conf = get_configuration(None).await.unwrap();
        let leptos_options = conf.leptos_options;
        let addr = leptos_options.site_addr;
        let routes = generate_route_list(TodoApp);

        let app_state = AppState{
            leptos_options,
            pool: pool.clone(),
            routes: routes.clone(),
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
        let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
        axum::serve(listener, app.into_make_service())
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
