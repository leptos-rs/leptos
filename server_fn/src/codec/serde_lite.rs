use super::{Encoding, FromReq, FromRes};
use crate::{
    error::ServerFnError,
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

impl<CustErr, T, Request> IntoReq<SerdeLite, Request, CustErr> for T
where
    Request: ClientReq<CustErr>,
    T: Serialize + Send,
{
    fn into_req(
        self,
        path: &str,
        accepts: &str,
    ) -> Result<Request, ServerFnError<CustErr>> {
        let data = serde_json::to_string(
            &self
                .serialize()
                .map_err(|e| ServerFnError::Serialization(e.to_string()))?,
        )
        .map_err(|e| ServerFnError::Serialization(e.to_string()))?;
        Request::try_new_post(path, accepts, SerdeLite::CONTENT_TYPE, data)
    }
}

impl<CustErr, T, Request> FromReq<SerdeLite, Request, CustErr> for T
where
    Request: Req<CustErr> + Send + 'static,
    T: Deserialize,
{
    async fn from_req(req: Request) -> Result<Self, ServerFnError<CustErr>> {
        let string_data = req.try_into_string().await?;
        Self::deserialize(
            &serde_json::from_str(&string_data)
                .map_err(|e| ServerFnError::Args(e.to_string()))?,
        )
        .map_err(|e| ServerFnError::Args(e.to_string()))
    }
}

impl<CustErr, T, Response> IntoRes<SerdeLite, Response, CustErr> for T
where
    Response: Res<CustErr>,
    T: Serialize + Send,
{
    async fn into_res(self) -> Result<Response, ServerFnError<CustErr>> {
        let data = serde_json::to_string(
            &self
                .serialize()
                .map_err(|e| ServerFnError::Serialization(e.to_string()))?,
        )
        .map_err(|e| ServerFnError::Serialization(e.to_string()))?;
        Response::try_from_string(SerdeLite::CONTENT_TYPE, data)
    }
}

impl<CustErr, T, Response> FromRes<SerdeLite, Response, CustErr> for T
where
    Response: ClientRes<CustErr> + Send,
    T: Deserialize + Send,
{
    async fn from_res(res: Response) -> Result<Self, ServerFnError<CustErr>> {
        let data = res.try_into_string().await?;
        Self::deserialize(
            &serde_json::from_str(&data)
                .map_err(|e| ServerFnError::Args(e.to_string()))?,
        )
        .map_err(|e| ServerFnError::Deserialization(e.to_string()))
    }
}
