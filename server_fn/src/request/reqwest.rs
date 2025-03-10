use super::ClientReq;
use crate::{
    client::get_server_url,
    error::{FromServerFnError, IntoAppError, ServerFnErrorErr},
};
use bytes::Bytes;
use futures::{Stream, StreamExt};
use once_cell::sync::Lazy;
use reqwest::{
    header::{ACCEPT, CONTENT_TYPE},
    Body,
};
pub use reqwest::{multipart::Form, Client, Method, Request, Url};

pub(crate) static CLIENT: Lazy<Client> = Lazy::new(Client::new);

impl<E> ClientReq<E> for Request
where
    E: FromServerFnError,
{
    type FormData = Form;

    fn try_new_get(
        path: &str,
        accepts: &str,
        content_type: &str,
        query: &str,
    ) -> Result<Self, E> {
        let url = format!("{}{}", get_server_url(), path);
        let mut url = Url::try_from(url.as_str()).map_err(|e| {
            E::from_server_fn_error(ServerFnErrorErr::Request(e.to_string()))
        })?;
        url.set_query(Some(query));
        let req = CLIENT
            .get(url)
            .header(CONTENT_TYPE, content_type)
            .header(ACCEPT, accepts)
            .build()
            .map_err(|e| {
                E::from_server_fn_error(ServerFnErrorErr::Request(
                    e.to_string(),
                ))
            })?;
        Ok(req)
    }

    fn try_new_post(
        path: &str,
        accepts: &str,
        content_type: &str,
        body: String,
    ) -> Result<Self, E> {
        let url = format!("{}{}", get_server_url(), path);
        CLIENT
            .post(url)
            .header(CONTENT_TYPE, content_type)
            .header(ACCEPT, accepts)
            .body(body)
            .build()
            .map_err(|e| {
                ServerFnErrorErr::Request(e.to_string()).into_app_error()
            })
    }

    fn try_new_post_bytes(
        path: &str,
        accepts: &str,
        content_type: &str,
        body: Bytes,
    ) -> Result<Self, E> {
        let url = format!("{}{}", get_server_url(), path);
        CLIENT
            .post(url)
            .header(CONTENT_TYPE, content_type)
            .header(ACCEPT, accepts)
            .body(body)
            .build()
            .map_err(|e| {
                ServerFnErrorErr::Request(e.to_string()).into_app_error()
            })
    }

    fn try_new_multipart(
        path: &str,
        accepts: &str,
        body: Self::FormData,
    ) -> Result<Self, E> {
        CLIENT
            .post(path)
            .header(ACCEPT, accepts)
            .multipart(body)
            .build()
            .map_err(|e| {
                ServerFnErrorErr::Request(e.to_string()).into_app_error()
            })
    }

    fn try_new_post_form_data(
        path: &str,
        accepts: &str,
        content_type: &str,
        body: Self::FormData,
    ) -> Result<Self, E> {
        CLIENT
            .post(path)
            .header(CONTENT_TYPE, content_type)
            .header(ACCEPT, accepts)
            .multipart(body)
            .build()
            .map_err(|e| {
                ServerFnErrorErr::Request(e.to_string()).into_app_error()
            })
    }

    fn try_new_streaming(
        path: &str,
        accepts: &str,
        content_type: &str,
        body: impl Stream<Item = Bytes> + Send + 'static,
    ) -> Result<Self, E> {
        let url = format!("{}{}", get_server_url(), path);
        let body = Body::wrap_stream(
            body.map(|chunk| Ok(chunk) as Result<Bytes, ServerFnErrorErr>),
        );
        CLIENT
            .post(url)
            .header(CONTENT_TYPE, content_type)
            .header(ACCEPT, accepts)
            .body(body)
            .build()
            .map_err(|e| {
                ServerFnErrorErr::Request(e.to_string()).into_app_error()
            })
    }
}
