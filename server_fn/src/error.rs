use core::fmt::Display;
use serde::{Deserialize, Serialize};
use std::{error, fmt, ops, sync::Arc};
use thiserror::Error;

/// This is a result type into which any error can be converted,
/// and which can be used directly in your `view`.
///
/// All errors will be stored as [`struct@Error`].
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

/// An empty value indicating that there is no custom error type associated
/// with this server function.
#[derive(Debug, Deserialize, Serialize)]
pub struct NoCustomError(());

// Implement `Display` for `NoCustomError`
impl fmt::Display for NoCustomError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Unit Type Displayed")
    }
}

#[derive(Debug)]
pub struct WrapError<T>(pub T);

/// This helper macro lets you call the gnarly autoref-specialization call
/// without having to worry about things like how many & you need.
/// Mostly used when you impl From<ServerFnError> for YourError
#[macro_export]
macro_rules! server_fn_error {
    () => {{
        use $crate::{ViaError, WrapError};
        (&&&&&WrapError(())).to_server_error()
    }};
    ($err:expr) => {{
        use $crate::error::{ViaError, WrapError};
        match $err {
            error => (&&&&&WrapError(error)).to_server_error(),
        }
    }};
}

/// This trait serves as the conversion method between a variety of types
/// and [`ServerFnError`].
pub trait ViaError<E> {
    fn to_server_error(&self) -> ServerFnError<E>;
}

// This impl should catch if you fed it a [`ServerFnError`] already.
impl<E: ServerFnErrorKind + std::error::Error + Clone> ViaError<E>
    for &&&&WrapError<ServerFnError<E>>
{
    fn to_server_error(&self) -> ServerFnError<E> {
        self.0.clone()
    }
}

// A type tag for ServerFnError so we can special case it
pub(crate) trait ServerFnErrorKind {}

impl ServerFnErrorKind for ServerFnError {}

// This impl should catch passing () or nothing to server_fn_error
impl ViaError<()> for &&&WrapError<()> {
    fn to_server_error(&self) -> ServerFnError<()> {
        ServerFnError::WrappedServerError(self.0.clone())
    }
}

// This impl will catch any type that implements any type that impls
// Error and Clone, so that it can be wrapped into ServerFnError
impl<E: std::error::Error + Clone> ViaError<E> for &&WrapError<E> {
    fn to_server_error(&self) -> ServerFnError<E> {
        ServerFnError::WrappedServerError(self.0.clone())
    }
}

// If it doesn't impl Error, but does impl Display and Clone,
// we can still wrap it in String form
impl<E: Display + Clone> ViaError<E> for &WrapError<E> {
    fn to_server_error(&self) -> ServerFnError<E> {
        ServerFnError::WrappedServerError(self.0.clone())
    }
}

// This is what happens if someone tries to pass in something that does
// not meet the above criteria
impl<E> ViaError<E> for WrapError<E> {
    #[track_caller]
    fn to_server_error(&self) -> ServerFnError<E> {
        panic!(
            "At {}, you call `to_server_error()` or use  `server_fn_error!` \
             with a value that does not implement `Clone` and either `Error` \
             or `Display`.",
            std::panic::Location::caller()
        );
    }
}

/// Type for errors that can occur when using server functions.
///
/// Unlike [`ServerFnErrorErr`], this does not implement [`Error`](std::error::Error).
/// This means that other error types can easily be converted into it using the
/// `?` operator.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerFnError<E = NoCustomError> {
    WrappedServerError(E),
    /// Error while trying to register the server function (only occurs in case of poisoned RwLock).
    Registration(String),
    /// Occurs on the client if there is a network error while trying to run function on server.
    Request(String),
    /// Occurs on the server if there is an error creating an HTTP response.
    Response(String),
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

impl<CustErr> From<CustErr> for ServerFnError<CustErr> {
    fn from(value: CustErr) -> Self {
        ServerFnError::WrappedServerError(value)
    }
}

impl<CustErr> Display for ServerFnError<CustErr>
where
    CustErr: Display,
{
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
                ServerFnError::Response(s) =>
                    format!("error generating HTTP response: {s}"),
                ServerFnError::WrappedServerError(e) => format!("{}", e),
            }
        )
    }
}
impl<E> std::error::Error for ServerFnError<E>
where
    E: std::error::Error + 'static,
    ServerFnError<E>: std::fmt::Display,
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ServerFnError::WrappedServerError(e) => Some(e),
            _ => None,
        }
    }
}

/// Type for errors that can occur when using server functions.
///
/// Unlike [`ServerFnError`], this implements [`std::error::Error`]. This means
/// it can be used in situations in which the `Error` trait is required, but it’s
/// not possible to create a blanket implementation that converts other errors into
/// this type.
///
/// [`ServerFnError`] and [`ServerFnErrorErr`] mutually implement [`From`], so
/// it is easy to convert between the two types.
#[derive(Error, Debug, Clone)]
pub enum ServerFnErrorErr<E = NoCustomError> {
    #[error("internal error: {0}")]
    WrappedServerError(E),
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
    /// Occurs on the server if there is an error creating an HTTP response.
    #[error("error creating response {0}")]
    Response(String),
}

impl<CustErr> From<ServerFnError<CustErr>> for ServerFnErrorErr<CustErr> {
    fn from(value: ServerFnError<CustErr>) -> Self {
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
            ServerFnError::WrappedServerError(value) => {
                ServerFnErrorErr::WrappedServerError(value)
            }
            ServerFnError::Response(value) => ServerFnErrorErr::Response(value),
        }
    }
}
