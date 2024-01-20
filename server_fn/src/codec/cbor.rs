use super::{Encoding, FromReq, FromRes, IntoReq, IntoRes};
use crate::{
    error::ServerFnError,
    request::{ClientReq, Req},
    response::{ClientRes, Res},
};
use bytes::Bytes;
use http::Method;
use serde::{de::DeserializeOwned, Serialize};

/// Pass arguments and receive responses using `cbor` in a `POST` request.
pub struct Cbor;

impl Encoding for Cbor {
    const CONTENT_TYPE: &'static str = "application/cbor";
    const METHOD: Method = Method::POST;
}

impl<CustErr, T, Request> IntoReq<Cbor, Request, CustErr> for T
where
    Request: ClientReq<CustErr>,
    T: Serialize + Send,
{
    fn into_req(
        self,
        path: &str,
        accepts: &str,
    ) -> Result<Request, ServerFnError<CustErr>> {
        let mut buffer: Vec<u8> = Vec::new();
        ciborium::ser::into_writer(&self, &mut buffer)
            .map_err(|e| ServerFnError::Serialization(e.to_string()))?;
        Request::try_new_post_bytes(
            path,
            accepts,
            Cbor::CONTENT_TYPE,
            Bytes::from(buffer),
        )
    }
}

impl<CustErr, T, Request> FromReq<Cbor, Request, CustErr> for T
where
    Request: Req<CustErr> + Send + 'static,
    T: DeserializeOwned,
{
    async fn from_req(req: Request) -> Result<Self, ServerFnError<CustErr>> {
        let body_bytes = req.try_into_bytes().await?;
        ciborium::de::from_reader(body_bytes.as_ref())
            .map_err(|e| ServerFnError::Args(e.to_string()))
    }
}

impl<CustErr, T, Response> IntoRes<Cbor, Response, CustErr> for T
where
    Response: Res<CustErr>,
    T: Serialize + Send,
{
    async fn into_res(self) -> Result<Response, ServerFnError<CustErr>> {
        let mut buffer: Vec<u8> = Vec::new();
        ciborium::ser::into_writer(&self, &mut buffer)
            .map_err(|e| ServerFnError::Serialization(e.to_string()))?;
        Response::try_from_bytes(Cbor::CONTENT_TYPE, Bytes::from(buffer))
    }
}

impl<CustErr, T, Response> FromRes<Cbor, Response, CustErr> for T
where
    Response: ClientRes<CustErr> + Send,
    T: DeserializeOwned + Send,
{
    async fn from_res(res: Response) -> Result<Self, ServerFnError<CustErr>> {
        let data = res.try_into_bytes().await?;
        ciborium::de::from_reader(data.as_ref())
            .map_err(|e| ServerFnError::Args(e.to_string()))
    }
}

/* use std::fmt::Display;

use super::{Codec, Encoding};
use crate::error::{ServerFnError, IntoErrorResponse};
use async_trait::async_trait;
use axum::body::{Body, HttpBody};
use http_body_util::BodyExt;
use serde::de::DeserializeOwned;
use serde::Serialize;
/// Pass argument as JSON in the body of a POST Request
pub struct PostCbor;

impl Encoding for PostCbor {
    const REQUEST_CONTENT_TYPE: &'static str = "application/cbor";
    const RESPONSE_CONTENT_TYPE: &'static str = "application/cbor";
}

#[async_trait]
impl<T, RequestBody, ResponseBody>
    Codec<
        RequestBody,
        ResponseBody,
        http::Request<RequestBody>,
        http::Response<ResponseBody>,
        Body,
        Body,
        http::Request<Body>,
        http::Response<Body>,
        PostCbor,
    > for T
where
    T: DeserializeOwned + Serialize + Send,
    for<'a> RequestBody: HttpBody + Send  + 'a,
    <RequestBody as HttpBody>::Error: Display + Send ,
    <ResponseBody as HttpBody>::Error: Display + Send ,
    for<'a> ResponseBody: HttpBody + Send  + 'a,
    <ResponseBody as HttpBody>::Data: Send ,
    <RequestBody as HttpBody>::Data: Send ,
{
    async fn from_req(req: http::Request<RequestBody>) -> Result<Self, ServerFnError<CustErr>> {
        let (_parts, body) = req.into_parts();

        let body_bytes = body
            .collect()
            .await
            .map(|c| c.to_bytes())
            .map_err(|e| ServerFnError::Deserialization(e.to_string()))?;
        let data = ciborium::de::from_reader(body_bytes.as_ref())
            .map_err(|e| ServerFnError::Args(e.to_string()))?;
        Ok(data)
    }

    async fn into_req(self) -> Result<http::Request<Body>, ServerFnError<CustErr>> {
        let mut buffer: Vec<u8> = Vec::new();
        ciborium::ser::into_writer(&self, &mut buffer)?;
        let req = http::Request::builder()
            .method("POST")
            .header(
                http::header::CONTENT_TYPE,
                <PostCbor as Encoding>::REQUEST_CONTENT_TYPE,
            )
            .body(Body::from(buffer))?;
        Ok(req)
    }
    async fn from_res(res: http::Response<ResponseBody>) -> Result<Self, ServerFnError<CustErr>> {
        let (_parts, body) = res.into_parts();

        let body_bytes = body
            .collect()
            .await
            .map(|c| c.to_bytes())
            .map_err(|e| ServerFnError::Deserialization(e.to_string()))?;

        ciborium::de::from_reader(body_bytes.as_ref())
            .map_err(|e| ServerFnError::Args(e.to_string()))
    }

    async fn into_res(self) -> http::Response<Body> {
        let mut buffer: Vec<u8> = Vec::new();
        match ciborium::ser::into_writer(&self, &mut buffer) {
            Ok(_) => (),
            Err(e) => return e.into_err_res(),
        };

        let res = http::Response::builder()
            .status(200)
            .header(
                http::header::CONTENT_TYPE,
                <PostCbor as Encoding>::REQUEST_CONTENT_TYPE,
            )
            .body(Body::from(buffer))
            .unwrap();
        res
    }
}
 */
