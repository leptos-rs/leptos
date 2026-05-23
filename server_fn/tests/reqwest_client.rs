//! Regression tests for the native `reqwest` client request builders.
//!
//! These tests assert that `try_new_req_multipart` and
//! `try_new_req_form_data` honour `server_fn::client::set_server_url(...)`.
//! Prior to the fix, those two constructors
//! passed the bare `path` to `reqwest::Client::post/put/patch`, which
//! caused `reqwest::Url::parse` to fail with `RelativeUrlWithoutBase`
//! (or send to the wrong host if `path` happened to be absolute).
//!
//! Sibling constructors (`try_new_req_query`, `try_new_req_text`,
//! `try_new_req_bytes`, `try_new_req_streaming`) already applied the
//! prefix; they are covered here as smoke tests.

#![cfg(feature = "reqwest")]

use bytes::Bytes;
use futures::stream;
use http::Method;
use server_fn::{
    client::set_server_url,
    error::ServerFnError,
    request::{ClientReq, reqwest::Form},
};
use std::sync::OnceLock;

type Req = reqwest::Request;

const SERVER: &str = "https://api.example.com";
static INIT: OnceLock<()> = OnceLock::new();

fn ensure_server_url_set() {
    INIT.get_or_init(|| set_server_url(SERVER));
}

fn assert_full_url(actual: &reqwest::Url, expected_path: &str) {
    let actual_str = actual.as_str();
    let expected_prefix = format!("{SERVER}{expected_path}");
    assert!(
        actual_str == expected_prefix
            || actual_str.starts_with(&expected_prefix),
        "expected URL {actual_str:?} to start with {expected_prefix:?}"
    );
}

#[test]
fn multipart_request_prepends_configured_server_url() {
    ensure_server_url_set();
    let req = <Req as ClientReq<ServerFnError>>::try_new_req_multipart(
        "/api/foo_multipart",
        "*/*",
        Form::new(),
        Method::POST,
    )
    .expect("should build multipart request");
    assert_full_url(req.url(), "/api/foo_multipart");
}

#[test]
fn form_data_request_prepends_configured_server_url() {
    ensure_server_url_set();
    let req = <Req as ClientReq<ServerFnError>>::try_new_req_form_data(
        "/api/foo_form",
        "*/*",
        "application/x-www-form-urlencoded",
        Form::new(),
        Method::POST,
    )
    .expect("should build form-data request");
    assert_full_url(req.url(), "/api/foo_form");
}

#[test]
fn query_request_prepends_configured_server_url() {
    ensure_server_url_set();
    let req = <Req as ClientReq<ServerFnError>>::try_new_req_query(
        "/api/foo_query",
        "application/json",
        "*/*",
        "x=1",
        Method::GET,
    )
    .expect("should build query request");
    let url = req.url();
    let url_str = url.as_str();
    assert!(
        url_str.starts_with(&format!("{SERVER}/api/foo_query")),
        "expected URL to start with server URL + path, got {url_str:?}"
    );
    assert_eq!(url.query(), Some("x=1"));
}

#[test]
fn text_request_prepends_configured_server_url() {
    ensure_server_url_set();
    let req = <Req as ClientReq<ServerFnError>>::try_new_req_text(
        "/api/foo_text",
        "application/json",
        "*/*",
        "{}".to_string(),
        Method::POST,
    )
    .expect("should build text request");
    assert_full_url(req.url(), "/api/foo_text");
}

#[test]
fn bytes_request_prepends_configured_server_url() {
    ensure_server_url_set();
    let req = <Req as ClientReq<ServerFnError>>::try_new_req_bytes(
        "/api/foo_bytes",
        "application/octet-stream",
        "*/*",
        Bytes::from_static(b""),
        Method::POST,
    )
    .expect("should build bytes request");
    assert_full_url(req.url(), "/api/foo_bytes");
}

#[test]
fn streaming_request_prepends_configured_server_url() {
    ensure_server_url_set();
    let body = stream::empty::<Bytes>();
    let req = <Req as ClientReq<ServerFnError>>::try_new_req_streaming(
        "/api/foo_stream",
        "*/*",
        "application/octet-stream",
        body,
        Method::POST,
    )
    .expect("should build streaming request");
    assert_full_url(req.url(), "/api/foo_stream");
}
