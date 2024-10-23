use std::{pin::Pin, sync::Arc};

use bytes::Bytes;
use futures::{Stream, StreamExt};
use http::{HeaderMap, HeaderName, HeaderValue, StatusCode};
use leptos_integration_utils::ExtendResponse;
use parking_lot::RwLock;

use server_fn::response::generic::Body as ServerFnBody;
use thiserror::Error;

use wasi::http::types::{HeaderError, Headers};

/// This crate uses platform-agnostic [`http::Response`]
/// with a custom [`Body`] and convert them under the hood to
/// WASI native types.
///
/// It supports both [`Body::Sync`] and [`Body::Async`],
/// allowing you to choose between synchronous response
/// (i.e. sending the whole response) and asynchronous response
/// (i.e. streaming the response).
pub struct Response(pub http::Response<Body>);

impl Response {
    pub fn headers(&self) -> Result<Headers, ResponseError> {
        let headers = Headers::new();
        for (name, value) in self.0.headers() {
            headers.append(&name.to_string(), &Vec::from(value.as_bytes()))?;
        }
        Ok(headers)
    }
}

impl<T> From<http::Response<T>> for Response
where
    T: Into<Body>,
{
    fn from(value: http::Response<T>) -> Self {
        Self(value.map(Into::into))
    }
}

pub enum Body {
    /// The response body will be written synchronously.
    Sync(Bytes),

    /// The response body will be written asynchronously,
    /// this execution model is also known as
    /// "streaming".
    Async(
        Pin<
            Box<
                dyn Stream<Item = Result<Bytes, throw_error::Error>>
                    + Send
                    + 'static,
            >,
        >,
    ),
}

impl From<ServerFnBody> for Body {
    fn from(value: ServerFnBody) -> Self {
        match value {
            ServerFnBody::Sync(data) => Self::Sync(data),
            ServerFnBody::Async(stream) => Self::Async(stream),
        }
    }
}

/// This struct lets you define headers and override the status of the Response from an Element or a Server Function
/// Typically contained inside of a ResponseOptions. Setting this is useful for cookies and custom responses.
#[derive(Debug, Clone, Default)]
pub struct ResponseParts {
    pub headers: HeaderMap,
    pub status: Option<StatusCode>,
}

/// Allows you to override details of the HTTP response like the status code and add Headers/Cookies.
#[derive(Debug, Clone, Default)]
pub struct ResponseOptions(Arc<RwLock<ResponseParts>>);

impl ResponseOptions {
    /// A simpler way to overwrite the contents of `ResponseOptions` with a new `ResponseParts`.
    #[inline]
    pub fn overwrite(&self, parts: ResponseParts) {
        *self.0.write() = parts
    }
    /// Set the status of the returned Response.
    #[inline]
    pub fn set_status(&self, status: StatusCode) {
        self.0.write().status = Some(status);
    }
    /// Insert a header, overwriting any previous value with the same key.
    #[inline]
    pub fn insert_header(&self, key: HeaderName, value: HeaderValue) {
        self.0.write().headers.insert(key, value);
    }
    /// Append a header, leaving any header with the same key intact.
    #[inline]
    pub fn append_header(&self, key: HeaderName, value: HeaderValue) {
        self.0.write().headers.append(key, value);
    }
}

impl ExtendResponse for Response {
    type ResponseOptions = ResponseOptions;

    fn from_stream(
        stream: impl Stream<Item = String> + Send + 'static,
    ) -> Self {
        let stream = stream.map(|data| {
            Result::<Bytes, throw_error::Error>::Ok(Bytes::from(data))
        });

        Self(http::Response::new(Body::Async(Box::pin(stream))))
    }

    fn extend_response(&mut self, opt: &Self::ResponseOptions) {
        let mut opt = opt.0.write();
        if let Some(status_code) = opt.status {
            *self.0.status_mut() = status_code;
        }
        self.0
            .headers_mut()
            .extend(std::mem::take(&mut opt.headers));
    }

    fn set_default_content_type(&mut self, content_type: &str) {
        let headers = self.0.headers_mut();
        if !headers.contains_key(http::header::CONTENT_TYPE) {
            headers.insert(
                http::header::CONTENT_TYPE,
                HeaderValue::from_str(content_type).unwrap(),
            );
        }
    }
}

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum ResponseError {
    #[error("failed to parse http::Response's headers")]
    WasiHeaders(#[from] HeaderError),
}
