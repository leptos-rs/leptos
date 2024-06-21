use serde::{Deserialize, Serialize};
use std::{
    error, fmt,
    fmt::{Display, Write},
    ops,
    str::FromStr,
    sync::Arc,
};
use thiserror::Error;
use url::Url;

/// A custom header that can be used to indicate a server function returned an error.
pub const SERVER_FN_ERROR_HEADER: &str = "serverfnerror";

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
#[derive(
    Debug,
    Deserialize,
    Serialize,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Clone,
    Copy,
)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
pub struct NoCustomError;

// Implement `Display` for `NoCustomError`
impl fmt::Display for NoCustomError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Unit Type Displayed")
    }
}

impl FromStr for NoCustomError {
    type Err = ();

    fn from_str(_s: &str) -> Result<Self, Self::Err> {
        Ok(NoCustomError)
    }
}

/// Wraps some error type, which may implement any of [`Error`](trait@std::error::Error), [`Clone`], or
/// [`Display`].
#[derive(Debug)]
pub struct WrapError<T>(pub T);

/// A helper macro to convert a variety of different types into `ServerFnError`.
/// This should mostly be used if you are implementing `From<ServerFnError>` for `YourError`.
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
    /// Converts something into an error.
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
impl ViaError<NoCustomError> for &&&WrapError<()> {
    fn to_server_error(&self) -> ServerFnError {
        ServerFnError::WrappedServerError(NoCustomError)
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
        ServerFnError::ServerError(self.0.to_string())
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
/// Unlike [`ServerFnErrorErr`], this does not implement [`Error`](trait@std::error::Error).
/// This means that other error types can easily be converted into it using the
/// `?` operator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
pub enum ServerFnError<E = NoCustomError> {
    /// A user-defined custom error type, which defaults to [`NoCustomError`].
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

impl ServerFnError<NoCustomError> {
    /// Constructs a new [`ServerFnError::ServerError`] from some other type.
    pub fn new(msg: impl ToString) -> Self {
        Self::ServerError(msg.to_string())
    }
}

impl<CustErr> From<CustErr> for ServerFnError<CustErr> {
    fn from(value: CustErr) -> Self {
        ServerFnError::WrappedServerError(value)
    }
}

impl<E: std::error::Error> From<E> for ServerFnError {
    fn from(value: E) -> Self {
        ServerFnError::ServerError(value.to_string())
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
                ServerFnError::WrappedServerError(e) => format!("{e}"),
            }
        )
    }
}

/// A serializable custom server function error type.
///
/// This is implemented for all types that implement [`FromStr`] + [`Display`].
///
/// This means you do not necessarily need the overhead of `serde` for a custom error type.
/// Instead, you can use something like `strum` to derive `FromStr` and `Display` for your
/// custom error type.
///
/// This is implemented for the default [`ServerFnError`], which uses [`NoCustomError`].
pub trait ServerFnErrorSerde: Sized {
    /// Converts the custom error type to a [`String`].
    fn ser(&self) -> Result<String, std::fmt::Error>;

