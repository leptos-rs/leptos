use bytes::Bytes;
use futures::{Sink, Stream};
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
pub trait Req<Error, InputStreamError = Error, OutputStreamError = Error>
where
    Self: Sized,
{
    /// The response type for websockets.
    type WebsocketResponse: Send;

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
    ) -> impl Future<Output = Result<Bytes, Error>> + Send;

    /// Attempts to convert the body of the request into a string.
    fn try_into_string(
        self,
    ) -> impl Future<Output = Result<String, Error>> + Send;

    /// Attempts to convert the body of the request into a stream of bytes.
    fn try_into_stream(
        self,
    ) -> Result<impl Stream<Item = Result<Bytes, Error>> + Send + 'static, Error>;

    /// Attempts to convert the body of the request into a websocket handle.
    #[allow(clippy::type_complexity)]
    fn try_into_websocket(
        self,
    ) -> impl Future<
        Output = Result<
            (
                impl Stream<Item = Result<Bytes, InputStreamError>> + Send + 'static,
                impl Sink<Result<Bytes, OutputStreamError>> + Send + 'static,
                Self::WebsocketResponse,
            ),
            Error,
        >,
    > + Send;
}

/// A mocked request type that can be used in place of the actual server request,
/// when compiling for the browser.
pub struct BrowserMockReq;

impl<Error, InputStreamError, OutputStreamError>
    Req<Error, InputStreamError, OutputStreamError> for BrowserMockReq
where
    Error: Send + 'static,
    InputStreamError: Send + 'static,
    OutputStreamError: Send + 'static,
{
    type WebsocketResponse = crate::response::BrowserMockRes;

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
    async fn try_into_bytes(self) -> Result<Bytes, Error> {
        unreachable!()
    }

    async fn try_into_string(self) -> Result<String, Error> {
        unreachable!()
    }

    fn try_into_stream(
        self,
    ) -> Result<impl Stream<Item = Result<Bytes, Error>> + Send, Error> {
        Ok(futures::stream::once(async { unreachable!() }))
    }

    async fn try_into_websocket(
        self,
    ) -> Result<
        (
            impl Stream<Item = Result<Bytes, InputStreamError>> + Send + 'static,
            impl Sink<Result<Bytes, OutputStreamError>> + Send + 'static,
            Self::WebsocketResponse,
        ),
        Error,
    > {
        #[allow(unreachable_code)]
        Err::<
            (
                futures::stream::Once<
                    std::future::Ready<Result<Bytes, InputStreamError>>,
                >,
                futures::sink::Drain<Result<Bytes, OutputStreamError>>,
                Self::WebsocketResponse,
            ),
            _,
        >(unreachable!())
    }
}
