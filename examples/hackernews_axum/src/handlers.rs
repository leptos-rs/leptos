use axum::{
    body::Body,
    http::{Request, Response, StatusCode, Uri},
    response::IntoResponse,
};
use tower::ServiceExt;
use tower_http::services::ServeDir;

pub async fn file_handler(
    uri: Uri,
) -> Result<Response<Body>, (StatusCode, String)> {
    let res = get_static_file(uri.clone(), "/pkg").await?;

    if res.status() == StatusCode::NOT_FOUND {
        // try with `.html`
        // TODO: handle if the Uri has query parameters
        match format!("{}.html", uri).parse() {
            Ok(uri_html) => get_static_file(uri_html, "/pkg").await,
            Err(_) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Invalid URI".to_string(),
            )),
        }
    } else {
        Ok(res)
    }
}

pub async fn get_static_file_handler(
    uri: Uri,
) -> Result<Response<Body>, (StatusCode, String)> {
    let res = get_static_file(uri.clone(), "/static").await?;

    if res.status() == StatusCode::NOT_FOUND {
        Err((StatusCode::INTERNAL_SERVER_ERROR, "Invalid URI".to_string()))
    } else {
        Ok(res)
    }
}

async fn get_static_file(
    uri: Uri,
    base: &str,
) -> Result<Response<Body>, (StatusCode, String)> {
    let req = Request::builder().uri(&uri).body(Body::empty()).unwrap();

    // `ServeDir` implements `tower::Service` so we can call it with `tower::ServiceExt::oneshot`
    // When run normally, the root should be the crate root
    if base == "/static" {
        match ServeDir::new("./static").oneshot(req).await {
            Ok(res) => Ok(res.into_response()),
            Err(err) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Something went wrong: {}", err),
            )),
        }
    } else if base == "/pkg" {
        match ServeDir::new("./pkg").oneshot(req).await {
            Ok(res) => Ok(res.into_response()),
            Err(err) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Something went wrong: {}", err),
            )),
        }
    } else {
        Err((StatusCode::NOT_FOUND, "Not Found".to_string()))
    }
}
