use std::{cell::RefCell, rc::Rc};

use actix_files::{Directory, Files, NamedFile};
use actix_web::*;
use leptos::*;
use todomvc::*;

#[get("/")]
async fn render_todomvc() -> impl Responder {
    HttpResponse::Ok().content_type("text/html").body(format!(
        r#"<!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <link rel="stylesheet" href="/static/todomvc-common/base.css"/>
                <link rel="stylesheet" href="/static/todomvc-app-css/index.css"/>
                <title>"Leptos • TodoMVC"</title>
            </head>
            <body>
                {}
            </body>
            <script type="module">import init, {{ main }} from './pkg/todomvc_ssr_client.js'; init().then(main);</script>
        </html>"#,
        run_scope({
            |cx| {
                let todos = Todos(vec![
                    Todo::new(cx, 0, "Buy milk".to_string()),
                    Todo::new(cx, 1, "???".to_string()),
                    Todo::new(cx, 2, "Profit!".to_string()),
                ]);

                view! {
                    <main>
                        <TodoMVC todos=todos/>
                    </main>
                }
            }
        })
    ))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .service(render_todomvc)
            .service(Files::new("/static", "../../todomvc/node_modules"))
            .service(Files::new("/pkg", "../todomvc-ssr-client/pkg"))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
