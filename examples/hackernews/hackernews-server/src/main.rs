use actix_files::{Directory, Files, NamedFile};
use actix_web::*;
use hackernews_app::*;
use leptos::*;

struct ActixIntegration {
    path: String,
}

impl History for ActixIntegration {
    fn location(&self, cx: leptos::Scope) -> ReadSignal<LocationChange> {
        eprintln!("path = {}", self.path);
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

#[get("{tail:.*}")]
async fn render_app(req: HttpRequest) -> impl Responder {
    let path = req.path();
    let query = req.query_string();
    let path = if query.is_empty() {
        "http://leptos".to_string() + path
    } else {
        "http://leptos".to_string() + path + "?" + query
    };
    let integration = ActixIntegration { path };

    HttpResponse::Ok().content_type("text/html").body(format!(
        r#"<!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <title>"Leptos • Hacker News"</title>
                <link rel="stylesheet" href="/static/style.css"/>
            </head>
            <body>{}</body>
            <script type="module">import init, {{ main }} from '/pkg/hackernews_client.js'; init().then(main);</script>
        </html>"#,
        run_scope({
            |cx| {
                view! {         
                    <div>
                        <Router mode=integration>
                            <App />
                        </Router>
                    </div>
                }
            }
        })
    ))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .service(css)
            .service(Files::new("/pkg", "../hackernews-client/pkg"))
            .service(render_app)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
