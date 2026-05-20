use super::{Patch, Post, Put};
use crate::{ContentType, Decodes, Encodes, Format, FormatType};
use bytes::Bytes;

/// Serializes and deserializes with [`bitcode`].
pub struct BitcodeEncoding;

impl ContentType for BitcodeEncoding {
    const CONTENT_TYPE: &'static str = "application/bitcode";
}

impl FormatType for BitcodeEncoding {
    const FORMAT_TYPE: Format = Format::Binary;
}

impl<T> Encodes<T> for BitcodeEncoding
where
    T: bitcode::Encode,
{
    type Error = std::convert::Infallible;

    fn encode(value: &T) -> Result<Bytes, Self::Error> {
        Ok(Bytes::from(bitcode::encode(value)))
    }
}

impl<T> Decodes<T> for BitcodeEncoding
where
    T: bitcode::DecodeOwned,
{
    type Error = bitcode::Error;

    fn decode(bytes: Bytes) -> Result<T, Self::Error> {
        bitcode::decode(bytes.as_ref())
    }
}

/// Pass arguments and receive responses using `bitcode` in a `POST` request.
pub type Bitcode = Post<BitcodeEncoding>;

/// Pass arguments and receive responses using `bitcode` in the body of a `PATCH` request.
/// **Note**: Browser support for `PATCH` requests without JS/WASM may be poor.
/// Consider using a `POST` request if functionality without JS/WASM is required.
pub type PatchBitcode = Patch<BitcodeEncoding>;

/// Pass arguments and receive responses using `bitcode` in the body of a `PUT` request.
/// **Note**: Browser support for `PUT` requests without JS/WASM may be poor.
/// Consider using a `POST` request if functionality without JS/WASM is required.
pub type PutBitcode = Put<BitcodeEncoding>;
