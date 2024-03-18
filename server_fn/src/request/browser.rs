use super::ClientReq;
use crate::{client::get_server_url, error::ServerFnError};
use bytes::Bytes;
use futures::{Stream, StreamExt};
pub use gloo_net::http::Request;
use js_sys::{Reflect, Uint8Array};
use send_wrapper::SendWrapper;
use std::ops::{Deref, DerefMut};
use thiserror::Error;
use wasm_bindgen::{JsCast, JsValue};
use wasm_streams::ReadableStream;
use web_sys::{
    AbortController, AbortSignal, Event, FormData, Headers, RequestInit,
    UrlSearchParams,
};

/// A `fetch` request made in the browser.
#[derive(Debug)]
pub struct BrowserRequest(pub(crate) SendWrapper<RequestInner>);

#[derive(Debug)]
pub(crate) struct RequestInner {
    pub(crate) request: Request,
    pub(crate) abort_ctrl: Option<AbortOnDrop>,
}

#[derive(Debug)]
pub(crate) struct AbortOnDrop(AbortController);

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

impl From<BrowserRequest> for Request {
    fn from(value: BrowserRequest) -> Self {
        value.0.take().request
    }
}

impl From<BrowserRequest> for web_sys::Request {
    fn from(value: BrowserRequest) -> Self {
        value.0.take().request.into()
    }
}

impl Deref for BrowserRequest {
    type Target = Request;

    fn deref(&self) -> &Self::Target {
        &self.0.deref().request
    }
}

impl DerefMut for BrowserRequest {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0.deref_mut().request
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

fn abort_signal() -> (Option<AbortOnDrop>, Option<AbortSignal>) {
    let ctrl = AbortController::new().ok();
    let signal = ctrl.as_ref().map(|ctrl| ctrl.signal());
    (ctrl.map(AbortOnDrop), signal)
}

impl<CustErr> ClientReq<CustErr> for BrowserRequest {
    type FormData = BrowserFormData;

    fn try_new_get(
        path: &str,
        accepts: &str,
        content_type: &str,
        query: &str,
    ) -> Result<Self, ServerFnError<CustErr>> {
        let (abort_ctrl, abort_signal) = abort_signal();
        let server_url = get_server_url();
        let mut url = String::with_capacity(
            server_url.len() + path.len() + 1 + query.len(),
        );
        url.push_str(server_url);
        url.push_str(path);
        url.push('?');
        url.push_str(query);
        Ok(Self(SendWrapper::new(RequestInner {
            request: Request::get(&url)
                .header("Content-Type", content_type)
                .header("Accept", accepts)
                .abort_signal(abort_signal.as_ref())
                .build()
                .map_err(|e| ServerFnError::Request(e.to_string()))?,
            abort_ctrl,
        })))
    }

    fn try_new_post(
        path: &str,
        accepts: &str,
        content_type: &str,
        body: String,
    ) -> Result<Self, ServerFnError<CustErr>> {
        let (abort_ctrl, abort_signal) = abort_signal();
        let server_url = get_server_url();
        let mut url = String::with_capacity(server_url.len() + path.len());
        url.push_str(server_url);
        url.push_str(path);
        Ok(Self(SendWrapper::new(RequestInner {
            request: Request::post(&url)
                .header("Content-Type", content_type)
                .header("Accept", accepts)
                .abort_signal(abort_signal.as_ref())
                .body(body)
                .map_err(|e| ServerFnError::Request(e.to_string()))?,
            abort_ctrl,
        })))
    }

    fn try_new_post_bytes(
        path: &str,
        accepts: &str,
        content_type: &str,
        body: Bytes,
    ) -> Result<Self, ServerFnError<CustErr>> {
        let (abort_ctrl, abort_signal) = abort_signal();
        let server_url = get_server_url();
        let mut url = String::with_capacity(server_url.len() + path.len());
        url.push_str(server_url);
        url.push_str(path);
        let body: &[u8] = &body;
        let body = Uint8Array::from(body).buffer();
        Ok(Self(SendWrapper::new(RequestInner {
            request: Request::post(&url)
                .header("Content-Type", content_type)
                .header("Accept", accepts)
                .abort_signal(abort_signal.as_ref())
                .body(body)
                .map_err(|e| ServerFnError::Request(e.to_string()))?,
            abort_ctrl,
        })))
    }

    fn try_new_multipart(
        path: &str,
        accepts: &str,
        body: Self::FormData,
    ) -> Result<Self, ServerFnError<CustErr>> {
        let (abort_ctrl, abort_signal) = abort_signal();
        let server_url = get_server_url();
        let mut url = String::with_capacity(server_url.len() + path.len());
        url.push_str(server_url);
        url.push_str(path);
        Ok(Self(SendWrapper::new(RequestInner {
            request: Request::post(&url)
                .header("Accept", accepts)
                .abort_signal(abort_signal.as_ref())
                .body(body.0.take())
                .map_err(|e| ServerFnError::Request(e.to_string()))?,
            abort_ctrl,
        })))
    }

    fn try_new_post_form_data(
        path: &str,
        accepts: &str,
        content_type: &str,
        body: Self::FormData,
    ) -> Result<Self, ServerFnError<CustErr>> {
        let (abort_ctrl, abort_signal) = abort_signal();
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
        Ok(Self(SendWrapper::new(RequestInner {
            request: Request::post(path)
                .header("Content-Type", content_type)
                .header("Accept", accepts)
                .abort_signal(abort_signal.as_ref())
                .body(url_params)
                .map_err(|e| ServerFnError::Request(e.to_string()))?,
            abort_ctrl,
        })))
    }

    fn try_new_streaming(
        path: &str,
        accepts: &str,
        content_type: &str,
        body: impl Stream<Item = Bytes> + 'static,
    ) -> Result<Self, ServerFnError<CustErr>> {
        // TODO abort signal
        let req = streaming_request(path, accepts, content_type, body)
            .map_err(|e| ServerFnError::Request(format!("{e:?}")))?;
        Ok(Self(SendWrapper::new(RequestInner {
            request: req,
            abort_ctrl: None,
        })))
    }
}

fn streaming_request(
    path: &str,
    accepts: &str,
    content_type: &str,
    body: impl Stream<Item = Bytes> + 'static,
) -> Result<Request, JsValue> {
    let (abort_ctrl, abort_signal) = abort_signal();
    let stream = ReadableStream::from_stream(body.map(|bytes| {
        let data = Uint8Array::from(bytes.as_ref());
        let data = JsValue::from(data);
        Ok(data) as Result<JsValue, JsValue>
    }))
    .into_raw();
    let headers = Headers::new()?;
    headers.append("Content-Type", content_type)?;
    headers.append("Accept", accepts)?;
    let mut init = RequestInit::new();
    init.headers(&headers).method("POST").body(Some(&stream));

    // Chrome requires setting `duplex: "half"` on streaming requests
    Reflect::set(
        &init,
        &JsValue::from_str("duplex"),
        &JsValue::from_str("half"),
    )?;
    let req = web_sys::Request::new_with_str_and_init(path, &init)?;
    Ok(Request::from(req))
}
