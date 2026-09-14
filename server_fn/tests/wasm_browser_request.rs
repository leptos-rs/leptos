//! Regression tests for the WASM browser client request builders.
//!
//! These tests assert that `try_new_req_form_data` and
//! `try_new_req_streaming` honour `server_fn::client::set_server_url(...)`.
//! Prior to the fix, these two constructors
//! ignored the configured server URL and built requests against the
//! page's own origin, causing 404/CORS failures for cross-origin clients.
//!
//! The other constructors in the same impl already applied the prefix,
//! so we cover them too as smoke tests to make sure they don't regress.

#![cfg(all(feature = "browser", target_family = "wasm"))]

use bytes::Bytes;
use futures::stream;
use http::Method;
use server_fn::{
    client::set_server_url,
    error::ServerFnError,
    request::{
        ClientReq,
        browser::{BrowserFormData, BrowserRequest},
    },
};
use std::sync::OnceLock;
use wasm_bindgen_test::*;
use web_sys::FormData;

wasm_bindgen_test_configure!(run_in_browser);

const SERVER: &str = "https://api.example.com";
static INIT: OnceLock<()> = OnceLock::new();

fn ensure_server_url_set() {
    INIT.get_or_init(|| set_server_url(SERVER));
}

fn assert_full_url(url: &str, path: &str) {
    assert!(
        url.starts_with(SERVER),
        "expected URL to start with {SERVER:?}, got {url:?}"
    );
    assert!(
        url.ends_with(path),
        "expected URL to end with {path:?}, got {url:?}"
    );
}

#[wasm_bindgen_test]
fn form_data_request_prepends_configured_server_url() {
    ensure_server_url_set();
    let form_data = FormData::new().expect("create FormData");
    let body: BrowserFormData = form_data.into();
    let req =
        <BrowserRequest as ClientReq<ServerFnError>>::try_new_req_form_data(
            "/api/foo_form",
            "*/*",
            "application/x-www-form-urlencoded",
            body,
            Method::POST,
        )
        .expect("should build form-data request");
    assert_full_url(&req.url(), "/api/foo_form");
}

#[wasm_bindgen_test]
fn streaming_request_prepends_configured_server_url() {
    ensure_server_url_set();
    let body = stream::empty::<Bytes>();
    let req =
        <BrowserRequest as ClientReq<ServerFnError>>::try_new_req_streaming(
            "/api/foo_stream",
            "*/*",
            "application/octet-stream",
            body,
            Method::POST,
        )
        .expect("should build streaming request");
    assert_full_url(&req.url(), "/api/foo_stream");
}

#[wasm_bindgen_test]
fn text_request_prepends_configured_server_url() {
    ensure_server_url_set();
    let req = <BrowserRequest as ClientReq<ServerFnError>>::try_new_req_text(
        "/api/foo_text",
        "application/json",
        "*/*",
        "{}".to_string(),
        Method::POST,
    )
    .expect("should build text request");
    assert_full_url(&req.url(), "/api/foo_text");
}

#[wasm_bindgen_test]
fn bytes_request_prepends_configured_server_url() {
    ensure_server_url_set();
    let req = <BrowserRequest as ClientReq<ServerFnError>>::try_new_req_bytes(
        "/api/foo_bytes",
        "application/octet-stream",
        "*/*",
        Bytes::from_static(b""),
        Method::POST,
    )
    .expect("should build bytes request");
    assert_full_url(&req.url(), "/api/foo_bytes");
}
