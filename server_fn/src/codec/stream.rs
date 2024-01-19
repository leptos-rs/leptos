use super::{Encoding, FromReq, FromRes, IntoReq};
use crate::{
    error::{NoCustomError, ServerFnError},
    request::{ClientReq, Req},
    response::{ClientRes, Res},
    IntoRes,
};
use bytes::Bytes;
use futures::{Stream, StreamExt};
use http::Method;
use std::{fmt::Debug, pin::Pin};

/// An encoding that represents a stream of bytes.
///
/// A server function that uses this as its output encoding should return [`ByteStream`].
///
/// ## Browser Support for Streaming Input
///
/// Browser fetch requests do not currently support full request duplexing, which
/// means that that they do begin handling responses until the full request has been sent.
/// This means that if you use a streaming input encoding, the input stream needs to
/// end before the output will begin.
///
/// Streaming requests are only allowed over HTTP2 or HTTP3.
pub struct Streaming;

impl Encoding for Streaming {
    const CONTENT_TYPE: &'static str = "application/octet-stream";
    const METHOD: Method = Method::POST;
}

impl<CustErr, T, Request> IntoReq<Streaming, Request, CustErr> for T
where
    Request: ClientReq<CustErr>,
    T: Stream<Item = Bytes> + Send + Sync + 'static,
{
    fn into_req(
        self,
        path: &str,
        accepts: &str,
    ) -> Result<Request, ServerFnError<CustErr>> {
        Request::try_new_streaming(path, accepts, Streaming::CONTENT_TYPE, self)
    }
}

impl<CustErr, T, Request> FromReq<Streaming, Request, CustErr> for T
where
    Request: Req<CustErr> + Send + 'static,
    T: From<ByteStream> + 'static,
{
    async fn from_req(req: Request) -> Result<Self, ServerFnError<CustErr>> {
        let data = req.try_into_stream()?;
        let s = ByteStream::new(data);
        Ok(s.into())
    }
}

/// A stream of bytes.
///
/// A server function can return this type if its output encoding is [`Streaming`].
///
/// ## Browser Support for Streaming Input
///
/// Browser fetch requests do not currently support full request duplexing, which
/// means that that they do begin handling responses until the full request has been sent.
/// This means that if you use a streaming input encoding, the input stream needs to
/// end before the output will begin.
///
/// Streaming requests are only allowed over HTTP2 or HTTP3.
pub struct ByteStream<CustErr = NoCustomError>(
    Pin<Box<dyn Stream<Item = Result<Bytes, ServerFnError<CustErr>>> + Send>>,
);

impl<CustErr> ByteStream<CustErr> {
    /// Consumes the wrapper, returning a stream of bytes.
    pub fn into_inner(
        self,
    ) -> impl Stream<Item = Result<Bytes, ServerFnError<CustErr>>> + Send {
        self.0
    }
}

impl<CustErr> Debug for ByteStream<CustErr> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ByteStream").finish()
    }
}

impl ByteStream {
    /// Creates a new `ByteStream` from the given stream.
    pub fn new<T>(
        value: impl Stream<Item = Result<T, ServerFnError>> + Send + 'static,
    ) -> Self
    where
        T: Into<Bytes>,
    {
        Self(Box::pin(value.map(|value| value.map(Into::into))))
    }
}

impl<S, T> From<S> for ByteStream
where
    S: Stream<Item = T> + Send + 'static,
    T: Into<Bytes>,
{
    fn from(value: S) -> Self {
        Self(Box::pin(value.map(|data| Ok(data.into()))))
    }
}

impl<CustErr, Response> IntoRes<Streaming, Response, CustErr>
    for ByteStream<CustErr>
where
    Response: Res<CustErr>,
    CustErr: 'static,
{
    async fn into_res(self) -> Result<Response, ServerFnError<CustErr>> {
        Response::try_from_stream(Streaming::CONTENT_TYPE, self.into_inner())
    }
}

impl<CustErr, Response> FromRes<Streaming, Response, CustErr> for ByteStream
where
    Response: ClientRes<CustErr> + Send,
{
    async fn from_res(res: Response) -> Result<Self, ServerFnError<CustErr>> {
        let stream = res.try_into_stream()?;
        Ok(ByteStream(Box::pin(stream)))
    }
}

