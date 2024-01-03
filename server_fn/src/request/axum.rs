use crate::{error::ServerFnError, request::Req};
use axum::body::{Body, Bytes};
use futures::{Stream, StreamExt};
use http::{header::CONTENT_TYPE, Request};
use http_body_util::BodyExt;

impl<CustErr> Req<CustErr> for Request<Body> {
    fn as_query(&self) -> Option<&str> {
        self.uri().query()
    }

    fn to_content_type(&self) -> Option<String> {
        self.headers()
            .get(CONTENT_TYPE)
            .map(|h| String::from_utf8_lossy(h.as_bytes()).to_string())
    }

    async fn try_into_bytes(self) -> Result<Bytes, ServerFnError<CustErr>> {
        let (_parts, body) = self.into_parts();

        body.collect()
            .await
            .map(|c| c.to_bytes())
            .map_err(|e| ServerFnError::Deserialization(e.to_string()))
    }

    async fn try_into_string(self) -> Result<String, ServerFnError<CustErr>> {
        let bytes = self.try_into_bytes().await?;
        let body = String::from_utf8(bytes.to_vec())
            .map_err(|e| ServerFnError::Deserialization(e.to_string()));
        body
    }

    fn try_into_stream(
        self,
    ) -> Result<
        impl Stream<Item = Result<Bytes, ServerFnError>> + Send,
        ServerFnError<CustErr>,
    > {
        Ok(self.into_body().into_data_stream().map(|chunk| {
            chunk.map_err(|e| ServerFnError::Deserialization(e.to_string()))
        }))
    }
}
