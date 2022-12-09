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
              let html = render_to_string(|cx| 
                view! {
                  cx,
                  <App/>
                }
              );
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
