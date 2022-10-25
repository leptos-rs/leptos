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

#[get("/api/get_server_count")]
async fn get_server_count() -> impl Responder {
    counter_isomorphic::get_server_count()
        .await
        .unwrap()
        .to_string()
}

#[get("/api/clear_server_count")]
async fn clear_server_count() -> impl Responder {
    counter_isomorphic::clear_server_count()
        .await
        .unwrap()
        .to_string()
}

#[get("/api/increment_server_count")]
async fn increment_server_count() -> impl Responder {
    counter_isomorphic::increment_server_count()
        .await
        .unwrap()
        .to_string()
}

#[get("/api/decrement_server_count")]
async fn decrement_server_count() -> impl Responder {
    counter_isomorphic::decrement_server_count()
        .await
        .unwrap()
        .to_string()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .service(render_todomvc)
            .service(Files::new("/pkg", "../client/pkg"))
            .service(get_server_count)
            .service(clear_server_count)
            .service(increment_server_count)
            .service(decrement_server_count)
            .wrap(middleware::Compress::default())
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
