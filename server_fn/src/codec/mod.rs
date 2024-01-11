//! The serialization/deserialization process for server functions consists of a series of steps,
//! each of which is represented by a different trait:
//! 1. [`IntoReq`]: The client serializes the [`ServerFn`] argument type into an HTTP request.
//! 2. The [`Client`] sends the request to the server.
//! 3. [`FromReq`]: The server deserializes the HTTP request back into the [`ServerFn`] type.
//! 4. The server calls calls [`ServerFn::run_body`] on the data.
//! 5. [`IntoRes`]: The server serializes the [`ServerFn::Output`] type into an HTTP response.
//! 6. The server integration applies any middleware from [`ServerFn::middlewares`] and responds to the request.
//! 7. [`FromRes`]: The client deserializes the response back into the [`ServerFn::Output`] type.
//!
//! Rather than a limited number of encodings, this crate allows you to define server functions that
//! mix and match the input encoding and output encoding. To define a new encoding, you simply implement
//! an input combination ([`IntoReq`] and [`FromReq`]) and/or an output encoding ([`IntoRes`] and [`FromRes`]).
//! This genuinely is an and/or: while some encodings can be used for both input and output ([`Json`], [`Cbor`], [`Rkyv`]),
//! others can only be used for input ([`GetUrl`], [`MultipartData`]) or only output ([`ByteStream`], [`StreamingText`]).

#[cfg(feature = "cbor")]
mod cbor;
#[cfg(any(feature = "cbor", doc))]
pub use cbor::*;

#[cfg(feature = "json")]
mod json;
#[cfg(any(feature = "json", doc))]
pub use json::*;

#[cfg(feature = "serde-lite")]
mod serde_lite;
#[cfg(any(feature = "serde-lite", doc))]
pub use serde_lite::*;

#[cfg(feature = "rkyv")]
mod rkyv;
#[cfg(any(feature = "rkyv", doc))]
pub use rkyv::*;

#[cfg(feature = "url")]
mod url;
#[cfg(any(feature = "url", doc))]
pub use url::*;

#[cfg(feature = "multipart")]
mod multipart;
#[cfg(any(feature = "multipart", doc))]
pub use multipart::*;

mod stream;
use crate::{error::ServerFnError, request::ClientReq};
use futures::Future;
use http::Method;
pub use stream::*;

/// Deserializes an HTTP request into the data type.
///
/// Implementations use the methods of the [`Req`](crate::Req) trait to access whatever is
/// needed from the request.
///
/// For example, here’s the implementation for [`Json`].
///
/// ```rust
/// impl<CustErr, T, Request> FromReq<CustErr, Request, Json> for T
/// where
///     // require the Request implement `Req`
///     Request: Req<CustErr> + Send + 'static,
///     // require that the type can be deserialized with `serde`
///     T: DeserializeOwned,
/// {
///     async fn from_req(
///         req: Request,
///     ) -> Result<Self, ServerFnError<CustErr>> {
///         // try to convert the body of the request into a `String`
///         let string_data = req.try_into_string().await?;
///         // deserialize the data
///         serde_json::from_str::<Self>(&string_data)
///             .map_err(|e| ServerFnError::Args(e.to_string()))
///     }
/// }
/// ```
pub trait FromReq<CustErr, Request, Encoding>
where
    Self: Sized,
{
    /// Attempts to deserialize the request.
    fn from_req(
        req: Request,
    ) -> impl Future<Output = Result<Self, ServerFnError<CustErr>>> + Send;
}

pub trait IntoReq<CustErr, Request, Encoding> {
    fn into_req(
        self,
        path: &str,
        accepts: &str,
    ) -> Result<Request, ServerFnError<CustErr>>;
}

pub trait FromRes<CustErr, Response, Encoding>
where
    Self: Sized,
{
    fn from_res(
        res: Response,
    ) -> impl Future<Output = Result<Self, ServerFnError<CustErr>>> + Send;
}

pub trait IntoRes<CustErr, Response, Encoding> {
    fn into_res(
        self,
    ) -> impl Future<Output = Result<Response, ServerFnError<CustErr>>> + Send;
}

pub trait Encoding {
    const CONTENT_TYPE: &'static str;
    const METHOD: Method;
}

pub trait FormDataEncoding<Client, CustErr, Request>
where
    Self: Sized,
    Client: ClientReq<CustErr>,
{
    fn form_data_into_req(
        form_data: Client::FormData,
    ) -> Result<Self, ServerFnError<CustErr>>;
}
