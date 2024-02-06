use crate::error::ServerFnError;
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
/// Request types for [`reqwest`].
#[cfg(feature = "reqwest")]
pub mod reqwest;

/// Represents a request as made by the client.
pub trait ClientReq<CustErr>
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
    ) -> Result<Self, ServerFnError<CustErr>>;

    /// Attempts to construct a new `POST` request with a text body.
    fn try_new_post(
        path: &str,
        content_type: &str,
        accepts: &str,
        body: String,
    ) -> Result<Self, ServerFnError<CustErr>>;

    /// Attempts to construct a new `POST` request with a binary body.
    fn try_new_post_bytes(
        path: &str,
        content_type: &str,
        accepts: &str,
        body: Bytes,
    ) -> Result<Self, ServerFnError<CustErr>>;

    /// Attempts to construct a new `POST` request with form data as the body.
    fn try_new_post_form_data(
        path: &str,
        accepts: &str,
        content_type: &str,
        body: Self::FormData,
    ) -> Result<Self, ServerFnError<CustErr>>;

    /// Attempts to construct a new `POST` request with a multipart body.
    fn try_new_multipart(
        path: &str,
        accepts: &str,
        body: Self::FormData,
    ) -> Result<Self, ServerFnError<CustErr>>;

    /// Attempts to construct a new `POST` request with a streaming body.
    fn try_new_streaming(
        path: &str,
        accepts: &str,
        content_type: &str,
        body: impl Stream<Item = Bytes> + Send + 'static,
    ) -> Result<Self, ServerFnError<CustErr>>;
}

/// Represents the request as received by the server.
pub trait Req<CustErr>
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
    fn try_into_bytes(
        self,
    ) -> impl Future<Output = Result<Bytes, ServerFnError<CustErr>>> + Send;

    /// Attempts to convert the body of the request into a string.
    fn try_into_string(
        self,
    ) -> impl Future<Output = Result<String, ServerFnError<CustErr>>> + Send;

    /// Attempts to convert the body of the request into a stream of bytes.
    fn try_into_stream(
        self,
    ) -> Result<
        impl Stream<Item = Result<Bytes, ServerFnError>> + Send + 'static,
        ServerFnError<CustErr>,
    >;
}

/// A mocked request type that can be used in place of the actual server request,
/// when compiling for the browser.
pub struct BrowserMockReq;

impl<CustErr> Req<CustErr> for BrowserMockReq
where
    CustErr: 'static,
{
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
    async fn try_into_bytes(self) -> Result<Bytes, ServerFnError<CustErr>> {
        unreachable!()
    }

    async fn try_into_string(self) -> Result<String, ServerFnError<CustErr>> {
        unreachable!()
    }

    fn try_into_stream(
        self,
    ) -> Result<
        impl Stream<Item = Result<Bytes, ServerFnError>> + Send,
        ServerFnError<CustErr>,
    > {
        Ok(futures::stream::once(async { unreachable!() }))
    }
}
