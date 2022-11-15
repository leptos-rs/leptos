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

        #[get("/static/style.css")]
        async fn css() -> impl Responder {
            NamedFile::open_async("./style.css").await
        }

        // match every path — our router will handle actual dispatch
        #[get("{tail:.*}")]
        async fn render_app(req: Request) -> StreamBody<impl Stream<Item = io::Result<Bytes>> {
            let path = req.path();

            let query = req.query_string();
            let path = if query.is_empty() {
                "http://leptos".to_string() + path
            } else {
                "http://leptos".to_string() + path + "?" + query
            };

            let app = move |cx| {
                let integration = ServerIntegration { path: path.clone() };
                provide_context(cx, RouterIntegrationContext::new(integration));

                view! { cx, <App/> }
            };

            let head = r#"<!DOCTYPE html>
                <html lang="en">
                    <head>
                        <meta charset="utf-8"/>
                        <meta name="viewport" content="width=device-width, initial-scale=1"/>
                        <script type="module">import init, { main } from '/pkg/leptos_hackernews_axum.js'; init().then(main);</script>"#;
            let tail = "</body></html>";

                let stream = futures::stream::once(async { head.to_string() })
                    .chain(render_to_stream(move |cx| {
                        let app = app(cx);
                        let head = use_context::<MetaContext>(cx)
                            .map(|meta| meta.dehydrate())
                            .unwrap_or_default();
                        format!("{head}</head><body>{app}")
                    }))
                    .chain(futures::stream::once(async { tail.to_string() }))
                    .map(|html| Ok(Bytes::from(html)) as Result<Bytes>);
                    StreamBody::new(stream)
        }

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
            .nest("/pkg", get(file_handler));

            // run our app with hyper
            // `axum::Server` is a re-export of `hyper::Server`
            tracing::debug!("listening on {}", addr);
            axum::Server::bind(&addr)
                .serve(app.into_make_service())
                .await
                .unwrap();

        }

        // client-only stuff for Trunk
        else {
            use leptos_hackernews::*;

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
}
