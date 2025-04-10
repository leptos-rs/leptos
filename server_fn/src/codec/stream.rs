use super::{Encoding, FromReq, FromRes, IntoReq};
use crate::{
    error::{FromServerFnError, ServerFnErrorErr},
    request::{ClientReq, Req},
    response::{ClientRes, TryRes},
    ContentType, IntoRes, ServerFnError,
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

impl ContentType for Streaming {
    const CONTENT_TYPE: &'static str = "application/octet-stream";
}

impl Encoding for Streaming {
    const METHOD: Method = Method::POST;
}

impl<E, T, Request> IntoReq<Streaming, Request, E> for T
where
    Request: ClientReq<E>,
    T: Stream<Item = Bytes> + Send + Sync + 'static,
{
    fn into_req(self, path: &str, accepts: &str) -> Result<Request, E> {
        Request::try_new_post_streaming(
            path,
            accepts,
            Streaming::CONTENT_TYPE,
            self,
        )
    }
}

impl<E, T, Request> FromReq<Streaming, Request, E> for T
where
    Request: Req<E> + Send + 'static,
    T: From<ByteStream> + 'static,
{
    async fn from_req(req: Request) -> Result<Self, E> {
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
pub struct ByteStream(Pin<Box<dyn Stream<Item = Result<Bytes, Bytes>> + Send>>);

impl ByteStream {
    /// Consumes the wrapper, returning a stream of bytes.
    pub fn into_inner(self) -> impl Stream<Item = Result<Bytes, Bytes>> + Send {
        self.0
    }
}

impl Debug for ByteStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ByteStream").finish()
    }
}

impl ByteStream {
    /// Creates a new `ByteStream` from the given stream.
    pub fn new<T, E>(
        value: impl Stream<Item = Result<T, E>> + Send + 'static,
    ) -> Self
    where
        T: Into<Bytes>,
        E: Into<Bytes>,
    {
        Self(Box::pin(
            value.map(|value| value.map(Into::into).map_err(Into::into)),
        ))
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

impl<E, Response> IntoRes<Streaming, Response, E> for ByteStream
where
    Response: TryRes<E>,
    E: 'static,
{
    async fn into_res(self) -> Result<Response, E> {
        Response::try_from_stream(Streaming::CONTENT_TYPE, self.into_inner())
    }
}

impl<E, Response> FromRes<Streaming, Response, E> for ByteStream
where
    Response: ClientRes<E> + Send,
{
    async fn from_res(res: Response) -> Result<Self, E> {
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

impl ContentType for StreamingText {
    const CONTENT_TYPE: &'static str = "text/plain";
}

impl Encoding for StreamingText {
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
pub struct TextStream<E = ServerFnError>(
    Pin<Box<dyn Stream<Item = Result<String, E>> + Send>>,
);

impl<E: FromServerFnError> Debug for TextStream<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("TextStream").finish()
    }
}

impl<E: FromServerFnError> TextStream<E> {
    /// Creates a new `ByteStream` from the given stream.
    pub fn new(
        value: impl Stream<Item = Result<String, E>> + Send + 'static,
    ) -> Self {
        Self(Box::pin(value.map(|value| value)))
    }
}

impl<E: FromServerFnError> TextStream<E> {
    /// Consumes the wrapper, returning a stream of text.
    pub fn into_inner(self) -> impl Stream<Item = Result<String, E>> + Send {
        self.0
    }
}

impl<E, S, T> From<S> for TextStream<E>
where
    S: Stream<Item = T> + Send + 'static,
    T: Into<String>,
    E: FromServerFnError,
{
    fn from(value: S) -> Self {
        Self(Box::pin(value.map(|data| Ok(data.into()))))
    }
}

impl<E, T, Request> IntoReq<StreamingText, Request, E> for T
where
    Request: ClientReq<E>,
    T: Into<TextStream<E>>,
    E: FromServerFnError,
{
    fn into_req(self, path: &str, accepts: &str) -> Result<Request, E> {
        let data = self.into();
        Request::try_new_post_streaming(
            path,
            accepts,
            Streaming::CONTENT_TYPE,
            data.0.map(|chunk| chunk.unwrap_or_default().into()),
        )
    }
}

impl<E, T, Request> FromReq<StreamingText, Request, E> for T
where
    Request: Req<E> + Send + 'static,
    T: From<TextStream<E>> + 'static,
    E: FromServerFnError,
{
    async fn from_req(req: Request) -> Result<Self, E> {
        let data = req.try_into_stream()?;
        let s = TextStream::new(data.map(|chunk| match chunk {
            Ok(bytes) => {
                let de = String::from_utf8(bytes.to_vec()).map_err(|e| {
                    E::from_server_fn_error(ServerFnErrorErr::Deserialization(
                        e.to_string(),
                    ))
                })?;
                Ok(de)
            }
            Err(bytes) => Err(E::de(bytes)),
        }));
        Ok(s.into())
    }
}

impl<E, Response> IntoRes<StreamingText, Response, E> for TextStream<E>
where
    Response: TryRes<E>,
    E: FromServerFnError,
{
    async fn into_res(self) -> Result<Response, E> {
        Response::try_from_stream(
            Streaming::CONTENT_TYPE,
            self.into_inner()
                .map(|stream| stream.map(Into::into).map_err(|e| e.ser())),
        )
    }
}

impl<E, Response> FromRes<StreamingText, Response, E> for TextStream<E>
where
    Response: ClientRes<E> + Send,
    E: FromServerFnError,
{
    async fn from_res(res: Response) -> Result<Self, E> {
        let stream = res.try_into_stream()?;
        Ok(TextStream(Box::pin(stream.map(|chunk| match chunk {
            Ok(bytes) => {
                let de = String::from_utf8(bytes.into()).map_err(|e| {
                    E::from_server_fn_error(ServerFnErrorErr::Deserialization(
                        e.to_string(),
                    ))
                })?;
                Ok(de)
            }
            Err(bytes) => Err(E::de(bytes)),
        }))))
    }
}
