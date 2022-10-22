use std::rc::Rc;
use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum SerializationError {
    #[error("error serializing Resource: {0}")]
    Serialize(Rc<dyn std::error::Error>),
    #[error("error deserializing Resource: {0}")]
    Deserialize(Rc<dyn std::error::Error>),
}

pub trait Serializable
where
    Self: Sized,
{
    fn to_json(&self) -> Result<String, SerializationError>;

    fn from_json(json: &str) -> Result<Self, SerializationError>;
}

#[cfg(all(
    feature = "serde",
    not(feature = "miniserde"),
    not(feature = "serde-lite")
))]
use serde::{de::DeserializeOwned, Serialize};

#[cfg(all(
    feature = "serde",
    not(feature = "miniserde"),
    not(feature = "serde-lite")
))]
impl<T> Serializable for T
where
    T: DeserializeOwned + Serialize,
{
    fn to_json(&self) -> Result<String, SerializationError> {
        serde_json::to_string(&self).map_err(|e| SerializationError::Serialize(Rc::new(e)))
    }

    fn from_json(json: &str) -> Result<Self, SerializationError> {
        serde_json::from_str(&json).map_err(|e| SerializationError::Deserialize(Rc::new(e)))
    }
}

#[cfg(all(
    feature = "serde-lite",
    not(feature = "serde"),
    not(feature = "miniserde")
))]
use serde_lite::{Deserialize, Serialize};

#[cfg(all(
    feature = "serde-lite",
    not(feature = "serde"),
    not(feature = "miniserde")
))]
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

#[cfg(all(
    feature = "miniserde",
    not(feature = "serde-lite"),
    not(feature = "serde")
))]
use miniserde::{json, Deserialize, Serialize};

#[cfg(all(
    feature = "miniserde",
    not(feature = "serde-lite"),
    not(feature = "serde")
))]
impl<T> Serializable for T
where
    T: Serialize + Deserialize,
{
    fn to_json(&self) -> Result<String, SerializationError> {
        Ok(json::to_string(&self))
    }

    fn from_json(json: &str) -> Result<Self, SerializationError> {
        json::from_str(&json).map_err(|e| SerializationError::Deserialize(Rc::new(e)))
    }
}
