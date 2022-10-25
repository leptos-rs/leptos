use actix_files::Files;
use actix_web::*;
use counter_isomorphic::*;
use leptos::*;

#[get("/")]
async fn render_todomvc() -> impl Responder {
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
                view! { cx, <Counter/>}
            }
        })
    ))
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
    HttpServer::new(|| {
        App::new()
            .service(render_todomvc)
            .service(Files::new("/pkg", "../client/pkg"))
            .service(get_server_count)
            .service(clear_server_count)
            .service(adjust_server_count)
            .wrap(middleware::Compress::default())
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
