use super::ClientReq;
use crate::error::ServerFnError;
use bytes::Bytes;
pub use gloo_net::http::Request;
use js_sys::Uint8Array;
use send_wrapper::SendWrapper;
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
        let mut url = path.to_owned();
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
        Ok(Self(SendWrapper::new(
            Request::post(path)
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
        body: Bytes,
    ) -> Result<Self, ServerFnError<CustErr>> {
        let body: &[u8] = &body;
        let body = Uint8Array::from(body).buffer();
        Ok(Self(SendWrapper::new(
            Request::post(path)
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
        Ok(Self(SendWrapper::new(
            Request::post(path)
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
}
