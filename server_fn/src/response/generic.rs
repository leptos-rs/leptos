//! This module uses platform-agnostic abstractions
//! allowing users to run server functions on a wide range of
//! platforms.
//!
//! The crates in use in this crate are:
//!
//! * `bytes`: platform-agnostic manipulation of bytes.
//! * `http`: low-dependency HTTP abstractions' *front-end*.
//!
//! # Users
//!
//! * `wasm32-wasip*` integration crate `leptos_wasi` is using this
//!   crate under the hood.

use super::{Res, TryRes};
use crate::error::{
    FromServerFnError, IntoAppError, ServerFnErrorErr,
    ServerFnErrorResponseParts, ServerFnErrorWrapper, SERVER_FN_ERROR_HEADER,
};
use bytes::Bytes;
use futures::{Stream, TryStreamExt};
use http::{header, HeaderValue, Response, StatusCode};
use std::pin::Pin;
use throw_error::Error;

/// The Body of a Response whose *execution model* can be
/// customised using the variants.
pub enum Body {
    /// The response body will be written synchronously.
    Sync(Bytes),

    /// The response body will be written asynchronously,
    /// this execution model is also known as
    /// "streaming".
    Async(Pin<Box<dyn Stream<Item = Result<Bytes, Error>> + Send + 'static>>),
}

impl From<String> for Body {
    fn from(value: String) -> Self {
        Body::Sync(Bytes::from(value))
    }
}

impl From<Bytes> for Body {
    fn from(value: Bytes) -> Self {
        Body::Sync(value)
    }
}

impl<E> TryRes<E> for Response<Body>
where
    E: Send + Sync + FromServerFnError,
{
    fn try_from_string(content_type: &str, data: String) -> Result<Self, E> {
        let builder = http::Response::builder();
        builder
            .status(200)
            .header(http::header::CONTENT_TYPE, content_type)
            .body(data.into())
            .map_err(|e| {
                ServerFnErrorErr::Response(e.to_string()).into_app_error()
            })
    }

    fn try_from_bytes(content_type: &str, data: Bytes) -> Result<Self, E> {
        let builder = http::Response::builder();
        builder
            .status(200)
            .header(http::header::CONTENT_TYPE, content_type)
            .body(Body::Sync(data))
            .map_err(|e| {
                ServerFnErrorErr::Response(e.to_string()).into_app_error()
            })
    }

    fn try_from_stream(
        content_type: &str,
        data: impl Stream<Item = Result<Bytes, Bytes>> + Send + 'static,
    ) -> Result<Self, E> {
        let builder = http::Response::builder();
        builder
            .status(200)
            .header(http::header::CONTENT_TYPE, content_type)
            .body(Body::Async(Box::pin(
                data.map_err(|e| ServerFnErrorWrapper(E::de(e)))
                    .map_err(Error::from),
            )))
            .map_err(|e| {
                ServerFnErrorErr::Response(e.to_string()).into_app_error()
            })
    }
}

impl Res for Response<Body> {
    fn error_response(path: &str, err: ServerFnErrorResponseParts) -> Self {
        let status_code = err.status_code;
        let content_type = err.content_type;
        let body = err.body;
        let mut builder = Response::builder()
            .status(status_code)
            .header(header::CONTENT_TYPE, content_type);
        // `path` originates from the request URI and could in principle
        // contain bytes that are not valid in an HTTP header value
        // (control bytes, etc.). Skip the diagnostic header rather than
        // letting the builder error propagate to an `unwrap()` panic.
        if let Ok(path_value) = HeaderValue::from_str(path) {
            builder = builder.header(SERVER_FN_ERROR_HEADER, path_value);
        }
        builder.body(Body::Sync(body.clone())).unwrap_or_else(|_| {
            let mut fallback = Response::new(Body::Sync(body));
            *fallback.status_mut() = status_code;
            fallback
        })
    }

    fn redirect(&mut self, path: &str) {
        if let Ok(path) = HeaderValue::from_str(path) {
            self.headers_mut().insert(header::LOCATION, path);
            *self.status_mut() = StatusCode::FOUND;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_parts() -> ServerFnErrorResponseParts {
        ServerFnErrorResponseParts::builder()
            .body(Bytes::from_static(b"oops"))
            .content_type("text/plain")
            .status_code(StatusCode::INTERNAL_SERVER_ERROR)
            .build()
    }

    #[test]
    fn error_response_does_not_panic_on_invalid_path_header_bytes() {
        // CR/LF are forbidden in HTTP header values. The previous
        // implementation called `.unwrap()` on the builder and panicked
        // for any path that could not be encoded as a `HeaderValue`.
        let resp = <Response<Body> as Res>::error_response(
            "/api/foo\r\nX-Evil: y",
            build_parts(),
        );
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
        // The malformed path must not have been inserted as a header.
        assert!(resp.headers().get(SERVER_FN_ERROR_HEADER).is_none());
    }

    #[test]
    fn error_response_includes_path_header_when_valid() {
        let resp =
            <Response<Body> as Res>::error_response("/api/foo", build_parts());
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(
            resp.headers()
                .get(SERVER_FN_ERROR_HEADER)
                .map(|v| v.as_bytes()),
            Some(&b"/api/foo"[..]),
        );
        assert_eq!(
            resp.headers()
                .get(header::CONTENT_TYPE)
                .map(|v| v.as_bytes()),
            Some(&b"text/plain"[..]),
        );
    }
}
