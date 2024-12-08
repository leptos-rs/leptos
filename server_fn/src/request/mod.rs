use bytes::Bytes;
use futures::Stream;
use std::{borrow::Cow, future::Future};

/// Request types for Actix.
#[cfg(feature = "actix")]
pub mod actix;
/// Request types for Axum.
#[cfg(feature = "axum-no-default")]
pub mod axum;
/// Request types for the browser.
#[cfg(feature = "browser")]
pub mod browser;
#[cfg(feature = "generic")]
pub mod generic;
/// Request types for [`reqwest`].
#[cfg(feature = "reqwest")]
pub mod reqwest;

/// Represents a request as made by the client.
pub trait ClientReq<E>
where
    Self: Sized,
{
    /// The type used for URL-encoded form data in this client.
    type FormData;

    /// Attempts to construct a new `GET` request.
    fn try_new_get(
        path: &str,
        content_type: &str,
        accepts: &str,
        query: &str,
    ) -> Result<Self, E>;

    /// Attempts to construct a new `POST` request with a text body.
    fn try_new_post(
        path: &str,
        content_type: &str,
        accepts: &str,
        body: String,
    ) -> Result<Self, E>;

    /// Attempts to construct a new `POST` request with a binary body.
    fn try_new_post_bytes(
        path: &str,
        content_type: &str,
        accepts: &str,
        body: Bytes,
    ) -> Result<Self, E>;

    /// Attempts to construct a new `POST` request with form data as the body.
    fn try_new_post_form_data(
        path: &str,
        accepts: &str,
        content_type: &str,
        body: Self::FormData,
    ) -> Result<Self, E>;

    /// Attempts to construct a new `POST` request with a multipart body.
    fn try_new_multipart(
        path: &str,
        accepts: &str,
        body: Self::FormData,
    ) -> Result<Self, E>;

    /// Attempts to construct a new `POST` request with a streaming body.
    fn try_new_streaming(
        path: &str,
        accepts: &str,
        content_type: &str,
        body: impl Stream<Item = Bytes> + Send + 'static,
    ) -> Result<Self, E>;
}

/// Represents the request as received by the server.
pub trait Req<E>
where
    Self: Sized,
{
    /// Returns the query string of the request’s URL, starting after the `?`.
    fn as_query(&self) -> Option<&str>;

    /// Returns the `Content-Type` header, if any.
    fn to_content_type(&self) -> Option<Cow<'_, str>>;

    /// Returns the `Accepts` header, if any.
    fn accepts(&self) -> Option<Cow<'_, str>>;

    /// Returns the `Referer` header, if any.
    fn referer(&self) -> Option<Cow<'_, str>>;

    /// Attempts to extract the body of the request into [`Bytes`].
    fn try_into_bytes(self) -> impl Future<Output = Result<Bytes, E>> + Send;

    /// Attempts to convert the body of the request into a string.
    fn try_into_string(self) -> impl Future<Output = Result<String, E>> + Send;

    /// Attempts to convert the body of the request into a stream of bytes.
    fn try_into_stream(
        self,
    ) -> Result<impl Stream<Item = Result<Bytes, E>> + Send + 'static, E>;
}

/// A mocked request type that can be used in place of the actual server request,
/// when compiling for the browser.
pub struct BrowserMockReq;

impl<E: 'static> Req<E> for BrowserMockReq {
    fn as_query(&self) -> Option<&str> {
        unreachable!()
    }

    fn to_content_type(&self) -> Option<Cow<'_, str>> {
        unreachable!()
    }

    fn accepts(&self) -> Option<Cow<'_, str>> {
        unreachable!()
    }

    fn referer(&self) -> Option<Cow<'_, str>> {
        unreachable!()
    }
    async fn try_into_bytes(self) -> Result<Bytes, E> {
        unreachable!()
    }

    async fn try_into_string(self) -> Result<String, E> {
        unreachable!()
    }

    fn try_into_stream(
        self,
    ) -> Result<impl Stream<Item = Result<Bytes, E>> + Send, E> {
        Ok(futures::stream::once(async { unreachable!() }))
    }
}
