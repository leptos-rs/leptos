#[cfg(feature = "ssr")]
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    use actix_files::Files;
    use actix_web::*;
    use futures::StreamExt;
    use leptos::*;
    use leptos_actix::{generate_route_list, LeptosRoutes};
    use leptos_start::app::*;

    let conf = get_configuration(Some("Cargo.toml")).await.unwrap();
    let addr = conf.leptos_options.site_address;
    // Generate the list of routes in your Leptos App
    let routes = generate_route_list(|cx| view! { cx, <App/> });

    let mut builder =
        openssl::ssl::SslAcceptor::mozilla_intermediate(openssl::ssl::SslMethod::tls()).unwrap();
    builder
        .set_private_key_file("key.pem", openssl::ssl::SslFiletype::PEM)
        .unwrap();
    builder.set_certificate_chain_file("cert.pem").unwrap();

    HttpServer::new(move || {
        let leptos_options = &conf.leptos_options;
        let site_root = &leptos_options.site_root;

        App::new()
            .route("/api/{tail:.*}", leptos_actix::handle_server_fns())
            .route(
                "/",
                web::get().to(|| async {
                    let stream = render_to_stream_in_order(|cx| {
                        let one_second = create_resource(
                            cx,
                            || (),
                            |_| async {
                                futures_timer::Delay::new(std::time::Duration::from_secs(2)).await;
                            },
                        );

                        view! { cx,
                            <html>
                                <head></head>
                                <body>
                                    <main>
                                        <h1>"Hello, world!"</h1>
                                        <Suspense fallback=|| "Loading...">
                                            <p>
                                                "One second: "
                                                {format!("{:?}", one_second.read())}
                                            </p>
                                        </Suspense>
                                    </main>
                                </body>
                            </html>
                        }
                        .into_view(cx)
                    })
                    .await
                    .inspect(|html| println!("chunk: {html}"))
                    .map(|html| Ok(web::Bytes::from(html)) as Result<web::Bytes>);

                    HttpResponse::Ok()
                        .content_type("text/html")
                        .streaming(stream)
                }),
            )
            /* .leptos_routes(
                leptos_options.to_owned(),
                routes.to_owned(),
                |cx| view! { cx, <App/> },
            ) */
            .service(Files::new("/", site_root))
        //.wrap(middleware::Compress::default())
    })
    .bind_openssl(&addr, builder)?
    //.bind(&addr)?
    //.bind_rustls(&addr, load_certs().unwrap())?
    .run()
    .await
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
    // no client-side main function
    // unless we want this to work with e.g., Trunk for pure client-side testing
    // see lib.rs for hydration function instead
}
