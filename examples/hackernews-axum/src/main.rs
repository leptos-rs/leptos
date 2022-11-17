use cfg_if::cfg_if;
use leptos::*;

// boilerplate to run in different modes
cfg_if! {
if #[cfg(feature = "ssr")] {
    // use actix_files::{Files, NamedFile};
    // use actix_web::*;
    use axum::{
        routing::{get},
        Router,
        handler::Handler,
    };
    use std::net::SocketAddr;
    use leptos_hackernews_axum::handlers::{file_handler, get_static_file_handler, render_app};

    // #[get("/static/style.css")]
    // async fn css() -> impl Responder {
    //     NamedFile::open_async("./style.css").await
    // }

    #[tokio::main]
    async fn main() {
        let addr = SocketAddr::from(([127, 0, 0, 1], 8082));

        log::debug!("serving at {addr}");

        simple_logger::init_with_level(log::Level::Debug).expect("couldn't initialize logging");

        // build our application with a route
        let app = Router::new()
        // `GET /` goes to `root`
        .nest("/pkg", get(file_handler))
        .nest("/static", get(get_static_file_handler))
        .fallback(render_app.into_service());

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
        use leptos_hackernews_axum::*;

        pub fn main() {
            console_error_panic_hook::set_once();
            _ = console_log::init_with_level(log::Level::Debug);
            console_error_panic_hook::set_once();
            mount_to_body(|cx| {
                view! { cx, <App/> }
            });
        }
    }
}
