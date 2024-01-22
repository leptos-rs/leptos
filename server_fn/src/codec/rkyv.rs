use super::{Encoding, FromReq, FromRes, IntoReq, IntoRes};
use crate::{
    error::ServerFnError,
    request::{ClientReq, Req},
    response::{ClientRes, Res},
};
use bytes::Bytes;
use futures::StreamExt;
use http::Method;
use rkyv::{
    de::deserializers::SharedDeserializeMap, ser::serializers::AllocSerializer,
    validation::validators::DefaultValidator, AlignedVec, Archive, CheckBytes,
    Deserialize, Serialize,
};

/// Pass arguments and receive responses using `rkyv` in a `POST` request.
pub struct Rkyv;

impl Encoding for Rkyv {
    const CONTENT_TYPE: &'static str = "application/rkyv";
    const METHOD: Method = Method::POST;
}

impl<CustErr, T, Request> IntoReq<Rkyv, Request, CustErr> for T
where
    Request: ClientReq<CustErr>,
    T: Serialize<AllocSerializer<1024>> + Send,
    T: Archive,
    T::Archived: for<'a> CheckBytes<DefaultValidator<'a>>
        + Deserialize<T, SharedDeserializeMap>,
{
    fn into_req(
        self,
        path: &str,
        accepts: &str,
    ) -> Result<Request, ServerFnError<CustErr>> {
        let encoded = rkyv::to_bytes::<T, 1024>(&self)
            .map_err(|e| ServerFnError::Serialization(e.to_string()))?;
        let bytes = Bytes::copy_from_slice(encoded.as_ref());
        Request::try_new_post_bytes(path, accepts, Rkyv::CONTENT_TYPE, bytes)
    }
}

impl<CustErr, T, Request> FromReq<Rkyv, Request, CustErr> for T
where
    Request: Req<CustErr> + Send + 'static,
    T: Serialize<AllocSerializer<1024>> + Send,
    T: Archive,
    T::Archived: for<'a> CheckBytes<DefaultValidator<'a>>
        + Deserialize<T, SharedDeserializeMap>,
{
    async fn from_req(req: Request) -> Result<Self, ServerFnError<CustErr>> {
        let mut aligned = AlignedVec::new();
        let mut body_stream = Box::pin(req.try_into_stream()?);
        while let Some(chunk) = body_stream.next().await {
            match chunk {
                Err(e) => {
                    return Err(ServerFnError::Deserialization(e.to_string()))
                }
                Ok(bytes) => {
                    for byte in bytes {
                        aligned.push(byte);
                    }
                }
            }
        }
        rkyv::from_bytes::<T>(aligned.as_ref())
            .map_err(|e| ServerFnError::Args(e.to_string()))
    }
}

impl<CustErr, T, Response> IntoRes<Rkyv, Response, CustErr> for T
where
    Response: Res<CustErr>,
    T: Serialize<AllocSerializer<1024>> + Send,
    T: Archive,
    T::Archived: for<'a> CheckBytes<DefaultValidator<'a>>
        + Deserialize<T, SharedDeserializeMap>,
{
    async fn into_res(self) -> Result<Response, ServerFnError<CustErr>> {
        let encoded = rkyv::to_bytes::<T, 1024>(&self)
            .map_err(|e| ServerFnError::Serialization(e.to_string()))?;
        let bytes = Bytes::copy_from_slice(encoded.as_ref());
        Response::try_from_bytes(Rkyv::CONTENT_TYPE, bytes)
    }
}

impl<CustErr, T, Response> FromRes<Rkyv, Response, CustErr> for T
where
    Response: ClientRes<CustErr> + Send,
    T: Serialize<AllocSerializer<1024>> + Send,
    T: Archive,
    T::Archived: for<'a> CheckBytes<DefaultValidator<'a>>
        + Deserialize<T, SharedDeserializeMap>,
{
    async fn from_res(res: Response) -> Result<Self, ServerFnError<CustErr>> {
        let data = res.try_into_bytes().await?;
        rkyv::from_bytes::<T>(&data)
            .map_err(|e| ServerFnError::Deserialization(e.to_string()))
    }
}
