use std::pin::Pin;

use super::{Encoding, FromRes};
use crate::error::{NoCustomError, ServerFnError};
use crate::response::{ClientRes, Res};
use crate::IntoRes;
use bytes::Bytes;
use futures::{Stream, StreamExt};

pub struct Streaming;

impl Encoding for Streaming {
    const CONTENT_TYPE: &'static str = "application/octet-stream";
}

/* impl<CustErr, T, Request> IntoReq<CustErr, Request, ByteStream> for T
where
    Request: ClientReq<CustErr>,
    T: Stream<Item = Bytes> + Send,
{
    fn into_req(self, path: &str, accepts: &str) -> Result<Request, ServerFnError<CustErr>> {
        Request::try_new_stream(path, ByteStream::CONTENT_TYPE, self)
    }
} */

/* impl<CustErr, T, Request> FromReq<CustErr, Request, ByteStream> for T
where
    Request: Req<CustErr> + Send + 'static,
    T: Stream<Item = Bytes> + Send,
{
    async fn from_req(req: Request) -> Result<Self, ServerFnError<CustErr>> {
        req.try_into_stream().await
    }
} */

pub struct ByteStream<CustErr = NoCustomError>(
    Pin<Box<dyn Stream<Item = Result<Bytes, ServerFnError<CustErr>>> + Send>>,
);

impl<CustErr> ByteStream<CustErr> {
    pub fn into_inner(self) -> impl Stream<Item = Result<Bytes, ServerFnError<CustErr>>> + Send {
        self.0
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

impl<CustErr, Response> IntoRes<CustErr, Response, Streaming> for ByteStream<CustErr>
where
    Response: Res<CustErr>,
    CustErr: 'static,
{
    async fn into_res(self) -> Result<Response, ServerFnError<CustErr>> {
        Response::try_from_stream(Streaming::CONTENT_TYPE, self.into_inner())
    }
}

impl<CustErr, Response> FromRes<CustErr, Response, Streaming> for ByteStream
where
    Response: ClientRes<CustErr> + Send,
{
    async fn from_res(res: Response) -> Result<Self, ServerFnError<CustErr>> {
        let stream = res.try_into_stream()?;
        Ok(ByteStream(Box::pin(stream)))
    }
}

pub struct StreamingText;

impl Encoding for StreamingText {
    const CONTENT_TYPE: &'static str = "text/plain";
}

pub struct TextStream<CustErr = NoCustomError>(
    Pin<Box<dyn Stream<Item = Result<String, ServerFnError<CustErr>>> + Send>>,
);

impl<CustErr> TextStream<CustErr> {
    pub fn into_inner(self) -> impl Stream<Item = Result<String, ServerFnError<CustErr>>> + Send {
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

impl<CustErr, Response> IntoRes<CustErr, Response, StreamingText> for TextStream<CustErr>
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

impl<CustErr, Response> FromRes<CustErr, Response, StreamingText> for TextStream
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
