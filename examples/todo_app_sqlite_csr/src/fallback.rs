use axum::{
    body::Body,
    extract::State,
    http::{Request, Response, StatusCode, Uri},
    response::{Html, IntoResponse, Response as AxumResponse},
};
use leptos::{
    config::LeptosOptions,
    hydration::{AutoReload, HydrationScripts},
    prelude::*,
};
use tower::util::ServiceExt;
use tower_http::services::ServeDir;

pub async fn file_or_index_handler(
    uri: Uri,
    State(options): State<LeptosOptions>,
) -> AxumResponse {
    let root = options.site_root.clone();
    let res = get_static_file(uri.clone(), &root).await.unwrap();

    if res.status() == StatusCode::OK {
        res.into_response()
    } else {
        Html(view! {
            <!DOCTYPE html>
            <html lang="en">
                <head>
                    <meta charset="utf-8"/>
                    <meta name="viewport" content="width=device-width, initial-scale=1"/>
                    <AutoReload options=options.clone() />
                    <HydrationScripts options=options.clone()/>
                    <link rel="stylesheet" id="leptos" href="/pkg/todo_app_sqlite_csr.css"/>
                    <link rel="shortcut icon" type="image/ico" href="/favicon.ico"/>
                </head>
                <body></body>
            </html>
        }.to_html()).into_response()
    }
}

async fn get_static_file(
    uri: Uri,
    root: &str,
) -> Result<Response<Body>, (StatusCode, String)> {
    let req = Request::builder()
        .uri(uri.clone())
        .body(Body::empty())
        .unwrap();
    // `ServeDir` implements `tower::Service` so we can call it with `tower::ServiceExt::oneshot`
    // This path is relative to the cargo root
    match ServeDir::new(root).oneshot(req).await {
        Ok(res) => Ok(res.into_response()),
        Err(err) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {err}"),
        )),
    }
}
