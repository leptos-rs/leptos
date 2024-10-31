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

use crate::request::Req;
use bytes::Bytes;
use futures::{
    stream::{self, Stream},
    StreamExt,
};
use http::Request;
use std::borrow::Cow;

impl<CustErr> Req<CustErr> for Request<Bytes>
where
    CustErr: 'static,
{
    async fn try_into_bytes(
        self,
    ) -> Result<Bytes, crate::ServerFnError<CustErr>> {
        Ok(self.into_body())
    }

    async fn try_into_string(
        self,
    ) -> Result<String, crate::ServerFnError<CustErr>> {
        String::from_utf8(self.into_body().into()).map_err(|err| {
            crate::ServerFnError::Deserialization(err.to_string())
        })
    }

    fn try_into_stream(
        self,
    ) -> Result<
        impl Stream<Item = Result<Bytes, crate::ServerFnError>> + Send + 'static,
        crate::ServerFnError<CustErr>,
    > {
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
}
