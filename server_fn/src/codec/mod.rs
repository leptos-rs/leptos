#[cfg(feature = "cbor")]
mod cbor;
#[cfg(feature = "cbor")]
pub use cbor::*;

#[cfg(feature = "json")]
mod json;
#[cfg(feature = "json")]
pub use json::*;

#[cfg(feature = "serde-lite")]
mod serde_lite;
#[cfg(feature = "serde-lite")]
pub use serde_lite::*;

#[cfg(feature = "rkyv")]
mod rkyv;
#[cfg(feature = "rkyv")]
pub use rkyv::*;

#[cfg(feature = "url")]
mod url;
#[cfg(feature = "url")]
pub use url::*;

#[cfg(feature = "multipart")]
mod multipart;
#[cfg(feature = "multipart")]
pub use multipart::*;

mod stream;
use crate::{error::ServerFnError, request::ClientReq};
use futures::Future;
use http::Method;
pub use stream::*;

pub trait FromReq<CustErr, Request, Encoding>
where
    Self: Sized,
{
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
