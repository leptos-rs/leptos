use base64::{engine::general_purpose::URL_SAFE, Engine as _};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::fmt::{self, Display};
use thiserror::Error;
use throw_error::Error;
use url::Url;

/// A custom header that can be used to indicate a server function returned an error.
pub const SERVER_FN_ERROR_HEADER: &str = "serverfnerror";

impl From<ServerFnError> for Error {
    fn from(e: ServerFnError) -> Self {
        Error::from(ServerFnErrorWrapper(e))
    }
}

/// Type for errors that can occur when using server functions.
/// This type is intended to be used as the return type of the server function for easy error conversion with `?` operator.
///
/// Unlike [`ServerFnErrorErr`], this does not implement [`Error`](trait@std::error::Error).
/// This means that other error types can easily be converted into it using the
/// `?` operator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
pub enum ServerFnError {
    /// Error while trying to register the server function (only occurs in case of poisoned RwLock).
    Registration(String),
    /// Occurs on the client if there is a network error while trying to run function on server.
    Request(String),
    /// Occurs on the server if there is an error creating an HTTP response.
    Response(String),
    /// Occurs when there is an error while actually running the function on the server.
    ServerError(String),
    /// Occurs when there is an error while actually running the middleware on the server.
    MiddlewareError(String),
    /// Occurs on the client if there is an error deserializing the server's response.
    Deserialization(String),
    /// Occurs on the client if there is an error serializing the server function arguments.
    Serialization(String),
    /// Occurs on the server if there is an error deserializing one of the arguments that's been sent.
    Args(String),
    /// Occurs on the server if there's a missing argument.
    MissingArg(String),
}

impl ServerFnError {
    /// Constructs a new [`ServerFnError::ServerError`] from some other type.
    pub fn new(msg: impl ToString) -> Self {
        Self::ServerError(msg.to_string())
    }
}

impl<E: std::error::Error> From<E> for ServerFnError {
    fn from(value: E) -> Self {
        ServerFnError::ServerError(value.to_string())
    }
}

impl Display for ServerFnError {
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
                ServerFnError::MiddlewareError(s) =>
                    format!("error running middleware: {s}"),
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
            }
        )
    }
}

#[doc(hidden)]
/// An extension trait for types that can be serialized and deserialized to a [`String`].
pub trait ServerFnErrorSerde: Sized {
    /// The error type that can occur when serializing the custom error type.
    type Error: std::error::Error;

    /// Converts the custom error type to a [`String`].
    fn ser(&self) -> Result<String, Self::Error>;

    /// Deserializes the custom error type from a [`String`].
    fn de(data: &str) -> Self;
}

/// Type for errors that can occur when using server functions. If you need to return a custom error type from a server function, implement `From<ServerFnErrorErr>` for your custom error type.
///
/// Unlike [`ServerFnError`], this implements [`std::error::Error`]. This means
/// it can be used in situations in which the `Error` trait is required, but it’s
/// not possible to create a blanket implementation that converts other errors into
/// this type.
///
/// [`ServerFnError`] and [`ServerFnErrorErr`] mutually implement [`From`], so
/// it is easy to convert between the two types.
#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
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
    /// Occurs when there is an error while actually running the middleware on the server.
    #[error("error running middleware: {0}")]
    MiddlewareError(String),
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

/// Associates a particular server function error with the server function
/// found at a particular path.
///
/// This can be used to pass an error from the server back to the client
/// without JavaScript/WASM supported, by encoding it in the URL as a query string.
/// This is useful for progressive enhancement.
#[derive(Debug)]
pub struct ServerFnUrlError<E> {
    path: String,
    error: E,
}

impl<E: FromServerFnError> ServerFnUrlError<E> {
    /// Creates a new structure associating the server function at some path
    /// with a particular error.
    pub fn new(path: impl Display, error: E) -> Self {
        Self {
            path: path.to_string(),
            error,
        }
    }

    /// The error itself.
    pub fn error(&self) -> &E {
        &self.error
    }

    /// The path of the server function that generated this error.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Adds an encoded form of this server function error to the given base URL.
    pub fn to_url(&self, base: &str) -> Result<Url, url::ParseError> {
        let mut url = Url::parse(base)?;
        url.query_pairs_mut()
            .append_pair("__path", &self.path)
            .append_pair(
                "__err",
                &URL_SAFE.encode(self.error.ser().unwrap_or_default()),
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

    /// Decodes an error from a URL.
    pub fn decode_err(err: &str) -> E {
        let decoded = match URL_SAFE.decode(err) {
            Ok(decoded) => decoded,
            Err(err) => {
                return ServerFnErrorErr::Deserialization(err.to_string())
                    .into()
            }
        };
        let s = match String::from_utf8(decoded) {
            Ok(s) => s,
            Err(err) => {
                return ServerFnErrorErr::Deserialization(err.to_string())
                    .into()
            }
        };
        E::de(&s)
    }
}

#[derive(Debug)]
#[doc(hidden)]
/// Only used instantly only when a framework needs E: Error.
pub struct ServerFnErrorWrapper<E>(pub E);

impl<E: FromServerFnError> Display for ServerFnErrorWrapper<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<E: FromServerFnError> std::error::Error for ServerFnErrorWrapper<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

/// A trait for types that can be returned from a server function.
pub trait FromServerFnError:
    Display
    + std::fmt::Debug
    + From<ServerFnErrorErr>
    + Serialize
    + DeserializeOwned
    + for<'a> Deserialize<'a>
    + 'static
{
}

#[test]
fn assert_from_server_fn_error_impl() {
    fn assert_impl<T: FromServerFnError>() {}

    assert_impl::<ServerFnError>();
}

impl<E> FromServerFnError for E where
    E: Display
        + std::fmt::Debug
        + From<ServerFnErrorErr>
        + Serialize
        + DeserializeOwned
        + for<'a> Deserialize<'a>
        + 'static
{
}

impl<E> ServerFnErrorSerde for E
where
    E: FromServerFnError,
{
    type Error = serde_json::Error;

    fn ser(&self) -> Result<String, Self::Error> {
        serde_json::to_string(self)
    }

    fn de(data: &str) -> Self {
        serde_json::from_str(data).unwrap_or_else(|e| {
            ServerFnErrorErr::Deserialization(e.to_string()).into()
        })
    }
}
