use crate::error::ServerFnError;
use bytes::Bytes;
use futures::Stream;
use std::future::Future;

#[cfg(feature = "actix")]
pub mod actix;
#[cfg(feature = "axum")]
pub mod axum;
#[cfg(feature = "browser")]
pub mod browser;
#[cfg(feature = "reqwest")]
pub mod reqwest;

/// Represents a request as made by the client.
pub trait ClientReq<CustErr>
where
    Self: Sized,
{
    type FormData;

    fn try_new_get(
        path: &str,
        content_type: &str,
        accepts: &str,
        query: &str,
    ) -> Result<Self, ServerFnError<CustErr>>;

    fn try_new_post(
        path: &str,
        content_type: &str,
        accepts: &str,
        body: String,
    ) -> Result<Self, ServerFnError<CustErr>>;

    fn try_new_post_bytes(
        path: &str,
        content_type: &str,
        accepts: &str,
        body: Bytes,
    ) -> Result<Self, ServerFnError<CustErr>>;

    fn try_new_multipart(
        path: &str,
        accepts: &str,
        body: Self::FormData,
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
    fn to_content_type(&self) -> Option<String>;

    /// Attempts to extract the body of the request into [`Bytes`].
    fn try_into_bytes(
        self,
    ) -> impl Future<Output = Result<Bytes, ServerFnError<CustErr>>> + Send;

    /// Attempts to convert the body of the request into a string.
    fn try_into_string(
        self,
    ) -> impl Future<Output = Result<String, ServerFnError<CustErr>>> + Send;

    /// Attempts to convert the body of the request into a string.
    fn try_into_stream(
        self,
    ) -> Result<
        impl Stream<Item = Result<Bytes, ServerFnError>> + Send,
        ServerFnError<CustErr>,
    >;
}

/// A mocked request type that can be used in place of the actual server request,
/// when compiling for the browser.
pub struct BrowserMockReq;

impl<CustErr> Req<CustErr> for BrowserMockReq {
    fn as_query(&self) -> Option<&str> {
        unreachable!()
    }

    fn to_content_type(&self) -> Option<String> {
        unreachable!()
    }

    fn try_into_bytes(
        self,
    ) -> impl Future<Output = Result<Bytes, ServerFnError<CustErr>>> + Send
    {
        async { unreachable!() }
    }

    fn try_into_string(
        self,
    ) -> impl Future<Output = Result<String, ServerFnError<CustErr>>> + Send
    {
        async { unreachable!() }
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
