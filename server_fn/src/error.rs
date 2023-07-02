use serde::{Deserialize, Serialize};
use std::{error, fmt, ops, sync::Arc};
use thiserror::Error;

/// This is a result type into which any error can be converted,
/// and which can be used directly in your `view`.
///
/// All errors will be stored as [`Error`].
pub type Result<T, E = Error> = core::result::Result<T, E>;

/// A generic wrapper for any error.
#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct Error(Arc<dyn error::Error + Send + Sync>);

impl Error {
    /// Converts the wrapper into the inner reference-counted error.
    pub fn into_inner(self) -> Arc<dyn error::Error + Send + Sync> {
        Arc::clone(&self.0)
    }
}

impl ops::Deref for Error {
    type Target = Arc<dyn error::Error + Send + Sync>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<T> From<T> for Error
where
    T: std::error::Error + Send + Sync + 'static,
{
    fn from(value: T) -> Self {
        Error(Arc::new(value))
    }
}

impl From<ServerFnError> for Error {
    fn from(e: ServerFnError) -> Self {
        Error(Arc::new(ServerFnErrorErr::from(e)))
    }
}

/// Type for errors that can occur when using server functions.
///
/// Unlike [`ServerFnErrorErr`], this does not implement [`std::error::Error`].
/// This means that other error types can easily be converted into it using the
/// `?` operator.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ServerFnError {
    /// Error while trying to register the server function (only occurs in case of poisoned RwLock).
    Registration(String),
    /// Occurs on the client if there is a network error while trying to run function on server.
    Request(String),
    /// Occurs when there is an error while actually running the function on the server.
    ServerError(String),
    /// Occurs on the client if there is an error deserializing the server's response.
    Deserialization(String),
    /// Occurs on the client if there is an error serializing the server function arguments.
    Serialization(String),
    /// Occurs on the server if there is an error deserializing one of the arguments that's been sent.
    Args(String),
    /// Occurs on the server if there's a missing argument.
    MissingArg(String),
}

impl std::fmt::Display for ServerFnError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ServerFnError::Registration(s) => format!(
                    "error while trying to register the server function: {s}"
                ),
                ServerFnError::Request(s) => format!(
                    "error reaching server to call server function: {s}"
                ),
                ServerFnError::ServerError(s) =>
                    format!("error running server function: {s}"),
                ServerFnError::Deserialization(s) =>
                    format!("error deserializing server function results: {s}"),
                ServerFnError::Serialization(s) =>
                    format!("error serializing server function arguments: {s}"),
                ServerFnError::Args(s) => format!(
                    "error deserializing server function arguments: {s}"
                ),
                ServerFnError::MissingArg(s) => format!("missing argument {s}"),
            }
        )
    }
}

impl<E> From<E> for ServerFnError
where
    E: std::error::Error,
{
    fn from(e: E) -> Self {
        ServerFnError::ServerError(e.to_string())
    }
}

/// Type for errors that can occur when using server functions.
///
/// Unlike [`ServerFnErrorErr`], this implements [`std::error::Error`]. This means
/// it can be used in situations in which the `Error` trait is required, but it’s
/// not possible to create a blanket implementation that converts other errors into
/// this type.
///
/// [`ServerFnError`] and [`ServerFnErrorErr`] mutually implement [`From`], so
/// it is easy to convert between the two types.
#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum ServerFnErrorErr {
    /// Error while trying to register the server function (only occurs in case of poisoned RwLock).
    #[error("error while trying to register the server function: {0}")]
    Registration(String),
    /// Occurs on the client if there is a network error while trying to run function on server.
    #[error("error reaching server to call server function: {0}")]
    Request(String),
    /// Occurs when there is an error while actually running the function on the server.
    #[error("error running server function: {0}")]
    ServerError(String),
    /// Occurs on the client if there is an error deserializing the server's response.
    #[error("error deserializing server function results: {0}")]
    Deserialization(String),
    /// Occurs on the client if there is an error serializing the server function arguments.
    #[error("error serializing server function arguments: {0}")]
    Serialization(String),
    /// Occurs on the server if there is an error deserializing one of the arguments that's been sent.
    #[error("error deserializing server function arguments: {0}")]
    Args(String),
    /// Occurs on the server if there's a missing argument.
    #[error("missing argument {0}")]
    MissingArg(String),
}

impl From<ServerFnError> for ServerFnErrorErr {
    fn from(value: ServerFnError) -> Self {
        match value {
            ServerFnError::Registration(value) => {
                ServerFnErrorErr::Registration(value)
            }
            ServerFnError::Request(value) => ServerFnErrorErr::Request(value),
            ServerFnError::ServerError(value) => {
                ServerFnErrorErr::ServerError(value)
            }
            ServerFnError::Deserialization(value) => {
                ServerFnErrorErr::Deserialization(value)
            }
            ServerFnError::Serialization(value) => {
                ServerFnErrorErr::Serialization(value)
            }
            ServerFnError::Args(value) => ServerFnErrorErr::Args(value),
            ServerFnError::MissingArg(value) => {
                ServerFnErrorErr::MissingArg(value)
            }
        }
    }
}
