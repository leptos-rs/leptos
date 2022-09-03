use actix_web::*;
use leptos::*;
use todomvc::*;

#[get("/")]
async fn render_todomvc() -> impl Responder {
    let mut buffer: String;
    _ = create_scope(|cx| {
        let todos = Todos(vec![
            Todo::new(cx, 0, "Buy milk".to_string()),
            Todo::new(cx, 1, "???".to_string()),
            Todo::new(cx, 2, "Profit!".to_string())
        ]);

        buffer = view! { <div id="root"><TodoMVC todos=todos/></div> };
    });
    buffer
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .service(render_todomvc)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}