use crate::{ContentType, Decodes, Encodes};
use bytes::Bytes;
use serde::{de::DeserializeOwned, Serialize};

/// Serializes and deserializes JSON with [`serde_json`].
pub struct Json;

impl ContentType for Json {
    const CONTENT_TYPE: &'static str = "application/json";
}

impl<T> Encodes<T> for Json
where
    T: Serialize,
{
    type Error = serde_json::Error;

    fn encode(output: T) -> Result<Bytes, Self::Error> {
        serde_json::to_vec(&output).map(Bytes::from)
    }
}

impl<T> Decodes<T> for Json
where
    T: DeserializeOwned,
{
    type Error = serde_json::Error;

    fn decode(bytes: Bytes) -> Result<T, Self::Error> {
        serde_json::from_slice(&bytes)
    }
}
