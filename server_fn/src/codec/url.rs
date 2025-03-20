use super::{Encoding, FromReq, IntoReq};
use crate::{
    error::{FromServerFnError, IntoAppError, ServerFnErrorErr},
    request::{ClientReq, Req},
    ContentType,
};
use http::Method;
use serde::{de::DeserializeOwned, Serialize};

/// Pass arguments as a URL-encoded query string of a `GET` request.
pub struct GetUrl;

/// Pass arguments as the URL-encoded body of a `POST` request.
pub struct PostUrl;

/// Pass arguments as the URL-encoded body of a `PATCH` request.
pub struct PatchUrl;

impl ContentType for GetUrl {
    const CONTENT_TYPE: &'static str = "application/x-www-form-urlencoded";
}

impl Encoding for GetUrl {
    const METHOD: Method = Method::GET;
}

impl<E, T, Request> IntoReq<GetUrl, Request, E> for T
where
    Request: ClientReq<E>,
    T: Serialize + Send,
    E: FromServerFnError,
{
    fn into_req(self, path: &str, accepts: &str) -> Result<Request, E> {
        let data = serde_qs::to_string(&self).map_err(|e| {
            ServerFnErrorErr::Serialization(e.to_string()).into_app_error()
        })?;
        Request::try_new_get(path, accepts, GetUrl::CONTENT_TYPE, &data)
    }
}

impl<E, T, Request> FromReq<GetUrl, Request, E> for T
where
    Request: Req<E> + Send + 'static,
    T: DeserializeOwned,
    E: FromServerFnError,
{
    async fn from_req(req: Request) -> Result<Self, E> {
        let string_data = req.as_query().unwrap_or_default();
        let args = serde_qs::Config::new(5, false)
            .deserialize_str::<Self>(string_data)
            .map_err(|e| {
                ServerFnErrorErr::Args(e.to_string()).into_app_error()
            })?;
        Ok(args)
    }
}

impl ContentType for PostUrl {
    const CONTENT_TYPE: &'static str = "application/x-www-form-urlencoded";
}

impl Encoding for PostUrl {
    const METHOD: Method = Method::POST;
}

impl<E, T, Request> IntoReq<PostUrl, Request, E> for T
where
    Request: ClientReq<E>,
    T: Serialize + Send,
    E: FromServerFnError,
{
    fn into_req(self, path: &str, accepts: &str) -> Result<Request, E> {
        let qs = serde_qs::to_string(&self).map_err(|e| {
            ServerFnErrorErr::Serialization(e.to_string()).into_app_error()
        })?;
        Request::try_new_post(path, accepts, PostUrl::CONTENT_TYPE, qs)
    }
}

impl<E, T, Request> FromReq<PostUrl, Request, E> for T
where
    Request: Req<E> + Send + 'static,
    T: DeserializeOwned,
    E: FromServerFnError,
{
    async fn from_req(req: Request) -> Result<Self, E> {
        let string_data = req.try_into_string().await?;
        let args = serde_qs::Config::new(5, false)
            .deserialize_str::<Self>(&string_data)
            .map_err(|e| {
                ServerFnErrorErr::Args(e.to_string()).into_app_error()
            })?;
        Ok(args)
    }
}

impl ContentType for PatchUrl {
    const CONTENT_TYPE: &'static str = "application/x-www-form-urlencoded";
}

impl Encoding for PatchUrl {
    const METHOD: Method = Method::PATCH;
}

impl<E, T, Request> IntoReq<PatchUrl, Request, E> for T
where
    Request: ClientReq<E>,
    T: Serialize + Send,
    E: FromServerFnError,
{
    fn into_req(self, path: &str, accepts: &str) -> Result<Request, E> {
        let qs = serde_qs::to_string(&self).map_err(|e| {
            ServerFnErrorErr::Serialization(e.to_string()).into_app_error()
        })?;
        Request::try_new_patch(path, accepts, PatchUrl::CONTENT_TYPE, qs)
    }
}

impl<E, T, Request> FromReq<PatchUrl, Request, E> for T
where
    Request: Req<E> + Send + 'static,
    T: DeserializeOwned,
    E: FromServerFnError,
{
    async fn from_req(req: Request) -> Result<Self, E> {
        let string_data = req.try_into_string().await?;
        let args = serde_qs::Config::new(5, false)
            .deserialize_str::<Self>(&string_data)
            .map_err(|e| {
                ServerFnErrorErr::Args(e.to_string()).into_app_error()
            })?;
        Ok(args)
    }
}
