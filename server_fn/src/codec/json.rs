use super::{Encoding, FromReq, FromRes};
use crate::{
    error::ServerFnError,
    request::{ClientReq, Req},
    response::{ClientRes, Res},
    IntoReq, IntoRes,
};
use serde::{de::DeserializeOwned, Serialize};
/// Pass arguments and receive responses as JSON in the body of a `POST` request.
pub struct Json;

impl Encoding for Json {
    const CONTENT_TYPE: &'static str = "application/json";
}

impl<CustErr, T, Request> IntoReq<CustErr, Request, Json> for T
where
    Request: ClientReq<CustErr>,
    T: Serialize + Send,
{
    fn into_req(
        self,
        path: &str,
        accepts: &str,
    ) -> Result<Request, ServerFnError<CustErr>> {
        let data = serde_json::to_string(&self)
            .map_err(|e| ServerFnError::Serialization(e.to_string()))?;
        Request::try_new_post(path, accepts, Json::CONTENT_TYPE, data)
    }
}

impl<CustErr, T, Request> FromReq<CustErr, Request, Json> for T
where
    Request: Req<CustErr> + Send + 'static,
    T: DeserializeOwned,
{
    async fn from_req(req: Request) -> Result<Self, ServerFnError<CustErr>> {
        let string_data = req.try_into_string().await?;
        serde_json::from_str::<Self>(&string_data)
            .map_err(|e| ServerFnError::Args(e.to_string()))
    }
}

impl<CustErr, T, Response> IntoRes<CustErr, Response, Json> for T
where
    Response: Res<CustErr>,
    T: Serialize + Send,
{
    async fn into_res(self) -> Result<Response, ServerFnError<CustErr>> {
        let data = serde_json::to_string(&self)
            .map_err(|e| ServerFnError::Serialization(e.to_string()))?;
        Response::try_from_string(Json::CONTENT_TYPE, data)
    }
}

impl<CustErr, T, Response> FromRes<CustErr, Response, Json> for T
where
    Response: ClientRes<CustErr> + Send,
    T: DeserializeOwned + Send,
{
    async fn from_res(res: Response) -> Result<Self, ServerFnError<CustErr>> {
        let data = res.try_into_string().await?;
        serde_json::from_str(&data)
            .map_err(|e| ServerFnError::Deserialization(e.to_string()))
    }
}
