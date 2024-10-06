use bytes::Bytes;
use http::{uri::{InvalidUri, Parts}, Uri};
use throw_error::Error;

use crate::{
    bindings::wasi::{
        http::types::{IncomingBody, IncomingRequest, Method, Scheme},
        io::streams::StreamError,
    },
    CHUNK_BYTE_SIZE,
};

impl TryFrom<IncomingRequest> for http::Request<Bytes> {
    type Error = Error;

    fn try_from(req: IncomingRequest) -> Result<Self, Self::Error> {
        let mut builder = http::Request::builder();
        let req_method = req.method();
        let headers = req.headers();

        for (header_name, header_value) in headers.entries() {
            builder = builder.header(header_name, header_value);
        }

        drop(headers);

        // NB(raskyld): consume could fail if, for some reason the caller
        // manage to recreate an IncomingRequest backed by the same underlying
        // resource handle (need to dig more to see if that's possible)
        let incoming_body = req
            .consume().expect("could not consume body");

        let body_stream = incoming_body.stream().expect("could not create a stream from body");

        let mut body_bytes =
            Vec::<u8>::with_capacity(CHUNK_BYTE_SIZE);

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

        uri_parts.scheme = req
            .scheme()
            .map(http::uri::Scheme::try_from)
            .transpose()?;
        uri_parts.authority = req
            .authority()
            .map(|aut| {
                http::uri::Authority::from_maybe_shared(aut.into_bytes())
            })
            .transpose()
            .map_err(Error::from)?;
        uri_parts.path_and_query = req
            .path_with_query()
            .map(|paq| {
                http::uri::PathAndQuery::from_maybe_shared(paq.into_bytes())
            })
            .transpose()
            .map_err(Error::from)?;

        drop(body_stream);
        IncomingBody::finish(incoming_body);
        builder
            .method(req_method)
            .uri(
                Uri::from_parts(uri_parts)
                    .map_err(Error::from)?,
            )
            .body(Bytes::from(body_bytes))
            .map_err(Error::from)
    }
}

impl TryFrom<Method> for http::Method {
    type Error = http::method::InvalidMethod;

    fn try_from(value: Method) -> Result<Self, Self::Error> {
        match value {
            Method::Connect => Ok(Self::CONNECT),
            Method::Delete => Ok(Self::DELETE),
            Method::Get => Ok(Self::GET),
            Method::Head => Ok(Self::HEAD),
            Method::Options => Ok(Self::OPTIONS),
            Method::Patch => Ok(Self::PATCH),
            Method::Post => Ok(Self::POST),
            Method::Put => Ok(Self::PUT),
            Method::Trace => Ok(Self::TRACE),
            Method::Other(mtd) => Self::from_bytes(mtd.as_bytes()),
        }
    }
}

impl TryFrom<Scheme> for http::uri::Scheme {
    type Error = InvalidUri;
    fn try_from(value: Scheme) -> Result<Self, Self::Error> {
        match value {
            Scheme::Http => Ok(Self::HTTP),
            Scheme::Https => Ok(Self::HTTPS),
            Scheme::Other(oth) => {
                Self::try_from(oth.as_bytes())
            }
        }
    }
}
