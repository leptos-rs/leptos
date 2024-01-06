use super::ClientRes;
use crate::error::ServerFnError;
use bytes::Bytes;
use futures::{Stream, TryStreamExt};
use reqwest::Response;

impl<CustErr> ClientRes<CustErr> for Response {
    async fn try_into_string(self) -> Result<String, ServerFnError<CustErr>> {
        self.text()
            .await
            .map_err(|e| ServerFnError::Deserialization(e.to_string()))
    }

    async fn try_into_bytes(self) -> Result<Bytes, ServerFnError<CustErr>> {
        self.bytes()
            .await
            .map_err(|e| ServerFnError::Deserialization(e.to_string()))
    }

    fn try_into_stream(
        self,
    ) -> Result<
        impl Stream<Item = Result<Bytes, ServerFnError>> + Send + 'static,
        ServerFnError<CustErr>,
    > {
        Ok(self
            .bytes_stream()
            .map_err(|e| ServerFnError::Response(e.to_string())))
    }

    fn status(&self) -> u16 {
        self.status().as_u16()
    }

    fn status_text(&self) -> String {
        self.status().to_string()
    }

    fn location(&self) -> String {
        self.headers()
            .get("Location")
            .map(|value| String::from_utf8_lossy(value.as_bytes()).to_string())
            .unwrap_or_else(|| self.url().to_string())
    }

    fn has_redirect(&self) -> bool {
        self.headers().get("Location").is_some()
    }
}
