use crate::{Decodes, Encodes};
use bytes::Bytes;
use serde::{de::DeserializeOwned, Serialize};

/// Serializes and deserializes MessagePack with [`rmp_serde`].
pub struct MsgPack;

impl<T> Encodes<T> for MsgPack
where
    T: Serialize,
{
    type Error = rmp_serde::encode::Error;
    const CONTENT_TYPE: &'static str = "application/msgpack";

    fn encode(value: T) -> Result<Bytes, Self::Error> {
        rmp_serde::to_vec(&value).map(|bytes| Bytes::from(bytes))
    }
}

impl<T> Decodes<T> for MsgPack
where
    T: DeserializeOwned,
{
    type Error = rmp_serde::decode::Error;

    fn decode(bytes: Bytes) -> Result<T, Self::Error> {
        rmp_serde::from_slice(&bytes)
    }
}
