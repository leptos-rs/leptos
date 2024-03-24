use core::str::FromStr;
use serde::{de::DeserializeOwned, Serialize};

pub trait SerializableData<Ser: Serializer>: Sized {
    type SerErr;
    type DeErr;

    fn ser(&self) -> Result<String, Self::SerErr>;

    fn de(data: &str) -> Result<Self, Self::DeErr>;
}

pub trait Serializer {}

/// A [`Serializer`] that serializes using [`ToString`] and deserializes
/// using [`FromStr`](core::str::FromStr).
pub struct Str;

impl Serializer for Str {}

impl<T> SerializableData<Str> for T
where
    T: ToString + FromStr,
{
    type SerErr = ();
    type DeErr = <T as FromStr>::Err;

    fn ser(&self) -> Result<String, Self::SerErr> {
        Ok(self.to_string())
    }

    fn de(data: &str) -> Result<Self, Self::DeErr> {
        T::from_str(data)
    }
}

/// A [`Serializer`] that serializes using [`serde_json`].
pub struct SerdeJson;

impl Serializer for SerdeJson {}

impl<T> SerializableData<SerdeJson> for T
where
    T: DeserializeOwned + Serialize,
{
    type SerErr = serde_json::Error;
    type DeErr = serde_json::Error;

    fn ser(&self) -> Result<String, Self::SerErr> {
        serde_json::to_string(&self)
    }

    fn de(data: &str) -> Result<Self, Self::DeErr> {
        serde_json::from_str(data)
    }
}

#[cfg(feature = "miniserde")]
mod miniserde {
    use super::{SerializableData, Serializer};
    use miniserde::{json, Deserialize, Serialize};

    /// A [`Serializer`] that serializes and deserializes using [`miniserde`].
    pub struct Miniserde;

    impl Serializer for Miniserde {}

    impl<T> SerializableData<Miniserde> for T
    where
        T: Deserialize + Serialize,
    {
        type SerErr = ();
        type DeErr = miniserde::Error;

        fn ser(&self) -> Result<String, Self::SerErr> {
            Ok(json::to_string(&self))
        }

        fn de(data: &str) -> Result<Self, Self::DeErr> {
            json::from_str(data)
        }
    }
}
#[cfg(feature = "miniserde")]
pub use miniserde::*;

#[cfg(feature = "serde-lite")]
mod serde_lite {
    use super::{SerializableData, Serializer};
    use serde_lite::{Deserialize, Serialize};
    use thiserror::Error;

    #[derive(Error, Debug)]
    pub enum SerdeLiteError {
        #[error("serde_lite error {0:?}")]
        SerdeLite(serde_lite::Error),
        #[error("serde_json error {0:?}")]
        SerdeJson(serde_json::Error),
    }

    impl From<serde_lite::Error> for SerdeLiteError {
        fn from(value: serde_lite::Error) -> Self {
            SerdeLiteError::SerdeLite(value)
        }
    }

    impl From<serde_json::Error> for SerdeLiteError {
        fn from(value: serde_json::Error) -> Self {
            SerdeLiteError::SerdeJson(value)
        }
    }

    /// A [`Serializer`] that serializes and deserializes using [`serde_lite`].
    pub struct SerdeLite;

    impl Serializer for SerdeLite {}

    impl<T> SerializableData<SerdeLite> for T
    where
        T: Deserialize + Serialize,
    {
        type SerErr = SerdeLiteError;
        type DeErr = SerdeLiteError;

        fn ser(&self) -> Result<String, Self::SerErr> {
            let intermediate = self.serialize()?;
            Ok(serde_json::to_string(&intermediate)?)
        }

        fn de(data: &str) -> Result<Self, Self::DeErr> {
            let intermediate = serde_json::from_str(data)?;
            Ok(Self::deserialize(&intermediate)?)
        }
    }
}
#[cfg(feature = "serde-lite")]
pub use serde_lite::*;

#[cfg(feature = "rkyv")]
mod rkyv {
    use super::{SerializableData, Serializer};
    use base64::{engine::general_purpose::STANDARD_NO_PAD, Engine as _};
    use rkyv::{
        de::deserializers::SharedDeserializeMap,
        ser::serializers::AllocSerializer,
        validation::validators::DefaultValidator, Archive, CheckBytes,
        Deserialize, Serialize,
    };
    use std::{error::Error, sync::Arc};
    use thiserror::Error;

    /// A [`Serializer`] that serializes and deserializes using [`rkyv`].
    pub struct Rkyv;

    impl Serializer for Rkyv {}

    #[derive(Error, Debug)]
    pub enum RkyvError {
        #[error("rkyv error {0:?}")]
        Rkyv(Arc<dyn Error>),
        #[error("base64 error {0:?}")]
        Base64Decode(base64::DecodeError),
    }

    impl From<Arc<dyn Error>> for RkyvError {
        fn from(value: Arc<dyn Error>) -> Self {
            RkyvError::Rkyv(value)
        }
    }

    impl From<base64::DecodeError> for RkyvError {
        fn from(value: base64::DecodeError) -> Self {
            RkyvError::Base64Decode(value)
        }
    }

    impl<T> SerializableData<Rkyv> for T
    where
        T: Serialize<AllocSerializer<1024>>,
        T: Archive,
        T::Archived: for<'b> CheckBytes<DefaultValidator<'b>>
            + Deserialize<T, SharedDeserializeMap>,
    {
        type SerErr = RkyvError;
        type DeErr = RkyvError;

        fn ser(&self) -> Result<String, Self::SerErr> {
            let bytes = rkyv::to_bytes::<T, 1024>(self)
                .map_err(|e| Arc::new(e) as Arc<dyn Error>)?;
            Ok(STANDARD_NO_PAD.encode(bytes))
        }

        fn de(data: &str) -> Result<Self, Self::DeErr> {
            let bytes = STANDARD_NO_PAD.decode(data.as_bytes())?;
            Ok(rkyv::from_bytes::<T>(&bytes)
                .map_err(|e| Arc::new(e) as Arc<dyn Error>)?)
        }
    }
}

#[cfg(feature = "rkyv")]
pub use rkyv::*;

#[cfg(feature = "serde-wasm-bindgen")]
mod serde_wasm_bindgen {
    use super::{SerializableData, Serializer};
    use serde::{de::DeserializeOwned, Serialize};

    /// A [`Serializer`] that serializes using [`serde_json`] and deserializes using
    /// [`serde-wasm-bindgen`].
    pub struct SerdeWasmBindgen;

    impl Serializer for SerdeWasmBindgen {}

    impl<T> SerializableData<SerdeWasmBindgen> for T
    where
        T: DeserializeOwned + Serialize,
    {
        type SerErr = serde_json::Error;
        type DeErr = wasm_bindgen::JsValue;

        fn ser(&self) -> Result<String, Self::SerErr> {
            serde_json::to_string(&self)
        }

        fn de(data: &str) -> Result<Self, Self::DeErr> {
            let json = js_sys::JSON::parse(data)?;
            serde_wasm_bindgen::from_value(json).map_err(Into::into)
        }
    }
}

#[cfg(feature = "serde-wasm-bindgen")]
pub use serde_wasm_bindgen::*;
