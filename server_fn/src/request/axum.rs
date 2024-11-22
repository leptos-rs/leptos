use crate::{
    error::{FromServerFnError, ServerFnErrorErr},
    request::Req,
};
use axum::body::{Body, Bytes};
use futures::{Stream, StreamExt};
use http::{
    header::{ACCEPT, CONTENT_TYPE, REFERER},
    Request,
};
use http_body_util::BodyExt;
use std::borrow::Cow;

impl<E> Req<E> for Request<Body>
where
    E: FromServerFnError,
{
    fn as_query(&self) -> Option<&str> {
        self.uri().query()
    }

    fn to_content_type(&self) -> Option<Cow<'_, str>> {
        self.headers()
            .get(CONTENT_TYPE)
            .map(|h| String::from_utf8_lossy(h.as_bytes()))
    }

    fn accepts(&self) -> Option<Cow<'_, str>> {
        self.headers()
            .get(ACCEPT)
            .map(|h| String::from_utf8_lossy(h.as_bytes()))
    }

    fn referer(&self) -> Option<Cow<'_, str>> {
        self.headers()
            .get(REFERER)
            .map(|h| String::from_utf8_lossy(h.as_bytes()))
    }

    async fn try_into_bytes(self) -> Result<Bytes, E> {
        let (_parts, body) = self.into_parts();

        body.collect().await.map(|c| c.to_bytes()).map_err(|e| {
            E::from(ServerFnErrorErr::Deserialization(e.to_string()))
        })
    }

    async fn try_into_string(self) -> Result<String, E> {
        // from:
        let (_parts, body) = self.into_parts();

        let bytes =
            body.collect().await.map(|c| c.to_bytes()).map_err(|e| {
                E::from(ServerFnErrorErr::Deserialization(e.to_string()))
            })?;
        // here: is the same as try_into_bytes, but rustc complains if `self.try_into_bytes()` called.

        String::from_utf8(bytes.to_vec()).map_err(|e| {
            E::from(ServerFnErrorErr::Deserialization(e.to_string()))
        })
    }

    fn try_into_stream(
        self,
    ) -> Result<impl Stream<Item = Result<Bytes, E>> + Send + 'static, E> {
        Ok(self.into_body().into_data_stream().map(|chunk| {
            chunk.map_err(|e| {
                E::from(ServerFnErrorErr::Deserialization(e.to_string()))
            })
        }))
    }
}
