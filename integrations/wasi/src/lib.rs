use std::sync::Arc;

use bindings::wasi::http::types::{Fields, IncomingRequest, OutgoingResponse};
use http::{HeaderMap, StatusCode, HeaderName, HeaderValue};
use hydration_context::PinnedStream;
use parking_lot::RwLock;

pub mod bindings {
    wit_bindgen::generate!({
        path: "wit",
        pub_export_macro: true,
        world: "http",
        generate_all,
    });
}

pub mod server_fn;

pub struct WasiRequest(pub IncomingRequest);

pub struct WasiResponse {
    fields: Fields,
    resp: OutgoingResponse,
    
    /// Optional stream to consume to produce the response,
    /// the tachys crate seems to produce String stream so we use
    /// the same here. If it is set, the stream is consumed and the
    /// chunks are appended to the body of resp.
    stream: Option<PinnedStream<String>>,
}

/// This struct lets you define headers and override the status of the Response from an Element or a Server Function
/// Typically contained inside of a ResponseOptions. Setting this is useful for cookies and custom responses.
#[derive(Debug, Clone, Default)]
pub struct ResponseParts {
    pub headers: HeaderMap,
    pub status: Option<StatusCode>,
}

/// Allows you to override details of the HTTP response like the status code and add Headers/Cookies.
#[derive(Debug, Clone, Default)]
pub struct ResponseOptions(Arc<RwLock<ResponseParts>>);

impl ResponseOptions {
    /// A simpler way to overwrite the contents of `ResponseOptions` with a new `ResponseParts`.
    pub fn overwrite(&self, parts: ResponseParts) {
        let mut writable = self.0.write();
        *writable = parts
    }
    /// Set the status of the returned Response.
    pub fn set_status(&self, status: StatusCode) {
        let mut writeable = self.0.write();
        let res_parts = &mut *writeable;
        res_parts.status = Some(status);
    }
    /// Insert a header, overwriting any previous value with the same key.
    pub fn insert_header(&self, key: HeaderName, value: HeaderValue) {
        let mut writeable = self.0.write();
        let res_parts = &mut *writeable;
        res_parts.headers.insert(key, value);
    }
    /// Append a header, leaving any header with the same key intact.
    pub fn append_header(&self, key: HeaderName, value: HeaderValue) {
        let mut writeable = self.0.write();
        let res_parts = &mut *writeable;
        res_parts.headers.append(key, value);
    }
}


