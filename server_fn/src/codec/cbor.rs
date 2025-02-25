use crate::{Decodes, Encodes};
use bytes::Bytes;
use serde::{de::DeserializeOwned, Serialize};

/// Serializes and deserializes CBOR with [`ciborium`].
pub struct Cbor;

impl<T> Encodes<T> for Cbor
where
    T: Serialize,
{
    type Error = ciborium::ser::Error<std::io::Error>;
    const CONTENT_TYPE: &'static str = "application/cbor";

    fn encode(value: T) -> Result<Bytes, Self::Error> {
        let mut buffer: Vec<u8> = Vec::new();
        ciborium::ser::into_writer(&value, &mut buffer)?;
        Ok(Bytes::from(buffer))
    }
}

impl<T> Decodes<T> for Cbor
where
    T: DeserializeOwned,
{
    type Error = ciborium::de::Error<std::io::Error>;

    fn decode(bytes: Bytes) -> Result<T, Self::Error> {
        ciborium::de::from_reader(bytes.as_ref())
    }
}
