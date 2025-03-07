use crate::{codec::Post, ContentType, Decodes, Encodes};
use bytes::Bytes;
use serde::{de::DeserializeOwned, Serialize};

/// Serializes and deserializes MessagePack with [`rmp_serde`].
pub struct MsgPack;

impl ContentType for MsgPack {
    const CONTENT_TYPE: &'static str = "application/msgpack";
}

impl<T> Encodes<T> for MsgPack
where
    T: Serialize,
{
    type Error = rmp_serde::encode::Error;

    fn encode(value: T) -> Result<Bytes, Self::Error> {
        rmp_serde::to_vec(&value).map(Bytes::from)
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

/// Pass arguments and receive responses as MessagePack in a `POST` request.
pub type PostMsgPack = Post<MsgPack>;
