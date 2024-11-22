use super::Res;
use crate::error::{
    FromServerFnError, ServerFnErrorErr, ServerFnErrorSerde,
    ServerFnErrorWrapper, SERVER_FN_ERROR_HEADER,
};
use axum::body::Body;
use bytes::Bytes;
use futures::{Stream, TryStreamExt};
use http::{header, HeaderValue, Response, StatusCode};

impl<E> Res<E> for Response<Body>
where
    E: Send + Sync + FromServerFnError,
{
    fn try_from_string(content_type: &str, data: String) -> Result<Self, E> {
        let builder = http::Response::builder();
        builder
            .status(200)
            .header(http::header::CONTENT_TYPE, content_type)
            .body(Body::from(data))
            .map_err(|e| ServerFnErrorErr::Response(e.to_string()).into())
    }

    fn try_from_bytes(content_type: &str, data: Bytes) -> Result<Self, E> {
        let builder = http::Response::builder();
        builder
            .status(200)
            .header(http::header::CONTENT_TYPE, content_type)
            .body(Body::from(data))
            .map_err(|e| ServerFnErrorErr::Response(e.to_string()).into())
    }

    fn try_from_stream(
        content_type: &str,
        data: impl Stream<Item = Result<Bytes, E>> + Send + 'static,
    ) -> Result<Self, E> {
        let body = Body::from_stream(data.map_err(|e| ServerFnErrorWrapper(e)));
        let builder = http::Response::builder();
        builder
            .status(200)
            .header(http::header::CONTENT_TYPE, content_type)
            .body(body)
            .map_err(|e| E::from(ServerFnErrorErr::Response(e.to_string())))
    }

    fn error_response(path: &str, err: &E) -> Self {
        Response::builder()
            .status(http::StatusCode::INTERNAL_SERVER_ERROR)
            .header(SERVER_FN_ERROR_HEADER, path)
            .body(err.ser().unwrap_or_else(|_| err.to_string()).into())
            .unwrap()
    }

    fn redirect(&mut self, path: &str) {
        if let Ok(path) = HeaderValue::from_str(path) {
            self.headers_mut().insert(header::LOCATION, path);
            *self.status_mut() = StatusCode::FOUND;
        }
    }
}
