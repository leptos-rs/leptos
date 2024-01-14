use super::Res;
use crate::error::{ServerFnError, ServerFnErrorErr};
use axum::body::Body;
use bytes::Bytes;
use futures::{Stream, StreamExt};
use http::{header, HeaderValue, Response, StatusCode};
use std::fmt::{Debug, Display};

impl<CustErr> Res<CustErr> for Response<Body>
where
    CustErr: Send + Sync + Debug + Display + 'static,
{
    fn try_from_string(
        content_type: &str,
        data: String,
    ) -> Result<Self, ServerFnError<CustErr>> {
        let builder = http::Response::builder();
        builder
            .status(200)
            .header(http::header::CONTENT_TYPE, content_type)
            .body(Body::from(data))
            .map_err(|e| ServerFnError::Response(e.to_string()))
    }

    fn try_from_bytes(
        content_type: &str,
        data: Bytes,
    ) -> Result<Self, ServerFnError<CustErr>> {
        let builder = http::Response::builder();
        builder
            .status(200)
            .header(http::header::CONTENT_TYPE, content_type)
            .body(Body::from(data))
            .map_err(|e| ServerFnError::Response(e.to_string()))
    }

    fn try_from_stream(
        content_type: &str,
        data: impl Stream<Item = Result<Bytes, ServerFnError<CustErr>>>
            + Send
            + 'static,
    ) -> Result<Self, ServerFnError<CustErr>> {
        let body =
            Body::from_stream(data.map(|n| n.map_err(ServerFnErrorErr::from)));
        let builder = http::Response::builder();
        builder
            .status(200)
            .header(http::header::CONTENT_TYPE, content_type)
            .body(body)
            .map_err(|e| ServerFnError::Response(e.to_string()))
    }

    fn error_response(path: &str, err: &ServerFnError<CustErr>) -> Self {
        Response::builder()
            .status(http::StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::from(err.to_string()))
            .unwrap()
    }

    fn redirect(&mut self, path: &str) {
        if let Ok(path) = HeaderValue::from_str(path) {
            self.headers_mut().insert(header::LOCATION, path);
            *self.status_mut() = StatusCode::FOUND;
        }
    }
}