    /// Deserializes the custom error type from a [`String`].
    fn de(data: &str) -> Self;
}

impl<CustErr> ServerFnErrorSerde for ServerFnError<CustErr>
where
    CustErr: FromStr + Display,
{
    fn ser(&self) -> Result<String, std::fmt::Error> {
        let mut buf = String::new();
        match self {
            ServerFnError::WrappedServerError(e) => {
                write!(&mut buf, "WrappedServerFn|{e}")
            }
            ServerFnError::Registration(e) => {
                write!(&mut buf, "Registration|{e}")
            }
            ServerFnError::Request(e) => write!(&mut buf, "Request|{e}"),
            ServerFnError::Response(e) => write!(&mut buf, "Response|{e}"),
            ServerFnError::ServerError(e) => {
                write!(&mut buf, "ServerError|{e}")
            }
            ServerFnError::Deserialization(e) => {
                write!(&mut buf, "Deserialization|{e}")
            }
            ServerFnError::Serialization(e) => {
                write!(&mut buf, "Serialization|{e}")
            }
            ServerFnError::Args(e) => write!(&mut buf, "Args|{e}"),
            ServerFnError::MissingArg(e) => {
                write!(&mut buf, "MissingArg|{e}")
            }
        }?;
        Ok(buf)
    }

    fn de(data: &str) -> Self {
        data.split_once('|')
            .and_then(|(ty, data)| match ty {
                "WrappedServerFn" => match CustErr::from_str(data) {
                    Ok(d) => Some(ServerFnError::WrappedServerError(d)),
                    Err(_) => None,
                },
                "Registration" => {
                    Some(ServerFnError::Registration(data.to_string()))
                }
                "Request" => Some(ServerFnError::Request(data.to_string())),
                "Response" => Some(ServerFnError::Response(data.to_string())),
                "ServerError" => {
                    Some(ServerFnError::ServerError(data.to_string()))
                }
                "Deserialization" => {
                    Some(ServerFnError::Deserialization(data.to_string()))
                }
                "Serialization" => {
                    Some(ServerFnError::Serialization(data.to_string()))
                }
                "Args" => Some(ServerFnError::Args(data.to_string())),
                "MissingArg" => {
                    Some(ServerFnError::MissingArg(data.to_string()))
                }
                _ => None,
            })
            .unwrap_or_else(|| {
                ServerFnError::Deserialization(format!(
                    "Could not deserialize error {data:?}"
                ))
            })
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
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ServerFnErrorErr<E = NoCustomError> {
    /// A user-defined custom error type, which defaults to [`NoCustomError`].
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

/// Associates a particular server function error with the server function
/// found at a particular path.
///
/// This can be used to pass an error from the server back to the client
/// without JavaScript/WASM supported, by encoding it in the URL as a query string.
/// This is useful for progressive enhancement.
#[derive(Debug)]
pub struct ServerFnUrlError<CustErr> {
    path: String,
    error: ServerFnError<CustErr>,
}

impl<CustErr> ServerFnUrlError<CustErr> {
    /// Creates a new structure associating the server function at some path
    /// with a particular error.
    pub fn new(path: impl Display, error: ServerFnError<CustErr>) -> Self {
        Self {
            path: path.to_string(),
            error,
        }
    }

    /// The error itself.
    pub fn error(&self) -> &ServerFnError<CustErr> {
        &self.error
    }

    /// The path of the server function that generated this error.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Adds an encoded form of this server function error to the given base URL.
    pub fn to_url(&self, base: &str) -> Result<Url, url::ParseError>
    where
        CustErr: FromStr + Display,
    {
        let mut url = Url::parse(base)?;
        url.query_pairs_mut()
            .append_pair("__path", &self.path)
            .append_pair(
                "__err",
                &ServerFnErrorSerde::ser(&self.error).unwrap_or_default(),
            );
        Ok(url)
    }

    /// Replaces any ServerFnUrlError info from the URL in the given string
    /// with the serialized success value given.
    pub fn strip_error_info(path: &mut String) {
        if let Ok(mut url) = Url::parse(&*path) {
            // NOTE: This is gross, but the Serializer you get from
            // .query_pairs_mut() isn't an Iterator so you can't just .retain().
            let pairs_previously = url
                .query_pairs()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect::<Vec<_>>();
            let mut pairs = url.query_pairs_mut();
            pairs.clear();
            for (key, value) in pairs_previously
                .into_iter()
                .filter(|(key, _)| key != "__path" && key != "__err")
            {
                pairs.append_pair(&key, &value);
            }
            drop(pairs);
            *path = url.to_string();
        }
    }
}

impl<CustErr> From<ServerFnUrlError<CustErr>> for ServerFnError<CustErr> {
    fn from(error: ServerFnUrlError<CustErr>) -> Self {
        error.error
    }
}

impl<CustErr> From<ServerFnUrlError<CustErr>> for ServerFnErrorErr<CustErr> {
    fn from(error: ServerFnUrlError<CustErr>) -> Self {
        error.error.into()
    }
}
