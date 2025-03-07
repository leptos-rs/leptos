//! This module uses platform-agnostic abstractions
//! allowing users to run server functions on a wide range of
//! platforms.
//!
//! The crates in use in this crate are:
//!
//! * `bytes`: platform-agnostic manipulation of bytes.
//! * `http`: low-dependency HTTP abstractions' *front-end*.
//!
//! # Users
//!
//! * `wasm32-wasip*` integration crate `leptos_wasi` is using this
//!   crate under the hood.

use crate::{
    error::{FromServerFnError, IntoAppError, ServerFnErrorErr},
    request::Req,
};
use bytes::Bytes;
use futures::{
    stream::{self, Stream},
    Sink, StreamExt,
};
use http::{Request, Response};
use std::borrow::Cow;

impl<E> Req<E> for Request<Bytes>
where
    E: FromServerFnError + Send,
{
    type WebsocketResponse = Response<Bytes>;

    async fn try_into_bytes(self) -> Result<Bytes, E> {
        Ok(self.into_body())
    }

    async fn try_into_string(self) -> Result<String, E> {
        String::from_utf8(self.into_body().into()).map_err(|err| {
            ServerFnErrorErr::Deserialization(err.to_string()).into_app_error()
        })
    }

    fn try_into_stream(
        self,
    ) -> Result<impl Stream<Item = Result<Bytes, E>> + Send + 'static, E> {
        Ok(stream::iter(self.into_body())
            .ready_chunks(16)
            .map(|chunk| Ok(Bytes::from(chunk))))
    }

    fn to_content_type(&self) -> Option<Cow<'_, str>> {
        self.headers()
            .get(http::header::CONTENT_TYPE)
            .map(|val| String::from_utf8_lossy(val.as_bytes()))
    }

    fn accepts(&self) -> Option<Cow<'_, str>> {
        self.headers()
            .get(http::header::ACCEPT)
            .map(|val| String::from_utf8_lossy(val.as_bytes()))
    }

    fn referer(&self) -> Option<Cow<'_, str>> {
        self.headers()
            .get(http::header::REFERER)
            .map(|val| String::from_utf8_lossy(val.as_bytes()))
    }

    fn as_query(&self) -> Option<&str> {
        self.uri().query()
    }

    async fn try_into_websocket(
        self,
    ) -> Result<
        (
            impl Stream<Item = Result<Bytes, E>> + Send + 'static,
            impl Sink<Result<Bytes, E>> + Send + 'static,
            Self::WebsocketResponse,
        ),
        E,
    > {
        Err::<
            (
                futures::stream::Once<std::future::Ready<Result<Bytes, E>>>,
                futures::sink::Drain<Result<Bytes, E>>,
                Self::WebsocketResponse,
            ),
            _,
        >(E::from_server_fn_error(crate::ServerFnErrorErr::Response(
            "Websockets are not supported on this platform.".to_string(),
        )))
    }
}
