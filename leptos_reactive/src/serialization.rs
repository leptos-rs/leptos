#![forbid(unsafe_code)]
use cfg_if::cfg_if;
use std::rc::Rc;
use thiserror::Error;

/// Describes errors that can occur while serializing and deserializing data,
/// typically during the process of streaming [`Resource`](crate::Resource)s from
/// the server to the client.
#[derive(Debug, Clone, Error)]
pub enum SerializationError {
    /// Errors that occur during serialization.
    #[error("error serializing Resource: {0}")]
    Serialize(Rc<dyn std::error::Error>),
    /// Errors that occur during deserialization.
    #[error("error deserializing Resource: {0}")]
    Deserialize(Rc<dyn std::error::Error>),
}

/// Describes an object that can be serialized to or from a supported format
/// Currently those are JSON and Cbor
///
/// This is primarily used for serializing and deserializing [`Resource`](crate::Resource)s
/// so they can begin on the server and be resolved on the client, but can be used
/// for any data that needs to be serialized/deserialized.
///
/// This trait is intended to abstract over various serialization crates,
/// as selected between by the crate features `serde` (default), `serde-lite`,
/// and `miniserde`.
pub trait Serializable
where
    Self: Sized,
{
    /// Serializes the object to a string.
    fn ser(&self) -> Result<String, SerializationError>;

    /// Deserializes the object from some bytes.
    fn de(bytes: &str) -> Result<Self, SerializationError>;
}

cfg_if! {
    if #[cfg(feature = "rkyv")] {
        use rkyv::{Archive, CheckBytes, Deserialize, Serialize, ser::serializers::AllocSerializer, de::deserializers::SharedDeserializeMap, validation::validators::DefaultValidator};
        use base64::Engine as _;
        use base64::engine::general_purpose::STANDARD_NO_PAD;

        impl<T> Serializable for T
        where
        T: Serialize<AllocSerializer<1024>>,
        T: Archive,
        T::Archived: for<'b> CheckBytes<DefaultValidator<'b>> + Deserialize<T, SharedDeserializeMap>,
        {
            fn ser(&self) -> Result<String, SerializationError> {
                let bytes = rkyv::to_bytes::<T, 1024>(self).map_err(|e| SerializationError::Serialize(Rc::new(e)))?;
                Ok(STANDARD_NO_PAD.encode(bytes))
            }

            fn de(serialized: &str) -> Result<Self, SerializationError> {
                let bytes = STANDARD_NO_PAD.decode(serialized.as_bytes()).map_err(|e| SerializationError::Deserialize(Rc::new(e)))?;
                rkyv::from_bytes::<T>(&bytes).map_err(|e| SerializationError::Deserialize(Rc::new(e)))
            }
        }
    }
    // prefer miniserde if it's chosen
    else if #[cfg(feature = "miniserde")] {
        use miniserde::{json, Deserialize, Serialize};

        impl<T> Serializable for T
        where
            T: Serialize + Deserialize,
        {
            fn ser(&self) -> Result<String, SerializationError> {
                Ok(json::to_string(&self))
            }

            fn de(json: &str) -> Result<Self, SerializationError> {
                json::from_str(json).map_err(|e| SerializationError::Deserialize(Rc::new(e)))
            }
        }

    }
    // use serde-lite if enabled
    else if #[cfg(feature = "serde-lite")] {
        use serde_lite::{Deserialize, Serialize};

        impl<T> Serializable for T
        where
            T: Serialize + Deserialize,
        {
            fn ser(&self) -> Result<String, SerializationError> {
                let intermediate = self
                    .serialize()
                    .map_err(|e| SerializationError::Serialize(Rc::new(e)))?;
                serde_json::to_string(&intermediate).map_err(|e| SerializationError::Serialize(Rc::new(e)))
            }

            fn de(json: &str) -> Result<Self, SerializationError> {
                let intermediate =
                    serde_json::from_str(json).map_err(|e| SerializationError::Deserialize(Rc::new(e)))?;
                Self::deserialize(&intermediate).map_err(|e| SerializationError::Deserialize(Rc::new(e)))
            }
        }

    }
    // otherwise, or if serde is chosen, default to serde
    else {
        use serde::{de::DeserializeOwned, Serialize};

        impl<T> Serializable for T
        where
            T: DeserializeOwned + Serialize,
        {
            fn ser(&self) -> Result<String, SerializationError> {
                serde_json::to_string(&self).map_err(|e| SerializationError::Serialize(Rc::new(e)))
            }

            fn de(json: &str) -> Result<Self, SerializationError> {
                serde_json::from_str(json).map_err(|e| SerializationError::Deserialize(Rc::new(e)))
            }

        }
    }
}
