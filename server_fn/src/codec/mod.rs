#[cfg(feature = "cbor")]
mod cbor;
#[cfg(feature = "cbor")]
pub use cbor::*;
#[cfg(feature = "json")]
mod json;
use http::Method;
#[cfg(feature = "json")]
pub use json::*;
#[cfg(feature = "rkyv")]
mod rkyv;
#[cfg(feature = "rkyv")]
pub use rkyv::*;
#[cfg(feature = "url")]
mod url;
use crate::{client::Client, error::ServerFnError, request::ClientReq};
use futures::Future;
#[cfg(feature = "url")]
pub use url::*;

#[cfg(feature = "multipart")]
mod multipart;
#[cfg(feature = "multipart")]
pub use multipart::*;

mod stream;
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
