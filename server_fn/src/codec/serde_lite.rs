use super::{Encoding, FromReq, FromRes};
use crate::{
    error::{FromServerFnError, ServerFnErrorErr},
    request::{ClientReq, Req},
    response::{ClientRes, Res},
    IntoReq, IntoRes,
};
use http::Method;
use serde_lite::{Deserialize, Serialize};
/// Pass arguments and receive responses as JSON in the body of a `POST` request.
pub struct SerdeLite;

impl Encoding for SerdeLite {
    const CONTENT_TYPE: &'static str = "application/json";
    const METHOD: Method = Method::POST;
}

impl<E, T, Request> IntoReq<SerdeLite, Request, E> for T
where
    Request: ClientReq<E>,
    T: Serialize + Send,
    E: FromServerFnError,
{
    fn into_req(self, path: &str, accepts: &str) -> Result<Request, E> {
        let data = serde_json::to_string(&self.serialize().map_err(|e| {
            E::from(ServerFnErrorErr::Serialization(e.to_string()))
        })?)
        .map_err(|e| E::from(ServerFnErrorErr::Serialization(e.to_string())))?;
        Request::try_new_post(path, accepts, SerdeLite::CONTENT_TYPE, data)
    }
}

impl<E, T, Request> FromReq<SerdeLite, Request, E> for T
where
    Request: Req<E> + Send + 'static,
    T: Deserialize,
    E: FromServerFnError,
{
    async fn from_req(req: Request) -> Result<Self, E> {
        let string_data = req.try_into_string().await?;
        Self::deserialize(
            &serde_json::from_str(&string_data)
                .map_err(|e| E::from(ServerFnErrorErr::Args(e.to_string())))?,
        )
        .map_err(|e| E::from(ServerFnErrorErr::Args(e.to_string())))
    }
}

impl<E, T, Response> IntoRes<SerdeLite, Response, E> for T
where
    Response: Res<E>,
    T: Serialize + Send,
    E: FromServerFnError,
{
    async fn into_res(self) -> Result<Response, E> {
        let data = serde_json::to_string(&self.serialize().map_err(|e| {
            E::from(ServerFnErrorErr::Serialization(e.to_string()))
        })?)
        .map_err(|e| E::from(ServerFnErrorErr::Serialization(e.to_string())))?;
        Response::try_from_string(SerdeLite::CONTENT_TYPE, data)
    }
}

impl<E, T, Response> FromRes<SerdeLite, Response, E> for T
where
    Response: ClientRes<E> + Send,
    T: Deserialize + Send,
    E: FromServerFnError,
{
    async fn from_res(res: Response) -> Result<Self, E> {
        let data = res.try_into_string().await?;
        Self::deserialize(
            &serde_json::from_str(&data)
                .map_err(|e| E::from(ServerFnErrorErr::Args(e.to_string())))?,
        )
        .map_err(|e| E::from(ServerFnErrorErr::Deserialization(e.to_string())))
    }
}
