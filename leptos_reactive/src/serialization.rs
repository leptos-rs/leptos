#![forbid(unsafe_code)]
use cfg_if::cfg_if;
use std::rc::Rc;
use thiserror::Error;

/// Describes errors that can occur while serializing and deserializing data,
/// typically during the process of streaming [Resource](crate::Resource)s from
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
/// This is primarily used for serializing and deserializing [Resource](crate::Resource)s
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
    /// Serializes the object to JSON.
    fn to_json(&self) -> Result<String, SerializationError>;

    /// Deserializes the object from JSON.
    fn from_json(json: &str) -> Result<Self, SerializationError>;
}

cfg_if! {
    // prefer miniserde if it's chosen
    if #[cfg(feature = "miniserde")] {
        use miniserde::{json, Deserialize, Serialize};

        impl<T> Serializable for T
        where
            T: Serialize + Deserialize,
        {
            fn to_json(&self) -> Result<String, SerializationError> {
                Ok(json::to_string(&self))
            }

            fn from_json(json: &str) -> Result<Self, SerializationError> {
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
            fn to_json(&self) -> Result<String, SerializationError> {
                let intermediate = self
                    .serialize()
                    .map_err(|e| SerializationError::Serialize(Rc::new(e)))?;
                serde_json::to_string(&intermediate).map_err(|e| SerializationError::Serialize(Rc::new(e)))
            }

            fn from_json(json: &str) -> Result<Self, SerializationError> {
                let intermediate =
                    serde_json::from_str(&json).map_err(|e| SerializationError::Deserialize(Rc::new(e)))?;
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
            fn to_json(&self) -> Result<String, SerializationError> {
                serde_json::to_string(&self).map_err(|e| SerializationError::Serialize(Rc::new(e)))
            }

            fn from_json(json: &str) -> Result<Self, SerializationError> {
                serde_json::from_str(json).map_err(|e| SerializationError::Deserialize(Rc::new(e)))
            }

        }
    }
}
