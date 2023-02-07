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
                                futures_timer::Delay::new(std::time::Duration::from_secs(1)).await;
                            },
                        );

                        let two_seconds = create_resource(
                            cx,
                            || (),
                            |_| async {
                                futures_timer::Delay::new(std::time::Duration::from_secs(2)).await;
                            },
                        );

                        let three_seconds = create_resource(
                            cx,
                            || (),
                            |_| async {
                                futures_timer::Delay::new(std::time::Duration::from_secs(3)).await;
                            },
                        );

                        view! { cx,
                            <html>
                                <head></head>
                                <body>
                                    <App/>
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
    .bind(&addr)?
    .run()
    .await
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
    // no client-side main function
    // unless we want this to work with e.g., Trunk for pure client-side testing
    // see lib.rs for hydration function instead
}
