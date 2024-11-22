use super::{Encoding, FromReq, FromRes, Streaming};
use crate::{
    error::{FromServerFnError, ServerFnErrorErr},
    request::{ClientReq, Req},
    response::{ClientRes, Res},
    IntoReq, IntoRes,
};
use bytes::Bytes;
use futures::{Stream, StreamExt};
use http::Method;
use serde::{de::DeserializeOwned, Serialize};
use std::pin::Pin;
/// Pass arguments and receive responses as JSON in the body of a `POST` request.
pub struct Json;

impl Encoding for Json {
    const CONTENT_TYPE: &'static str = "application/json";
    const METHOD: Method = Method::POST;
}

impl<E, T, Request> IntoReq<Json, Request, E> for T
where
    Request: ClientReq<E>,
    T: Serialize + Send,
    E: FromServerFnError,
{
    fn into_req(self, path: &str, accepts: &str) -> Result<Request, E> {
        let data = serde_json::to_string(&self).map_err(|e| {
            E::from(ServerFnErrorErr::Serialization(e.to_string()))
        })?;
        Request::try_new_post(path, accepts, Json::CONTENT_TYPE, data)
    }
}

impl<E, T, Request> FromReq<Json, Request, E> for T
where
    Request: Req<E> + Send + 'static,
    T: DeserializeOwned,
    E: FromServerFnError,
{
    async fn from_req(req: Request) -> Result<Self, E> {
        let string_data = req.try_into_string().await?;
        serde_json::from_str::<Self>(&string_data)
            .map_err(|e| E::from(ServerFnErrorErr::Args(e.to_string())))
    }
}

impl<E, T, Response> IntoRes<Json, Response, E> for T
where
    Response: Res<E>,
    T: Serialize + Send,
    E: FromServerFnError,
{
    async fn into_res(self) -> Result<Response, E> {
        let data = serde_json::to_string(&self).map_err(|e| {
            E::from(ServerFnErrorErr::Serialization(e.to_string()))
        })?;
        Response::try_from_string(Json::CONTENT_TYPE, data)
    }
}

impl<E, T, Response> FromRes<Json, Response, E> for T
where
    Response: ClientRes<E> + Send,
    T: DeserializeOwned + Send,
    E: FromServerFnError,
{
    async fn from_res(res: Response) -> Result<Self, E> {
        let data = res.try_into_string().await?;
        serde_json::from_str(&data).map_err(|e| {
            E::from(ServerFnErrorErr::Deserialization(e.to_string()))
        })
    }
}

/// An encoding that represents a stream of JSON data.
///
/// A server function that uses this as its output encoding should return [`StreamingJson`]
///
/// ## Browser Support for Streaming Input
///
/// Browser fetch requests do not currently support full request duplexing, which
/// means that that they do begin handling responses until the full request has been sent.
/// This means that if you use a streaming input encoding, the input stream needs to
/// end before the output will begin.
///
/// Streaming requests are only allowed over HTTP2 or HTTP3.
pub struct StreamingJson;

impl Encoding for StreamingJson {
    // Each chunk is encoded as a JSON object, but the overall stream is not valid JSON so this uses the default stream content type
    const CONTENT_TYPE: &'static str = Streaming::CONTENT_TYPE;
    const METHOD: Method = Streaming::METHOD;
}

/// A stream of typed data encoded as JSON.
///
/// A server function can return this type if its output encoding is [`StreamingJson`].
///
/// ## Browser Support for Streaming Input
///
/// Browser fetch requests do not currently support full request duplexing, which
/// means that that they do begin handling responses until the full request has been sent.
/// This means that if you use a streaming input encoding, the input stream needs to
/// end before the output will begin.
///
/// Streaming requests are only allowed over HTTP2 or HTTP3.
pub struct JsonStream<T, E>(Pin<Box<dyn Stream<Item = Result<T, E>> + Send>>);

impl<T, E> std::fmt::Debug for JsonStream<T, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("JsonStream").finish()
    }
}

impl<T, E> JsonStream<T, E> {
    /// Creates a new `ByteStream` from the given stream.
    pub fn new(
        value: impl Stream<Item = Result<T, E>> + Send + 'static,
    ) -> Self {
        Self(Box::pin(value.map(|value| value.map(Into::into))))
    }
}

impl<T, E> JsonStream<T, E> {
    /// Consumes the wrapper, returning a stream of text.
    pub fn into_inner(self) -> impl Stream<Item = Result<T, E>> + Send {
        self.0
    }
}

impl<S, T: 'static, E: 'static> From<S> for JsonStream<T, E>
where
    S: Stream<Item = T> + Send + 'static,
{
    fn from(value: S) -> Self {
        Self(Box::pin(value.map(Ok)))
    }
}

impl<E, S, T, Request> IntoReq<StreamingJson, Request, E> for S
where
    Request: ClientReq<E>,
    S: Stream<Item = T> + Send + 'static,
    T: Serialize + 'static,
    E: FromServerFnError + Serialize,
{
    fn into_req(self, path: &str, accepts: &str) -> Result<Request, E> {
        let data: JsonStream<T, E> = self.into();
        Request::try_new_streaming(
            path,
            accepts,
            Streaming::CONTENT_TYPE,
            data.0.map(|chunk| {
                serde_json::to_vec(&chunk)
                    .unwrap_or_else(|_| Vec::new())
                    .into()
            }),
        )
    }
}

impl<E, T, S, Request> FromReq<StreamingJson, Request, E> for S
where
    Request: Req<E> + Send + 'static,
    // The additional `Stream<Item = T>` bound is never used, but it is required to avoid an error where `T` is unconstrained
    S: Stream<Item = T> + From<JsonStream<T, E>> + Send + 'static,
    T: DeserializeOwned + 'static,
    E: FromServerFnError,
{
    async fn from_req(req: Request) -> Result<Self, E> {
        let data = req.try_into_stream()?;
        let s = JsonStream::new(data.map(|chunk| {
            chunk.and_then(|bytes| {
                serde_json::from_slice(bytes.as_ref()).map_err(|e| {
                    E::from(ServerFnErrorErr::Deserialization(e.to_string()))
                })
            })
        }));
        Ok(s.into())
    }
}

impl<E, T, Response> IntoRes<StreamingJson, Response, E> for JsonStream<T, E>
where
    Response: Res<E>,
    T: Serialize + 'static,
    E: FromServerFnError,
{
    async fn into_res(self) -> Result<Response, E> {
        Response::try_from_stream(
            Streaming::CONTENT_TYPE,
            self.into_inner().map(|value| {
                serde_json::to_vec(&value?).map(Bytes::from).map_err(|e| {
                    ServerFnErrorErr::Serialization(e.to_string()).into()
                })
            }),
        )
    }
}

impl<E, T, Response> FromRes<StreamingJson, Response, E> for JsonStream<T, E>
where
    Response: ClientRes<E> + Send,
    T: DeserializeOwned,
    E: FromServerFnError,
{
    async fn from_res(res: Response) -> Result<Self, E> {
        let stream = res.try_into_stream()?;
        Ok(JsonStream::new(stream.map(|chunk| {
            chunk.and_then(|bytes| {
                serde_json::from_slice(bytes.as_ref()).map_err(|e| {
                    ServerFnErrorErr::Deserialization(e.to_string()).into()
                })
            })
        })))
    }
}
