use super::Post;
use crate::{ContentType, Decodes, Encodes};
use bytes::Bytes;
use serde::{de::DeserializeOwned, Serialize};

/// Serializes and deserializes JSON with [`serde_json`].
pub struct JsonEncoding;

impl ContentType for JsonEncoding {
    const CONTENT_TYPE: &'static str = "application/json";
}

impl<T> Encodes<T> for JsonEncoding
where
    T: Serialize,
{
    type Error = serde_json::Error;

    fn encode(output: T) -> Result<Bytes, Self::Error> {
        serde_json::to_vec(&output).map(Bytes::from)
    }
}

impl<T> Decodes<T> for JsonEncoding
where
    T: DeserializeOwned,
{
    type Error = serde_json::Error;

    fn decode(bytes: Bytes) -> Result<T, Self::Error> {
        serde_json::from_slice(&bytes)
    }
}

/// Pass arguments and receive responses as JSON in the body of a `POST` request.
pub type Json = Post<JsonEncoding>;
