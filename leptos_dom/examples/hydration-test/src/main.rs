use actix_files::Files;
use actix_web::*;
use futures::StreamExt;
use hydration_test::*;
use leptos::prelude::*;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
  HttpServer::new(|| App::new()
      .service(Files::new("/pkg", "./pkg"))
      .route("/", web::get().to(
        || async {
          let pkg_path = "/pkg/hydration_test";

          let head = format!(
            r#"<!DOCTYPE html>
            <html lang="en">
                <head>
                    <meta charset="utf-8"/>
                    <meta name="viewport" content="width=device-width, initial-scale=1"/>
                    <link rel="modulepreload" href="{pkg_path}.js">
                    <link rel="preload" href="{pkg_path}_bg.wasm" as="fetch" type="application/wasm" crossorigin="">
                    <script type="module">import init, {{ hydrate }} from '{pkg_path}.js'; init('{pkg_path}_bg.wasm').then(hydrate);</script>
                </head>
                <body>"#
        );

        let tail = "</body></html>";

        HttpResponse::Ok().content_type("text/html").streaming(
            futures::stream::once(async move { head.clone() })
            .chain(render_to_stream( 
                || view! { <App/> }.into_view(),
            ))
            .chain(futures::stream::once(async { tail.to_string() }))
            .inspect(|html| println!("{html}"))
            .map(|html| Ok(web::Bytes::from(html)) as Result<web::Bytes>),
      )})
    ))
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
