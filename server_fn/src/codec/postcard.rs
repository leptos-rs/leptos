use super::{Encoding, FromReq, FromRes, IntoReq, IntoRes};
use crate::{
    error::ServerFnError,
    request::{ClientReq, Req},
    response::{ClientRes, Res},
};
use bytes::Bytes;
use http::Method;
use serde::{de::DeserializeOwned, Serialize};

/// A codec for Postcard.
pub struct Postcard;

impl Encoding for Postcard {
    const CONTENT_TYPE: &'static str = "application/x-postcard";
    const METHOD: Method = Method::POST;
}

impl<T, Request, Err> IntoReq<Postcard, Request, Err> for T
where
    Request: ClientReq<Err>,
    T: Serialize,
{
    fn into_req(
        self,
        path: &str,
        accepts: &str,
    ) -> Result<Request, ServerFnError<Err>> {
        let data = postcard::to_allocvec(&self)
            .map_err(|e| ServerFnError::Serialization(e.to_string()))?;
        Request::try_new_post_bytes(
            path,
            Postcard::CONTENT_TYPE,
            accepts,
            Bytes::from(data),
        )
    }
}

impl<T, Request, Err> FromReq<Postcard, Request, Err> for T
where
    Request: Req<Err> + Send,
    T: DeserializeOwned,
{
    async fn from_req(req: Request) -> Result<Self, ServerFnError<Err>> {
        let data = req.try_into_bytes().await?;
        postcard::from_bytes::<T>(&data)
            .map_err(|e| ServerFnError::Args(e.to_string()))
    }
}

impl<T, Response, Err> IntoRes<Postcard, Response, Err> for T
where
    Response: Res<Err>,
    T: Serialize + Send,
{
    async fn into_res(self) -> Result<Response, ServerFnError<Err>> {
        let data = postcard::to_allocvec(&self)
            .map_err(|e| ServerFnError::Serialization(e.to_string()))?;
        Response::try_from_bytes(Postcard::CONTENT_TYPE, Bytes::from(data))
    }
}

impl<T, Response, Err> FromRes<Postcard, Response, Err> for T
where
    Response: ClientRes<Err> + Send,
    T: DeserializeOwned,
{
    async fn from_res(res: Response) -> Result<Self, ServerFnError<Err>> {
        let data = res.try_into_bytes().await?;
        postcard::from_bytes(&data)
            .map_err(|e| ServerFnError::Deserialization(e.to_string()))
    }
}
