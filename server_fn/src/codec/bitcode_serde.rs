use crate::{
    codec::{Patch, Post, Put},
    ContentType, Decodes, Encodes, Format, FormatType,
};
use bytes::Bytes;
use serde::{de::DeserializeOwned, Serialize};

/// Serializes and deserializes with [`bitcode`]'s `serde` integration.
pub struct BitcodeSerdeEncoding;

impl ContentType for BitcodeSerdeEncoding {
    const CONTENT_TYPE: &'static str = "application/x-bitcode-serde";
}

impl FormatType for BitcodeSerdeEncoding {
    const FORMAT_TYPE: Format = Format::Binary;
}

impl<T> Encodes<T> for BitcodeSerdeEncoding
where
    T: Serialize,
{
    type Error = bitcode::Error;

    fn encode(value: &T) -> Result<Bytes, Self::Error> {
        bitcode::serialize(value).map(Bytes::from)
    }
}

impl<T> Decodes<T> for BitcodeSerdeEncoding
where
    T: DeserializeOwned,
{
    type Error = bitcode::Error;

    fn decode(bytes: Bytes) -> Result<T, Self::Error> {
        bitcode::deserialize(bytes.as_ref())
    }
}

/// Pass arguments and receive responses using `bitcode`'s serde integration in a `POST` request.
pub type BitcodeSerde = Post<BitcodeSerdeEncoding>;

/// Pass arguments and receive responses using `bitcode`'s serde integration in the body of a `PATCH` request.
/// **Note**: Browser support for `PATCH` requests without JS/WASM may be poor.
/// Consider using a `POST` request if functionality without JS/WASM is required.
pub type PatchBitcodeSerde = Patch<BitcodeSerdeEncoding>;

/// Pass arguments and receive responses using `bitcode`'s serde integration in the body of a `PUT` request.
/// **Note**: Browser support for `PUT` requests without JS/WASM may be poor.
/// Consider using a `POST` request if functionality without JS/WASM is required.
pub type PutBitcodeSerde = Put<BitcodeSerdeEncoding>;
