use super::{Encoding, FromReq, FromRes, IntoReq, IntoRes};
use crate::{
    error::{FromServerFnError, ServerFnErrorErr},
    request::{ClientReq, Req},
    response::{ClientRes, Res},
};
use bytes::Bytes;
use futures::StreamExt;
use http::Method;
use rkyv::{
    api::high::{HighDeserializer, HighSerializer, HighValidator},
    bytecheck::CheckBytes,
    rancor,
    ser::allocator::ArenaHandle,
    util::AlignedVec,
    Archive, Deserialize, Serialize,
};

type RkyvSerializer<'a> =
    HighSerializer<AlignedVec, ArenaHandle<'a>, rancor::Error>;
type RkyvDeserializer = HighDeserializer<rancor::Error>;
type RkyvValidator<'a> = HighValidator<'a, rancor::Error>;

/// Pass arguments and receive responses using `rkyv` in a `POST` request.
pub struct Rkyv;

impl Encoding for Rkyv {
    const CONTENT_TYPE: &'static str = "application/rkyv";
    const METHOD: Method = Method::POST;
}

impl<E, T, Request> IntoReq<Rkyv, Request, E> for T
where
    Request: ClientReq<E>,
    T: Archive + for<'a> Serialize<RkyvSerializer<'a>>,
    T::Archived: Deserialize<T, RkyvDeserializer>
        + for<'a> CheckBytes<RkyvValidator<'a>>,
    E: FromServerFnError,
{
    fn into_req(self, path: &str, accepts: &str) -> Result<Request, E> {
        let encoded = rkyv::to_bytes::<rancor::Error>(&self).map_err(|e| {
            E::from(ServerFnErrorErr::Serialization(e.to_string()))
        })?;
        let bytes = Bytes::copy_from_slice(encoded.as_ref());
        Request::try_new_post_bytes(path, accepts, Rkyv::CONTENT_TYPE, bytes)
    }
}

impl<E, T, Request> FromReq<Rkyv, Request, E> for T
where
    Request: Req<E> + Send + 'static,
    T: Archive + for<'a> Serialize<RkyvSerializer<'a>>,
    T::Archived: Deserialize<T, RkyvDeserializer>
        + for<'a> CheckBytes<RkyvValidator<'a>>,
    E: FromServerFnError,
{
    async fn from_req(req: Request) -> Result<Self, E> {
        let mut aligned = AlignedVec::<1024>::new();
        let mut body_stream = Box::pin(req.try_into_stream()?);
        while let Some(chunk) = body_stream.next().await {
            match chunk {
                Err(e) => {
                    return Err(ServerFnErrorErr::Deserialization(
                        e.to_string(),
                    )
                    .into())
                }
                Ok(bytes) => {
                    for byte in bytes {
                        aligned.push(byte);
                    }
                }
            }
        }
        rkyv::from_bytes::<T, rancor::Error>(aligned.as_ref())
            .map_err(|e| ServerFnErrorErr::Args(e.to_string()).into())
    }
}

impl<E, T, Response> IntoRes<Rkyv, Response, E> for T
where
    Response: Res<E>,
    T: Send,
    T: Archive + for<'a> Serialize<RkyvSerializer<'a>>,
    T::Archived: Deserialize<T, RkyvDeserializer>
        + for<'a> CheckBytes<RkyvValidator<'a>>,
    E: FromServerFnError,
{
    async fn into_res(self) -> Result<Response, E> {
        let encoded = rkyv::to_bytes::<rancor::Error>(&self).map_err(|e| {
            E::from(ServerFnErrorErr::Serialization(format!("{e:?}")))
        })?;
        let bytes = Bytes::copy_from_slice(encoded.as_ref());
        Response::try_from_bytes(Rkyv::CONTENT_TYPE, bytes)
    }
}

impl<E, T, Response> FromRes<Rkyv, Response, E> for T
where
    Response: ClientRes<E> + Send,
    T: Archive + for<'a> Serialize<RkyvSerializer<'a>>,
    T::Archived: Deserialize<T, RkyvDeserializer>
        + for<'a> CheckBytes<RkyvValidator<'a>>,
    E: FromServerFnError,
{
    async fn from_res(res: Response) -> Result<Self, E> {
        let data = res.try_into_bytes().await?;
        rkyv::from_bytes::<T, rancor::Error>(&data).map_err(|e| {
            ServerFnErrorErr::Deserialization(e.to_string()).into()
        })
    }
}