/// An encoding that represents a stream of text.
///
/// A server function that uses this as its output encoding should return [`TextStream`].
///
/// ## Browser Support for Streaming Input
///
/// Browser fetch requests do not currently support full request duplexing, which
/// means that that they do begin handling responses until the full request has been sent.
/// This means that if you use a streaming input encoding, the input stream needs to
/// end before the output will begin.
///
/// Streaming requests are only allowed over HTTP2 or HTTP3.
pub struct StreamingText;

impl Encoding for StreamingText {
    const CONTENT_TYPE: &'static str = "text/plain";
    const METHOD: Method = Method::POST;
}

/// A stream of text.
///
/// A server function can return this type if its output encoding is [`StreamingText`].
///
/// ## Browser Support for Streaming Input
///
/// Browser fetch requests do not currently support full request duplexing, which
/// means that that they do begin handling responses until the full request has been sent.
/// This means that if you use a streaming input encoding, the input stream needs to
/// end before the output will begin.
///
/// Streaming requests are only allowed over HTTP2 or HTTP3.
pub struct TextStream<CustErr = NoCustomError>(
    Pin<Box<dyn Stream<Item = Result<String, ServerFnError<CustErr>>> + Send>>,
);

impl<CustErr> Debug for TextStream<CustErr> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("TextStream").finish()
    }
}

impl TextStream {
    /// Creates a new `ByteStream` from the given stream.
    pub fn new(
        value: impl Stream<Item = Result<String, ServerFnError>> + Send + 'static,
    ) -> Self {
        Self(Box::pin(value.map(|value| value.map(Into::into))))
    }
}

impl<CustErr> TextStream<CustErr> {
    /// Consumes the wrapper, returning a stream of text.
    pub fn into_inner(
        self,
    ) -> impl Stream<Item = Result<String, ServerFnError<CustErr>>> + Send {
        self.0
    }
}

impl<S, T> From<S> for TextStream
where
    S: Stream<Item = T> + Send + 'static,
    T: Into<String>,
{
    fn from(value: S) -> Self {
        Self(Box::pin(value.map(|data| Ok(data.into()))))
    }
}

impl<CustErr, T, Request> IntoReq<StreamingText, Request, CustErr> for T
where
    Request: ClientReq<CustErr>,
    T: Into<TextStream>,
{
    fn into_req(
        self,
        path: &str,
        accepts: &str,
    ) -> Result<Request, ServerFnError<CustErr>> {
        let data = self.into();
        Request::try_new_streaming(
            path,
            accepts,
            Streaming::CONTENT_TYPE,
            data.0.map(|chunk| chunk.unwrap_or_default().into()),
        )
    }
}

impl<CustErr, T, Request> FromReq<StreamingText, Request, CustErr> for T
where
    Request: Req<CustErr> + Send + 'static,
    T: From<TextStream> + 'static,
{
    async fn from_req(req: Request) -> Result<Self, ServerFnError<CustErr>> {
        let data = req.try_into_stream()?;
        let s = TextStream::new(data.map(|chunk| {
            chunk.and_then(|bytes| {
                String::from_utf8(bytes.to_vec())
                    .map_err(|e| ServerFnError::Deserialization(e.to_string()))
            })
        }));
        Ok(s.into())
    }
}

impl<CustErr, Response> IntoRes<StreamingText, Response, CustErr>
    for TextStream<CustErr>
where
    Response: Res<CustErr>,
    CustErr: 'static,
{
    async fn into_res(self) -> Result<Response, ServerFnError<CustErr>> {
        Response::try_from_stream(
            Streaming::CONTENT_TYPE,
            self.into_inner().map(|stream| stream.map(Into::into)),
        )
    }
}

impl<CustErr, Response> FromRes<StreamingText, Response, CustErr> for TextStream
where
    Response: ClientRes<CustErr> + Send,
{
    async fn from_res(res: Response) -> Result<Self, ServerFnError<CustErr>> {
        let stream = res.try_into_stream()?;
        Ok(TextStream(Box::pin(stream.map(|chunk| {
            chunk.and_then(|bytes| {
                String::from_utf8(bytes.into())
                    .map_err(|e| ServerFnError::Deserialization(e.to_string()))
            })
        }))))
    }
}
