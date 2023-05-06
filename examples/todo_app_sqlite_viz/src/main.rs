use cfg_if::cfg_if;

// boilerplate to run in different modes
cfg_if! {
    if #[cfg(feature = "ssr")] {
    use leptos::*;
    use crate::fallback::file_and_error_handler;
    use crate::todo::*;
    use leptos_viz::{generate_route_list, LeptosRoutes};
    use std::sync::Arc;
    use todo_app_sqlite_viz::*;
    use viz::{
        types::{State, StateError},
        Request, RequestExt, Response, Result, Router, ServiceMaker,
    };

    //Define a handler to test extractor with state
    async fn custom_handler(req: Request) -> Result<Response> {
        let id = req.params::<String>()?;
        let options = &*req
            .state::<Arc<LeptosOptions>>()
            .ok_or(StateError::new::<LeptosOptions>())?;
        let handler = leptos_viz::render_app_to_stream_with_context(
            options.clone(),
            move |cx| {
                provide_context(cx, id.clone());
            },
            |cx| view! { cx, <TodoApp/> },
        );
        handler(req).await
    }

    #[tokio::main]
    async fn main() {
        simple_logger::init_with_level(log::Level::Debug)
            .expect("couldn't initialize logging");

        let _conn = db().await.expect("couldn't connect to DB");
        /* sqlx::migrate!()
        .run(&mut conn)
        .await
        .expect("could not run SQLx migrations"); */

        crate::todo::register_server_functions();

        // Setting this to None means we'll be using cargo-leptos and its env vars
        let conf = get_configuration(None).await.unwrap();
        let leptos_options = conf.leptos_options;
        let addr = leptos_options.site_addr;
        let routes = generate_route_list(|cx| view! { cx, <TodoApp/> }).await;

        // build our application with a route
        let app = Router::new()
            .post("/api/:fn_name*", leptos_viz::handle_server_fns)
            .get("/special/:id", custom_handler)
            .leptos_routes(
                leptos_options.clone(),
                routes,
                |cx| view! { cx, <TodoApp/> },
            )
            .get("/*", file_and_error_handler)
            .with(State(Arc::new(leptos_options)));

        // run our app with hyper
        // `viz::Server` is a re-export of `hyper::Server`
        log!("listening on http://{}", &addr);
        viz::Server::bind(&addr)
            .serve(ServiceMaker::from(app))
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
