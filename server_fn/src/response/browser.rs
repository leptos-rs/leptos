use super::ClientRes;
use crate::{error::ServerFnError, redirect::REDIRECT_HEADER};
use bytes::Bytes;
use futures::{Stream, StreamExt};
pub use gloo_net::http::Response;
use js_sys::Uint8Array;
use send_wrapper::SendWrapper;
use std::future::Future;
use wasm_bindgen::JsCast;
use wasm_streams::ReadableStream;

/// The response to a `fetch` request made in the browser.
pub struct BrowserResponse(pub(crate) SendWrapper<Response>);

impl<CustErr> ClientRes<CustErr> for BrowserResponse {
    fn try_into_string(
        self,
    ) -> impl Future<Output = Result<String, ServerFnError<CustErr>>> + Send
    {
        // the browser won't send this async work between threads (because it's single-threaded)
        // so we can safely wrap this
        SendWrapper::new(async move {
            self.0
                .text()
                .await
                .map_err(|e| ServerFnError::Deserialization(e.to_string()))
        })
    }

    fn try_into_bytes(
        self,
    ) -> impl Future<Output = Result<Bytes, ServerFnError<CustErr>>> + Send
    {
        // the browser won't send this async work between threads (because it's single-threaded)
        // so we can safely wrap this
        SendWrapper::new(async move {
            self.0
                .binary()
                .await
                .map(Bytes::from)
                .map_err(|e| ServerFnError::Deserialization(e.to_string()))
        })
    }

    fn try_into_stream(
        self,
    ) -> Result<
        impl Stream<Item = Result<Bytes, ServerFnError>> + Send + 'static,
        ServerFnError<CustErr>,
    > {
        let stream = ReadableStream::from_raw(self.0.body().unwrap())
            .into_stream()
            .map(|data| {
                let data = data.unwrap().unchecked_into::<Uint8Array>();
                let mut buf = Vec::new();
                let length = data.length();
                buf.resize(length as usize, 0);
                data.copy_to(&mut buf);
                Ok(Bytes::from(buf))
            });
        Ok(SendWrapper::new(stream))
    }

    fn status(&self) -> u16 {
        self.0.status()
    }

    fn status_text(&self) -> String {
        self.0.status_text()
    }

    fn location(&self) -> String {
        self.0
            .headers()
            .get("Location")
            .unwrap_or_else(|| self.0.url())
    }

    fn has_redirect(&self) -> bool {
        self.0.headers().get(REDIRECT_HEADER).is_some()
    }
}
