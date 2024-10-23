use bytes::Bytes;
use http::{uri::Parts, Uri};
use thiserror::Error;

use wasi::{
    http::types::{IncomingBody, IncomingRequest, Method, Scheme},
    io::streams::StreamError,
};

use crate::CHUNK_BYTE_SIZE;

pub struct Request(pub IncomingRequest);

impl TryFrom<Request> for http::Request<Bytes> {
    type Error = RequestError;

    fn try_from(req: Request) -> Result<Self, Self::Error> {
        let mut builder = http::Request::builder();
        let req = req.0;
        let req_method = method_wasi_to_http(req.method())?;
        let headers = req.headers();

        for (header_name, header_value) in headers.entries() {
            builder = builder.header(header_name, header_value);
        }

        drop(headers);

        // NB(raskyld): consume could fail if, for some reason the caller
        // manage to recreate an IncomingRequest backed by the same underlying
        // resource handle (need to dig more to see if that's possible)
        let incoming_body = req.consume().expect("could not consume body");

        let body_stream = incoming_body
            .stream()
            .expect("could not create a stream from body");

        let mut body_bytes = Vec::<u8>::with_capacity(CHUNK_BYTE_SIZE);

        loop {
            match body_stream.blocking_read(CHUNK_BYTE_SIZE as u64) {
                Err(StreamError::Closed) => break,
                Err(StreamError::LastOperationFailed(err)) => {
                    return Err(StreamError::LastOperationFailed(err).into())
                }
                Ok(data) => {
                    body_bytes.extend(data);
                }
            }
        }

        let mut uri_parts = Parts::default();

        uri_parts.scheme = req.scheme().map(scheme_wasi_to_http).transpose()?;
        uri_parts.authority = req
            .authority()
            .map(|aut| {
                http::uri::Authority::from_maybe_shared(aut.into_bytes())
            })
            .transpose()
            .map_err(http::Error::from)?;
        uri_parts.path_and_query = req
            .path_with_query()
            .map(|paq| {
                http::uri::PathAndQuery::from_maybe_shared(paq.into_bytes())
            })
            .transpose()
            .map_err(http::Error::from)?;

        drop(body_stream);
        IncomingBody::finish(incoming_body);
        builder
            .method(req_method)
            .uri(Uri::from_parts(uri_parts).map_err(http::Error::from)?)
            .body(Bytes::from(body_bytes))
            .map_err(RequestError::from)
    }
}

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum RequestError {
    #[error("failed to convert wasi bindings to http types")]
    Http(#[from] http::Error),

    #[error("error while processing wasi:http body stream")]
    WasiIo(#[from] wasi::io::streams::StreamError),
}

pub fn method_wasi_to_http(value: Method) -> Result<http::Method, http::Error> {
    match value {
        Method::Connect => Ok(http::Method::CONNECT),
        Method::Delete => Ok(http::Method::DELETE),
        Method::Get => Ok(http::Method::GET),
        Method::Head => Ok(http::Method::HEAD),
        Method::Options => Ok(http::Method::OPTIONS),
        Method::Patch => Ok(http::Method::PATCH),
        Method::Post => Ok(http::Method::POST),
        Method::Put => Ok(http::Method::PUT),
        Method::Trace => Ok(http::Method::TRACE),
        Method::Other(mtd) => {
            http::Method::from_bytes(mtd.as_bytes()).map_err(http::Error::from)
        }
    }
}

pub fn scheme_wasi_to_http(
    value: Scheme,
) -> Result<http::uri::Scheme, http::Error> {
    match value {
        Scheme::Http => Ok(http::uri::Scheme::HTTP),
        Scheme::Https => Ok(http::uri::Scheme::HTTPS),
        Scheme::Other(oth) => http::uri::Scheme::try_from(oth.as_bytes())
            .map_err(http::Error::from),
    }
}
