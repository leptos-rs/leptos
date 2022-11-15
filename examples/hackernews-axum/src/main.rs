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
        body::Bytes,
        response::{IntoResponse, Response},
    };
    use std::net::SocketAddr;
    use futures::StreamExt;
    use leptos_meta::*;
    use leptos_router::*;
    use leptos_hackernews_axum::*;
    use crate::handlers::file_handler;
    use std::io;

    // #[get("/static/style.css")]
    // async fn css() -> impl Responder {
    //     NamedFile::open_async("./style.css").await
    // }

    #[tokio::main]
    async fn main() -> std::io::Result<()> {
        let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

        log::debug!("serving at {addr}");

        simple_logger::init_with_level(log::Level::Debug).expect("couldn't initialize logging");

        // uncomment these lines (and .bind_openssl() below) to enable HTTPS, which is sometimes
        // necessary for proper HTTP/2 streaming

        // load TLS keys
        // to create a self-signed temporary cert for testing:
        // `openssl req -x509 -newkey rsa:4096 -nodes -keyout key.pem -out cert.pem -days 365 -subj '/CN=localhost'`
        // let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
        // builder
        //     .set_private_key_file("key.pem", SslFiletype::PEM)
        //     .unwrap();
        // builder.set_certificate_chain_file("cert.pem").unwrap();

        // HttpServer::new(|| {
        //     App::new()
        //         .service(css)
        //         .service(
        //             web::scope("/pkg")
        //                 .service(Files::new("", "./dist"))
        //                 .wrap(middleware::Compress::default()),
        //         )
        //         .service(render_app)
        // })
        // .bind(("127.0.0.1", 8080))?
        // // replace .bind with .bind_openssl to use HTTPS
        // //.bind_openssl(&format!("{}:{}", host, port), builder)?
        // .run()
        // .await

        // build our application with a route
        let app = Router::new()
        // `GET /` goes to `root`
        .nest("/pkg", get(file_handler))
        .fallback(fallback.into_service());

        // run our app with hyper
        // `axum::Server` is a re-export of `hyper::Server`
        tracing::debug!("listening on {}", addr);
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
