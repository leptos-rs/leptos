use cfg_if::cfg_if;
use leptos::*;

// boilerplate to run in different modes
cfg_if! {
if #[cfg(feature = "ssr")] {
    use axum::{
        routing::{get, post},
        Router,
        handler::Handler,
    };
    use std::net::SocketAddr;
    use crate::todo::*;
    use todo_app_sqlite_axum::handlers::{file_handler, get_static_file_handler};
    use todo_app_sqlite_axum::*;

    #[tokio::main]
    async fn main() {
        let addr = SocketAddr::from(([127, 0, 0, 1], 8082));
        log::debug!("serving at {addr}");

        simple_logger::init_with_level(log::Level::Debug).expect("couldn't initialize logging");

        let mut conn = db().await.expect("couldn't connect to DB");
        sqlx::migrate!()
            .run(&mut conn)
            .await
            .expect("could not run SQLx migrations");

        crate::todo::register_server_functions();

        // build our application with a route
        let app = Router::new()
        .route("/api/*path", post(leptos_axum::handle_server_fns))
        .nest("/pkg", get(file_handler))
        .nest("/static", get(get_static_file_handler))
        .fallback(leptos_axum::render_app_to_stream("todo_app_sqlite_axum", |cx| view! { cx, <TodoApp/> }).into_service());

        // run our app with hyper
        // `axum::Server` is a re-export of `hyper::Server`
        log!("listening on {}", addr);
        axum::Server::bind(&addr)
            .serve(app.into_make_service())
            .await
            .unwrap();
    }
}

    // client-only stuff for Trunk
    else {
        use todo_app_sqlite_axum::*;

        pub fn main() {
            console_error_panic_hook::set_once();
            _ = console_log::init_with_level(log::Level::Debug);
            console_error_panic_hook::set_once();
            mount_to_body(|cx| {
                view! { cx, <TodoApp/> }
            });
        }
    }
}
