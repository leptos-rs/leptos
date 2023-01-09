use cfg_if::cfg_if;
use leptos::*;
// boilerplate to run in different modes
cfg_if! {
if #[cfg(feature = "ssr")] {
    use axum::{
        routing::post,
        extract::Extension,
        Router,
    };
    use crate::todo::*;
    use todo_app_sqlite_axum::*;
    use crate::file::file_handler;
    use leptos_axum::{generate_route_list, LeptosRoutes};
    use std::sync::Arc;

    #[tokio::main]
    async fn main() {
        simple_logger::init_with_level(log::Level::Debug).expect("couldn't initialize logging");

        let conn = db().await.expect("couldn't connect to DB");
        /* sqlx::migrate!()
            .run(&mut conn)
            .await
            .expect("could not run SQLx migrations"); */

        crate::todo::register_server_functions();

        let conf = get_configuration(Some("Cargo.toml")).await.unwrap();
        let leptos_options = conf.leptos_options;
        let addr = leptos_options.site_address.clone();
        let routes = generate_route_list(|cx| view! { cx, <TodoApp/> }).await;

        // build our application with a route
        let app = Router::new()
        .route("/api/*fn_name", post(leptos_axum::handle_server_fns))
        .leptos_routes(leptos_options.clone(), routes, |cx| view! { cx, <TodoApp/> } )
        .fallback(file_handler)
        .layer(Extension(Arc::new(leptos_options)));

        // run our app with hyper
        // `axum::Server` is a re-export of `hyper::Server`
        log!("listening on {}", &addr);
        axum::Server::bind(&addr)
            .serve(app.into_make_service())
            .await
            .unwrap();
    }
}

    // client-only stuff for Trunk
    else {
        use todo_app_sqlite_axum::todo::*;

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
