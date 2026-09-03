use crate::{
    ContentType, Decodes, Encodes, Format, FormatType,
    codec::{Patch, Post, Put},
};
use bytes::{Bytes, BytesMut};
use serde::{Serialize, de::DeserializeOwned};

/// A codec for Postcard.
pub struct PostcardEncoding;

impl ContentType for PostcardEncoding {
    const CONTENT_TYPE: &'static str = "application/x-postcard";
}

impl FormatType for PostcardEncoding {
    const FORMAT_TYPE: Format = Format::Binary;
}

impl<T> Encodes<T> for PostcardEncoding
where
    T: Serialize,
{
    type Error = postcard::Error;

    fn encode(value: &T) -> Result<Bytes, Self::Error> {
        postcard::to_allocvec(value).map(Bytes::from)
    }

    fn encode_into(value: &T, buf: &mut BytesMut) -> Result<(), Self::Error> {
        // `to_extend` appends into the buffer we already own (which holds any
        // framing written before it); `mem::take` hands it over by move, so the
        // existing contents are not copied.
        *buf = postcard::to_extend(value, std::mem::take(buf))?;
        Ok(())
    }
}

impl<T> Decodes<T> for PostcardEncoding
where
    T: DeserializeOwned,
{
    type Error = postcard::Error;

    fn decode(bytes: Bytes) -> Result<T, Self::Error> {
        postcard::from_bytes(&bytes)
    }
}

/// Pass arguments and receive responses with postcard in a `POST` request.
pub type Postcard = Post<PostcardEncoding>;

/// Pass arguments and receive responses with postcard in a `PATCH` request.
/// **Note**: Browser support for `PATCH` requests without JS/WASM may be poor.
/// Consider using a `POST` request if functionality without JS/WASM is required.
pub type PatchPostcard = Patch<PostcardEncoding>;

/// Pass arguments and receive responses with postcard in a `PUT` request.
/// **Note**: Browser support for `PUT` requests without JS/WASM may be poor.
/// Consider using a `POST` request if functionality without JS/WASM is required.
pub type PutPostcard = Put<PostcardEncoding>;
