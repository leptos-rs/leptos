use crate::{codec::Post, ContentType, Decodes, Encodes};
use bytes::Bytes;
use serde::{de::DeserializeOwned, Serialize};

/// A codec for Postcard.
pub struct PostcardEncoding;

impl ContentType for PostcardEncoding {
    const CONTENT_TYPE: &'static str = "application/x-postcard";
}

impl<T> Encodes<T> for PostcardEncoding
where
    T: Serialize,
{
    type Error = postcard::Error;

    fn encode(value: T) -> Result<Bytes, Self::Error> {
        postcard::to_allocvec(&value).map(Bytes::from)
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
