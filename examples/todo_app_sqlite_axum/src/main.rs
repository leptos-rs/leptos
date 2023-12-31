use cfg_if::cfg_if;
// boilerplate to run in different modes
cfg_if! {
    if #[cfg(feature = "ssr")] {
    use leptos::*;
    use axum::{
        routing::{post, get},
        extract::{State, Path},
        http::Request,
        response::{IntoResponse, Response},
        Router,
    };
    use axum::body::Body;
    use crate::todo::*;
    use todo_app_sqlite_axum::*;
    use crate::fallback::file_and_error_handler;
    use leptos_axum::{generate_route_list, LeptosRoutes};

    //Define a handler to test extractor with state
    async fn custom_handler(Path(id): Path<String>, State(options): State<LeptosOptions>, req: Request<Body>) -> Response{
            let handler = leptos_axum::render_app_to_stream_with_context(options,
            move || {
                provide_context(id.clone());
            },
            || view! { <TodoApp/> }
        );
            handler(req).await.into_response()
    }

    #[tokio::main]
    async fn main() {
        simple_logger::init_with_level(log::Level::Error).expect("couldn't initialize logging");

        let _conn = db().await.expect("couldn't connect to DB");
        /* sqlx::migrate!()
            .run(&mut conn)
            .await
            .expect("could not run SQLx migrations"); */

        // Explicit server function registration is no longer required
        // on the main branch. On 0.3.0 and earlier, uncomment the lines
        // below to register the server functions.
        // _ = GetTodos::register();
        // _ = AddTodo::register();
        // _ = DeleteTodo::register();

        // Setting this to None means we'll be using cargo-leptos and its env vars
        let conf = get_configuration(None).await.unwrap();
        let leptos_options = conf.leptos_options;
        let addr = leptos_options.site_addr;
        let routes = generate_route_list(TodoApp);

        // build our application with a route
        let app = Router::new()
        .route("/api/*fn_name", post(leptos_axum::handle_server_fns))
        .route("/special/:id", get(custom_handler))
        .leptos_routes(&leptos_options, routes, || view! { <TodoApp/> } )
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
}

    // client-only stuff for Trunk
    else {
        pub fn main() {
            // This example cannot be built as a trunk standalone CSR-only app.
            // Only the server may directly connect to the database.
        }
    }
}
