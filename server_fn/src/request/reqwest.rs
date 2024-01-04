use super::ClientReq;
use crate::error::ServerFnError;
use bytes::Bytes;
use once_cell::sync::Lazy;
use reqwest::header::{ACCEPT, CONTENT_TYPE};
pub use reqwest::{multipart::Form, Client, Method, Request, Url};
use std::sync::OnceLock;

pub(crate) static CLIENT: Lazy<Client> = Lazy::new(Client::new);
static ROOT_URL: OnceLock<&'static str> = OnceLock::new();

/// Set the root server url that all server function paths are relative to for the client.
///
/// If this is not set, it defaults to the origin.
pub fn set_server_url(url: &'static str) {
    ROOT_URL.set(url).unwrap();
}

fn get_server_url() -> &'static str {
    ROOT_URL
        .get()
        .expect("Call `set_root_url` before calling a server function.")
}

impl<CustErr> ClientReq<CustErr> for Request {
    type FormData = Form;

    fn try_new_get(
        path: &str,
        accepts: &str,
        content_type: &str,
        query: &str,
    ) -> Result<Self, ServerFnError<CustErr>> {
        let url = format!("{}{}", get_server_url(), path);
        let mut url = Url::try_from(url.as_str())
            .map_err(|e| ServerFnError::Request(e.to_string()))?;
        url.set_query(Some(query));
        let req = CLIENT
            .get(url)
            .header(CONTENT_TYPE, content_type)
            .header(ACCEPT, accepts)
            .build()
            .map_err(|e| ServerFnError::Request(e.to_string()))?;
        Ok(req)
    }

    fn try_new_post(
        path: &str,
        accepts: &str,
        content_type: &str,
        body: String,
    ) -> Result<Self, ServerFnError<CustErr>> {
        let url = format!("{}{}", get_server_url(), path);
        CLIENT
            .post(url)
            .header(CONTENT_TYPE, content_type)
            .header(ACCEPT, accepts)
            .body(body)
            .build()
            .map_err(|e| ServerFnError::Request(e.to_string()))
    }

    fn try_new_post_bytes(
        path: &str,
        accepts: &str,
        content_type: &str,
        body: Bytes,
    ) -> Result<Self, ServerFnError<CustErr>> {
        let url = format!("{}{}", get_server_url(), path);
        CLIENT
            .post(url)
            .header(CONTENT_TYPE, content_type)
            .header(ACCEPT, accepts)
            .body(body)
            .build()
            .map_err(|e| ServerFnError::Request(e.to_string()))
    }

    fn try_new_multipart(
        path: &str,
        accepts: &str,
        body: Self::FormData,
    ) -> Result<Self, ServerFnError<CustErr>> {
        CLIENT
            .post(path)
            .header(ACCEPT, accepts)
            .multipart(body)
            .build()
            .map_err(|e| ServerFnError::Request(e.to_string()))
    }
}
