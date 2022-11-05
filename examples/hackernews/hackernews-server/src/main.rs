use actix_files::{Files, NamedFile};
use actix_web::*;
use futures::StreamExt;
use hackernews_app::*;
use leptos::*;
use leptos_meta::*;
use leptos_router::*;
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};

#[get("/static/style.css")]
async fn css() -> impl Responder {
    NamedFile::open_async("../hackernews-app/style.css").await
}

// match every path — our router will handle actual dispatch
#[get("{tail:.*}")]
async fn render_app(req: HttpRequest) -> impl Responder {
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
                <script type="module">import init, { main } from '/pkg/hackernews_client.js'; init().then(main);</script>"#;
    let tail = "</body></html>";

    HttpResponse::Ok().content_type("text/html").streaming(
        futures::stream::once(async { head.to_string() })
            .chain(render_to_stream(move |cx| {
                let app = app(cx);
                let head = use_context::<MetaContext>(cx)
                    .map(|meta| meta.dehydrate())
                    .unwrap_or_default();
                format!("{head}</head><body>{app}")
            }))
            .chain(futures::stream::once(async { tail.to_string() }))
            .map(|html| Ok(web::Bytes::from(html)) as Result<web::Bytes>),
    )
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    log::debug!("serving at {host}:{port}");

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

    HttpServer::new(|| {
        App::new()
            .service(css)
            .service(
                web::scope("/pkg")
                    .service(Files::new("", "../hackernews-client/pkg"))
                    .wrap(middleware::Compress::default()),
            )
            .service(render_app)
    })
    .bind(("127.0.0.1", 8080))?
    // replace .bind with .bind_openssl to use HTTPS
    //.bind_openssl(&format!("{}:{}", host, port), builder)?
    .run()
    .await
}
