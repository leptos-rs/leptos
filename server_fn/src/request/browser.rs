use super::ClientReq;
use crate::{client::get_server_url, error::ServerFnError};
use bytes::Bytes;
use futures::{Stream, StreamExt};
pub use gloo_net::http::Request;
use js_sys::Uint8Array;
use send_wrapper::SendWrapper;
use wasm_bindgen::JsValue;
use wasm_streams::ReadableStream;
use web_sys::{FormData, UrlSearchParams};

/// A `fetch` request made in the browser.
#[derive(Debug)]
pub struct BrowserRequest(pub(crate) SendWrapper<Request>);

impl From<Request> for BrowserRequest {
    fn from(value: Request) -> Self {
        Self(SendWrapper::new(value))
    }
}

/// The `FormData` type available in the browser.
#[derive(Debug)]
pub struct BrowserFormData(pub(crate) SendWrapper<FormData>);

impl From<FormData> for BrowserFormData {
    fn from(value: FormData) -> Self {
        Self(SendWrapper::new(value))
    }
}

impl<CustErr> ClientReq<CustErr> for BrowserRequest {
    type FormData = BrowserFormData;

    fn try_new_get(
        path: &str,
        accepts: &str,
        content_type: &str,
        query: &str,
    ) -> Result<Self, ServerFnError<CustErr>> {
        let server_url = get_server_url();
        let mut url = String::with_capacity(
            server_url.len() + path.len() + 1 + query.len(),
        );
        url.push_str(server_url);
        url.push_str(path);
        url.push('?');
        url.push_str(query);
        Ok(Self(SendWrapper::new(
            Request::get(&url)
                .header("Content-Type", content_type)
                .header("Accept", accepts)
                .build()
                .map_err(|e| ServerFnError::Request(e.to_string()))?,
        )))
    }

    fn try_new_post(
        path: &str,
        accepts: &str,
        content_type: &str,
        body: String,
    ) -> Result<Self, ServerFnError<CustErr>> {
        let server_url = get_server_url();
        let mut url = String::with_capacity(server_url.len() + path.len());
        url.push_str(server_url);
        url.push_str(path);
        Ok(Self(SendWrapper::new(
            Request::post(&url)
                .header("Content-Type", content_type)
                .header("Accept", accepts)
                .body(body)
                .map_err(|e| ServerFnError::Request(e.to_string()))?,
        )))
    }

    fn try_new_post_bytes(
        path: &str,
        accepts: &str,
        content_type: &str,
        body: &[u8],
    ) -> Result<Self, ServerFnError<CustErr>> {
        let server_url = get_server_url();
        let mut url = String::with_capacity(server_url.len() + path.len());
        url.push_str(server_url);
        url.push_str(path);
        let body = Uint8Array::from(body).buffer();
        Ok(Self(SendWrapper::new(
            Request::post(&url)
                .header("Content-Type", content_type)
                .header("Accept", accepts)
                .body(body)
                .map_err(|e| ServerFnError::Request(e.to_string()))?,
        )))
    }

    fn try_new_multipart(
        path: &str,
        accepts: &str,
        body: Self::FormData,
    ) -> Result<Self, ServerFnError<CustErr>> {
        let server_url = get_server_url();
        let mut url = String::with_capacity(server_url.len() + path.len());
        url.push_str(server_url);
        url.push_str(path);
        Ok(Self(SendWrapper::new(
            Request::post(&url)
                .header("Accept", accepts)
                .body(body.0.take())
                .map_err(|e| ServerFnError::Request(e.to_string()))?,
        )))
    }

    fn try_new_post_form_data(
        path: &str,
        accepts: &str,
        content_type: &str,
        body: Self::FormData,
    ) -> Result<Self, ServerFnError<CustErr>> {
        let form_data = body.0.take();
        let url_params =
            UrlSearchParams::new_with_str_sequence_sequence(&form_data)
                .map_err(|e| {
                    ServerFnError::Serialization(e.as_string().unwrap_or_else(
                        || {
                            "Could not serialize FormData to URLSearchParams"
                                .to_string()
                        },
                    ))
                })?;
        Ok(Self(SendWrapper::new(
            Request::post(path)
                .header("Content-Type", content_type)
                .header("Accept", accepts)
                .body(url_params)
                .map_err(|e| ServerFnError::Request(e.to_string()))?,
        )))
    }

    fn try_new_streaming(
        path: &str,
        accepts: &str,
        content_type: &str,
        body: impl Stream<Item = Bytes> + 'static,
    ) -> Result<Self, ServerFnError<CustErr>> {
        let stream = ReadableStream::from_stream(body.map(|bytes| {
            let data = Uint8Array::from(bytes.as_ref());
            let data = JsValue::from(data);
            Ok(data) as Result<JsValue, JsValue>
        }));
        Ok(Self(SendWrapper::new(
            Request::post(path)
                .header("Content-Type", content_type)
                .header("Accept", accepts)
                .body(stream.into_raw())
                .map_err(|e| ServerFnError::Request(e.to_string()))?,
        )))
    }
}
