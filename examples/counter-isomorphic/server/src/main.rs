use actix_files::Files;
use actix_web::*;
use counter_isomorphic::*;
use leptos::*;

#[get("/")]
async fn render() -> impl Responder {
    HttpResponse::Ok().content_type("text/html").body(format!(
        r#"<!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <title>Isomorphic Counter</title>
            </head>
            <body>
                {}
            </body>
            <script type="module">import init, {{ main }} from './pkg/counter_client.js'; init().then(main);</script>
        </html>"#,
        run_scope({
            |cx| {
                view! { cx, <Counters/>}
            }
        })
    ))
}

#[get("/api/events")]
async fn counter_events() -> impl Responder {
    use futures::StreamExt;

    let stream =
        futures::stream::once(async { counter_isomorphic::get_server_count().await.unwrap_or(0) })
            .chain(COUNT_CHANNEL.clone())
            .map(|value| {
                Ok(web::Bytes::from(format!(
                    "event: message\ndata: {value}\n\n"
                ))) as Result<web::Bytes>
            });
    HttpResponse::Ok()
        .insert_header(("Content-Type", "text/event-stream"))
        .streaming(stream)
}

#[post("/api/get_server_count")]
async fn get_server_count() -> impl Responder {
    counter_isomorphic::get_server_count()
        .await
        .unwrap()
        .to_string()
}

#[post("/api/clear_server_count")]
async fn clear_server_count() -> impl Responder {
    counter_isomorphic::clear_server_count()
        .await
        .unwrap()
        .to_string()
}

#[post("/api/adjust_server_count")]
async fn adjust_server_count(data: web::Form<AdjustServerCount>) -> impl Responder {
    let AdjustServerCount { delta } = data.0;
    counter_isomorphic::adjust_server_count(delta).await;
    HttpResponse::SeeOther()
        .insert_header(("Location", "/"))
        .body("")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    //simple_logger::init_with_level(log::Level::Debug).expect("couldn't initialize logging");

    // load TLS keys
    // to create a self-signed temporary cert for testing:
    // `openssl req -x509 -newkey rsa:4096 -nodes -keyout key.pem -out cert.pem -days 365 -subj '/CN=localhost'`

    /* let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
    builder
        .set_private_key_file("key.pem", SslFiletype::PEM)
        .unwrap();
    builder.set_certificate_chain_file("cert.pem").unwrap(); */

    HttpServer::new(|| {
        App::new()
            .service(render)
            .service(Files::new("/pkg", "../client/pkg"))
            .service(get_server_count)
            .service(clear_server_count)
            .service(adjust_server_count)
            .service(counter_events)
        //.wrap(middleware::Compress::default())
    })
    .bind(("127.0.0.1", 8080))?
    //.bind_openssl(("127.0.0.1", 8080), builder)?
    .run()
    .await
}
