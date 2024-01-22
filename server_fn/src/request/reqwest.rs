use super::ClientReq;
use crate::{client::get_server_url, error::ServerFnError};
use bytes::Bytes;
use futures::Stream;
use once_cell::sync::Lazy;
use reqwest::header::{ACCEPT, CONTENT_TYPE};
pub use reqwest::{multipart::Form, Client, Method, Request, Url};

pub(crate) static CLIENT: Lazy<Client> = Lazy::new(Client::new);

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

    fn try_new_post_form_data(
        path: &str,
        accepts: &str,
        content_type: &str,
        body: Self::FormData,
    ) -> Result<Self, ServerFnError<CustErr>> {
        CLIENT
            .post(path)
            .header(CONTENT_TYPE, content_type)
            .header(ACCEPT, accepts)
            .multipart(body)
            .build()
            .map_err(|e| ServerFnError::Request(e.to_string()))
    }

    fn try_new_streaming(
        _path: &str,
        _accepts: &str,
        _content_type: &str,
        _body: impl Stream<Item = Bytes> + 'static,
    ) -> Result<Self, ServerFnError<CustErr>> {
        todo!("Streaming requests are not yet implemented for reqwest.")
        // We run into a fundamental issue here.
        // To be a reqwest body, the type must be Sync
        // That means the streaming types need to be wrappers over Sync streams
        // However, Axum BodyDataStream is !Sync, so we can't use the same wrapper type there

        /*        let url = format!("{}{}", get_server_url(), path);
            let body = Body::wrap_stream(
                body.map(|chunk| Ok(chunk) as Result<Bytes, ServerFnErrorErr>),
            );
            CLIENT
                .post(url)
                .header(CONTENT_TYPE, content_type)
                .header(ACCEPT, accepts)
                .body(body)
                .build()
                .map_err(|e| ServerFnError::Request(e.to_string()))
        }*/
    }
}
