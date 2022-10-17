use actix_files::{Files, NamedFile};
use actix_web::*;
use futures::StreamExt;
use hackernews_app::*;
use leptos::*;
use leptos_meta::*;
use leptos_router::*;
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};

#[derive(Copy, Clone, Debug)]
struct ActixIntegration {
    path: ReadSignal<String>,
}

impl History for ActixIntegration {
    fn location(&self, cx: leptos::Scope) -> ReadSignal<LocationChange> {
        create_signal(
            cx,
            LocationChange {
                value: self.path.get(),
                replace: false,
                scroll: true,
                state: State(None),
            },
        )
        .0
    }

    fn navigate(&self, _loc: &LocationChange) {}
}

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
        let integration = ActixIntegration {
            path: create_signal(cx, path.clone()).0,
        };
        provide_context(cx, RouterIntegrationContext(std::rc::Rc::new(integration)));

        view! { cx, <App/> }
    };

    let accepts_type = req.headers().get("Accept").map(|h| h.to_str());
    match accepts_type {
        // if asks for JSON, send the loader function JSON or 404
        Some(Ok("application/json")) => {
            let json = loader_to_json(app).await;

            let res = if let Some(json) = json {
                HttpResponse::Ok()
                    .content_type("application/json")
                    .body(json)
            } else {
                HttpResponse::NotFound().body(())
            };

            res
        }
        // otherwise, send HTML
        _ => {
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
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    log::debug!("serving at {host}:{port}");

    simple_logger::init_with_level(log::Level::Debug).expect("couldn't initialize logging");

    // load TLS keys
    // to create a self-signed temporary cert for testing:
    // `openssl req -x509 -newkey rsa:4096 -nodes -keyout key.pem -out cert.pem -days 365 -subj '/CN=localhost'`
    let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
    builder
        .set_private_key_file("key.pem", SslFiletype::PEM)
        .unwrap();
    builder.set_certificate_chain_file("cert.pem").unwrap();

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
    //.bind_openssl(&format!("{}:{}", host, port), builder)?
    .run()
    .await
}
