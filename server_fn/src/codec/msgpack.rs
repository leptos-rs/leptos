use super::{Encoding, FromReq, FromRes, IntoReq, IntoRes};
use crate::{
    error::ServerFnError,
    request::{ClientReq, Req},
    response::{ClientRes, Res},
};
use bytes::Bytes;
use http::Method;
use serde::{de::DeserializeOwned, Serialize};

/// A codec for MessagePack.
pub struct MsgPack;

impl Encoding for MsgPack {
    const CONTENT_TYPE: &'static str = "application/msgpack";
    const METHOD: Method = Method::POST;
}

impl<T, Request, Err> IntoReq<MsgPack, Request, Err> for T
where
    Request: ClientReq<Err>,
    T: Serialize,
{
    fn into_req(
        self,
        path: &str,
        accepts: &str,
    ) -> Result<Request, ServerFnError<Err>> {
        let data = rmp_serde::to_vec(&self)
            .map_err(|e| ServerFnError::Serialization(e.to_string()))?;
        Request::try_new_post_bytes(
            path,
            MsgPack::CONTENT_TYPE,
            accepts,
            Bytes::from(data),
        )
    }
}

impl<T, Request, Err> FromReq<MsgPack, Request, Err> for T
where
    Request: Req<Err> + Send,
    T: DeserializeOwned,
{
    async fn from_req(req: Request) -> Result<Self, ServerFnError<Err>> {
        let data = req.try_into_bytes().await?;
        rmp_serde::from_slice::<T>(&data)
            .map_err(|e| ServerFnError::Args(e.to_string()))
    }
}

impl<T, Response, Err> IntoRes<MsgPack, Response, Err> for T
where
    Response: Res<Err>,
    T: Serialize + Send,
{
    async fn into_res(self) -> Result<Response, ServerFnError<Err>> {
        let data = rmp_serde::to_vec(&self)
            .map_err(|e| ServerFnError::Serialization(e.to_string()))?;
        Response::try_from_bytes(MsgPack::CONTENT_TYPE, Bytes::from(data))
    }
}

impl<T, Response, Err> FromRes<MsgPack, Response, Err> for T
where
    Response: ClientRes<Err> + Send,
    T: DeserializeOwned,
{
    async fn from_res(res: Response) -> Result<Self, ServerFnError<Err>> {
        let data = res.try_into_bytes().await?;
        rmp_serde::from_slice(&data)
            .map_err(|e| ServerFnError::Deserialization(e.to_string()))
    }
}
