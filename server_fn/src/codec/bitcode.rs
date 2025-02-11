use bytes::Bytes;
use http::Method;
use crate::codec::{Encoding, FromReq, FromRes, IntoReq, IntoRes};
use crate::request::{ClientReq, Req};
use crate::response::{ClientRes, Res};
use crate::ServerFnError;
use bitcode::*;

pub struct Bitcode;

impl Encoding for Bitcode {
    const CONTENT_TYPE: &'static str = "application/bitcode";
    const METHOD: Method = Method::POST;
}

impl<CustErr, T, Request> IntoReq<Bitcode, Request, CustErr> for T
where
    Request: ClientReq<CustErr>,
    T: Encode + Send,
{
    fn into_req(
        self,
        path: &str,
        accepts: &str,
    ) -> Result<Request, ServerFnError<CustErr>> {
        Request::try_new_post_bytes(
            path,
            accepts,
            Bitcode::CONTENT_TYPE,
            Bytes::from(encode(&self)),
        )
    }
}

impl<CustErr, T, Request> FromReq<Bitcode, Request, CustErr> for T
where
    Request: Req<CustErr> + Send + 'static,
    T:DecodeOwned,
{
    async fn from_req(req: Request) -> Result<Self, ServerFnError<CustErr>> {
        let body_bytes = req.try_into_bytes().await?;
        decode(body_bytes.as_ref())
            .map_err(|e| ServerFnError::Args(e.to_string()))
    }
}

impl<CustErr, T, Response> IntoRes<Bitcode, Response, CustErr> for T
where
    Response: Res<CustErr>,
    T: Encode + Send,
{
    async fn into_res(self) -> Result<Response, ServerFnError<CustErr>> {
        Response::try_from_bytes(Bitcode::CONTENT_TYPE, Bytes::from(encode(&self)))
    }
}

impl<CustErr, T, Response> FromRes<Bitcode, Response, CustErr> for T
where
    Response: ClientRes<CustErr> + Send,
    T: DecodeOwned + Send,
{
    async fn from_res(res: Response) -> Result<Self, ServerFnError<CustErr>> {
        let data = res.try_into_bytes().await?;
        decode(data.as_ref())
            .map_err(|e| ServerFnError::Deserialization(e.to_string()))
    }
}