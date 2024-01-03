#[cfg(feature = "actix")]
pub mod actix;
#[cfg(feature = "browser")]
pub mod browser;
#[cfg(feature = "axum")]
pub mod http;
#[cfg(feature = "reqwest")]
pub mod reqwest;

use crate::error::ServerFnError;
use bytes::Bytes;
use futures::Stream;
use std::future::Future;

/// Represents the response as created by the server;
pub trait Res<CustErr>
where
    Self: Sized,
{
    /// Attempts to convert a UTF-8 string into an HTTP response.
    fn try_from_string(
        content_type: &str,
        data: String,
    ) -> Result<Self, ServerFnError<CustErr>>;

    /// Attempts to convert a binary blob represented as bytes into an HTTP response.
    fn try_from_bytes(
        content_type: &str,
        data: Bytes,
    ) -> Result<Self, ServerFnError<CustErr>>;

    /// Attempts to convert a stream of bytes into an HTTP response.
    fn try_from_stream(
        content_type: &str,
        data: impl Stream<Item = Result<Bytes, ServerFnError<CustErr>>>
            + Send
            + 'static,
    ) -> Result<Self, ServerFnError<CustErr>>;

    fn error_response(err: ServerFnError<CustErr>) -> Self;
}

/// Represents the response as received by the client.
pub trait ClientRes<CustErr> {
    /// Attempts to extract a UTF-8 string from an HTTP response.
    fn try_into_string(
        self,
    ) -> impl Future<Output = Result<String, ServerFnError<CustErr>>> + Send;

    /// Attempts to extract a binary blob from an HTTP response.
    fn try_into_bytes(
        self,
    ) -> impl Future<Output = Result<Bytes, ServerFnError<CustErr>>> + Send;

    /// Attempts to extract a binary stream from an HTTP response.
    fn try_into_stream(
        self,
    ) -> Result<
        impl Stream<Item = Result<Bytes, ServerFnError>> + Send + 'static,
        ServerFnError<CustErr>,
    >;

    /// HTTP status code of the response.
    fn status(&self) -> u16;

    /// Status text for the status code.
    fn status_text(&self) -> String;

    /// The `Location` header or (if none is set), the URL of the response.
    fn location(&self) -> String;
}

/// A mocked response type that can be used in place of the actual server response,
/// when compiling for the browser.
pub struct BrowserMockRes;

impl<CustErr> Res<CustErr> for BrowserMockRes {
    fn try_from_string(
        content_type: &str,
        data: String,
    ) -> Result<Self, ServerFnError<CustErr>> {
        unreachable!()
    }

    fn try_from_bytes(
        content_type: &str,
        data: Bytes,
    ) -> Result<Self, ServerFnError<CustErr>> {
        unreachable!()
    }

    fn error_response(err: ServerFnError<CustErr>) -> Self {
        unreachable!()
    }

    fn try_from_stream(
        content_type: &str,
        data: impl Stream<Item = Result<Bytes, ServerFnError<CustErr>>>,
    ) -> Result<Self, ServerFnError<CustErr>> {
        todo!()
    }
}
