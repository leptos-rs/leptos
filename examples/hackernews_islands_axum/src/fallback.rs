use axum::{
    body::Body,
    http::{header, Request, Response, StatusCode, Uri},
    response::{IntoResponse, Response as AxumResponse},
};
use rust_embed::Embed;
use std::borrow::Cow;

#[cfg(not(debug_assertions))]
const DEV_MODE: bool = false;

#[cfg(debug_assertions)]
const DEV_MODE: bool = true;

#[derive(Embed)]
#[folder = "target/site/"]
struct Assets;

pub async fn file_and_error_handler(
    uri: Uri,
    req: Request<Body>,
) -> AxumResponse {
    let accept_encoding = req
        .headers()
        .get("accept-encoding")
        .map(|h| h.to_str().unwrap_or("none"))
        .unwrap_or("none")
        .to_string();
    let static_result = get_static_file(uri.clone(), accept_encoding).await;

    match static_result {
        Ok(res) => {
            if res.status() == StatusCode::OK {
                res.into_response()
            } else {
                (StatusCode::NOT_FOUND, "Not found.").into_response()
            }
        }
        Err(e) => e.into_response(),
    }
}

async fn get_static_file(
    uri: Uri,
    accept_encoding: String,
) -> Result<Response<Body>, (StatusCode, String)> {
    let (_, path) = uri.path().split_at(1); // split off the first `/`
    let mime = mime_guess::from_path(path);

    let (path, encoding) = if DEV_MODE {
        // during DEV, don't care about the precompression -> faster workflow
        (Cow::from(path), "none")
    } else if accept_encoding.contains("br") {
        (Cow::from(format!("{}.br", path)), "br")
    } else if accept_encoding.contains("gzip") {
        (Cow::from(format!("{}.gz", path)), "gzip")
    } else {
        (Cow::from(path), "none")
    };

    match Assets::get(path.as_ref()) {
        Some(content) => {
            let body = Body::from(content.data);

            let res = match DEV_MODE {
                true => Response::builder()
                    .header(
                        header::CONTENT_TYPE,
                        mime.first_or_octet_stream().as_ref(),
                    )
                    .header(header::CONTENT_ENCODING, encoding)
                    .body(body)
                    .unwrap(),
                false => Response::builder()
                    .header(header::CACHE_CONTROL, "max-age=86400")
                    .header(
                        header::CONTENT_TYPE,
                        mime.first_or_octet_stream().as_ref(),
                    )
                    .header(header::CONTENT_ENCODING, encoding)
                    .body(body)
                    .unwrap(),
            };

            Ok(res.into_response())
        }

        None => {
            eprintln!(">> Asset {} not found", path);
            for a in Assets::iter() {
                eprintln!("Available asset: {}", a);
            }

            Err((StatusCode::NOT_FOUND, "Not found".to_string()))
        }
    }
}
