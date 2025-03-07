use crate::{codec::Post, ContentType, Decodes, Encodes};
use bytes::Bytes;
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

impl ContentType for Rkyv {
    const CONTENT_TYPE: &'static str = "application/rkyv";
}

impl<T> Encodes<T> for Rkyv
where
    T: Archive + for<'a> Serialize<RkyvSerializer<'a>>,
    T::Archived: Deserialize<T, RkyvDeserializer>
        + for<'a> CheckBytes<RkyvValidator<'a>>,
{
    type Error = rancor::Error;

    fn encode(value: T) -> Result<Bytes, Self::Error> {
        let encoded = rkyv::to_bytes::<rancor::Error>(&value)?;
        Ok(Bytes::copy_from_slice(encoded.as_ref()))
    }
}

impl<T> Decodes<T> for Rkyv
where
    T: Archive + for<'a> Serialize<RkyvSerializer<'a>>,
    T::Archived: Deserialize<T, RkyvDeserializer>
        + for<'a> CheckBytes<RkyvValidator<'a>>,
{
    type Error = rancor::Error;

    fn decode(bytes: Bytes) -> Result<T, Self::Error> {
        let mut aligned = AlignedVec::<1024>::new();
        aligned.extend_from_slice(bytes.as_ref());
        rkyv::from_bytes::<T, rancor::Error>(aligned.as_ref())
    }
}

/// Pass arguments and receive responses as `rkyv` in a `POST` request.
pub type PostRkyv = Post<Rkyv>;
