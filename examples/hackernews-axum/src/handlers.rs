use cfg_if::cfg_if;

cfg_if! {
if #[cfg(feature = "ssr")] {
    use axum::{
        body::{boxed, Body, Bytes, BoxBody, StreamBody},
        http::{Request, Response, StatusCode, Uri},
    };
    use tower::ServiceExt;
    use tower_http::services::ServeDir;
    use std::io;
    use futures::{Stream, StreamExt};
    use leptos::*;
    use leptos_router::*;
    use leptos_meta::*;
    use crate::*;

    pub async fn file_handler(uri: Uri) -> Result<Response<BoxBody>, (StatusCode, String)> {
        let res = get_static_file(uri.clone()).await?;
        println!("{:?}", res);

        if res.status() == StatusCode::NOT_FOUND {
            // try with `.html`
            // TODO: handle if the Uri has query parameters
            match format!("{}.html", uri).parse() {
                Ok(uri_html) => get_static_file(uri_html).await,
                Err(_) => Err((StatusCode::INTERNAL_SERVER_ERROR, "Invalid URI".to_string())),
            }
        } else {
            Ok(res)
        }
    }

    async fn get_static_file(uri: Uri) -> Result<Response<BoxBody>, (StatusCode, String)> {
        let req = Request::builder().uri(uri).body(Body::empty()).unwrap();

        // `ServeDir` implements `tower::Service` so we can call it with `tower::ServiceExt::oneshot`
        // When run normally, the root should be the crate root
        match ServeDir::new("./pkg").oneshot(req).await {
            Ok(res) => Ok(res.map(boxed)),
            Err(err) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Something went wrong: {}", err),
            )),
        }
    }

    // match every path — our router will handle actual dispatch, except for the static css files
    pub async fn render_app(req: Request<Body>) -> StreamBody<impl Stream<Item = io::Result<Bytes>>> {

        // Need to get the path and query string of the Request
        let path = req.uri();
        let query = path.query();

        let full_path;
        if let Some(query) = query {
            full_path = "http://leptos".to_string() + &path.to_string() + "?" + query

        } else {
            full_path = "http://leptos".to_string() + &path.to_string()
        }

        let app = move |cx| {
            let integration = ServerIntegration { path: full_path.clone() };
            provide_context(cx, RouterIntegrationContext::new(integration));

            view! { cx, <App/> }
        };

        let head = r#"<!DOCTYPE html>
                    <html lang="en">
                        <head>
                            <meta charset="utf-8"/>
                            <meta name="viewport" content="width=device-width, initial-scale=1"/>
                            <script type="module">import init, { main } from '/pkg/leptos_hackernews_axum.js'; init().then(main);</script>"#;
        let tail = "</body></html>";

            let stream = futures::stream::once(async { head.to_string() })
                .chain(render_to_stream(move |cx| {
                    let app = app(cx);
                    let head = use_context::<MetaContext>(cx)
                        .map(|meta| meta.dehydrate())
                        .unwrap_or_default();
                    format!("{head}</head><body>{app}")
                }))
                .chain(futures::stream::once(async { tail.to_string() }))
                .map(|html| Ok(Bytes::from(html)) as io::Result<Bytes>);
                StreamBody::new(stream)
    }
    }
    }
