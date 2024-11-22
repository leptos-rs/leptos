use super::{Encoding, FromReq, FromRes, IntoReq, IntoRes};
use crate::{
    error::{FromServerFnError, ServerFnErrorErr},
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

impl<T, Request, E> IntoReq<MsgPack, Request, E> for T
where
    Request: ClientReq<E>,
    T: Serialize,
    E: FromServerFnError,
{
    fn into_req(self, path: &str, accepts: &str) -> Result<Request, E> {
        let data = rmp_serde::to_vec(&self).map_err(|e| {
            E::from(ServerFnErrorErr::Serialization(e.to_string()))
        })?;
        Request::try_new_post_bytes(
            path,
            MsgPack::CONTENT_TYPE,
            accepts,
            Bytes::from(data),
        )
    }
}

impl<T, Request, E> FromReq<MsgPack, Request, E> for T
where
    Request: Req<E> + Send,
    T: DeserializeOwned,
    E: FromServerFnError,
{
    async fn from_req(req: Request) -> Result<Self, E> {
        let data = req.try_into_bytes().await?;
        rmp_serde::from_slice::<T>(&data)
            .map_err(|e| ServerFnErrorErr::Args(e.to_string()).into())
    }
}

impl<T, Response, E> IntoRes<MsgPack, Response, E> for T
where
    Response: Res<E>,
    T: Serialize + Send,
    E: FromServerFnError,
{
    async fn into_res(self) -> Result<Response, E> {
        let data = rmp_serde::to_vec(&self).map_err(|e| {
            E::from(ServerFnErrorErr::Serialization(e.to_string()))
        })?;
        Response::try_from_bytes(MsgPack::CONTENT_TYPE, Bytes::from(data))
    }
}

impl<T, Response, E> FromRes<MsgPack, Response, E> for T
where
    Response: ClientRes<E> + Send,
    T: DeserializeOwned,
    E: FromServerFnError,
{
    async fn from_res(res: Response) -> Result<Self, E> {
        let data = res.try_into_bytes().await?;
        rmp_serde::from_slice(&data).map_err(|e| {
            E::from(ServerFnErrorErr::Deserialization(e.to_string()))
        })
    }
}
