use std::pin::Pin;

use actix_files::{Directory, Files, NamedFile};
use actix_web::{*, http::header::CACHE_CONTROL, dev::Service};
use futures::{StreamExt, stream::FuturesUnordered, Future};
use hackernews_app::*;
use leptos::*;
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};

struct ActixIntegration {
    path: String,
}

impl History for ActixIntegration {
    fn location(&self, cx: leptos::Scope) -> ReadSignal<LocationChange> {
        create_signal(
            cx,
            LocationChange {
                value: self.path.clone(),
                replace: false,
                scroll: true,
                state: State(None),
            },
        )
        .0
    }

    fn navigate(&self, loc: &LocationChange) {}
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

    log::debug!("GET {path}");

    let integration = ActixIntegration { path };

    HttpResponse::Ok().content_type("text/html").streaming(
        // Head HTML — allows you to start streaming WASM before we've even run the template code
        futures::stream::once(async move {
            r#"<!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <title>Leptos • Hacker News</title>
                <link rel="stylesheet" href="/static/style.css"/>
                <script type="module">import init, { main } from '/pkg/hackernews_client.js'; init().then(main);</script>
            </head>
            <body>"#
                .to_string()
        })
        
        .chain({
            let ((shell, pending_resources, pending_fragments, serializers), disposer) = run_scope_undisposed({
                move |cx| {
                    // the actual app body/template code
                    // this does NOT contain any of the data being loaded asynchronously in resources
                    let shell = view! { cx, 
                        <div>
                            <Router mode=integration>
                                <App />
                            </Router>
                        </div>
                    };

                    let resources = cx.all_resources();
                    let pending_resources = serde_json::to_string(&resources).unwrap();

                    //tx.unbounded_send(format!("{template}<script>const __LEPTOS_PENDING_RESOURCES = {pending_resources}; const __LEPTOS_RESOLVED_RESOURCES = {{}}; const __LEPTOS_RESOURCE_RESOLVERS = {{}};</script>"));
                    (shell, pending_resources, cx.pending_fragments(), cx.serialization_resolvers())
                }
            });

            let fragments = FuturesUnordered::new();
            for (fragment_id, fut) in pending_fragments {
                fragments.push(async move { (fragment_id, fut.await)} )
            }
            
            futures::stream::once(async move {
                format!(r#"
                    {shell}
                    <script>
                        __LEPTOS_PENDING_RESOURCES = {pending_resources};
                        __LEPTOS_RESOLVED_RESOURCES = new Map();
                        __LEPTOS_RESOURCE_RESOLVERS = new Map();
                    </script>
                "#)
            })
            .chain(fragments.map(|(fragment_id, html)| {
                format!(
                    r#"<script>const f = document.querySelector(`[data-fragment-id="{fragment_id}"]`); f.innerHTML = {html:?};</script>"#
                )
            }))
            .chain(serializers.map(|(id, json)| {
                let id = serde_json::to_string(&id).unwrap();
                format!(
                    r#"<script>if(__LEPTOS_RESOURCE_RESOLVERS.get({id})) {{ console.log("(create_resource) calling resolver"); __LEPTOS_RESOURCE_RESOLVERS.get({id})({json:?}) }} else {{ console.log("(create_resource) saving data for resource creation"); __LEPTOS_RESOLVED_RESOURCES.set({id}, {json:?}) }} </script>"#,
                )
            })
            .chain(futures::stream::once(async { "</body></html>".to_string() })))

            // TODO handle disposer; currently leaking memory from scope
        })
        .map(|html| Ok(web::Bytes::from(html)) as Result<web::Bytes>),
    )
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
                    .wrap(middleware::Compress::default())
                )
            .service(render_app)
            //.wrap(middleware::Compress::default())
            /* .service(
                    web::scope("/pkg")
                        // cache settings for static files
                        .wrap_fn(|req, srv| {
                            let fut = srv.call(req);
                            async {
                                let mut res = fut.await?;
                                res.headers_mut().insert(
                                    CACHE_CONTROL,
                                    actix_web::http::header::HeaderValue::from_static(
                                        "max-age=31536000",
                                    ),
                                );
                                Ok(res)
                            }
                        })
                        .wrap(middleware::Compress::default())
                        .service(Files::new("", "../hackernews-client/pkg")),
                ) */
    })
    .bind(("127.0.0.1", 8080))?
    //.bind_openssl(&format!("{}:{}", host, port), builder)?
    .run()
    .await
}
