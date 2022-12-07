use actix_files::Files;
use actix_web::{web, App, HttpResponse, HttpServer};
use hydration_test::*;
use leptos::*;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
  HttpServer::new(|| App::new()
      .service(Files::new("/pkg", "./pkg"))
      .route("/", web::get().to(
        || async {
          HttpResponse::Ok()
            .content_type("text/html")
            .body({
              let runtime = create_runtime();
              let html = run_scope(runtime, move |cx| {
                view! {
                  cx,
                  <App/>
                }.render_to_string().to_string()
              });
              runtime.dispose();
              let html = format!(
                r#"<!DOCTYPE html>
                <html>
                  <head>
                  <script type="module">import init from '/pkg/hydration_test.js'; init();</script>
                  </head>
                  <body>{html}</body>
                </html>"#
              );

              println!("{html}");
              
              html
            })
        }
      )
    ))
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
