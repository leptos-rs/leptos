#![cfg(feature = "axum")]

//! Checks that a request body over `DefaultBodyLimit` logs one WARN with the
//! `server_fn::body_limit` target and the request path. It has its own test
//! binary because tracing caches callsite interest per process: a parallel
//! test thread without a subscriber that first reaches the same `warn!` would
//! disable it for this test.

use axum::{
    body::{Body, Bytes},
    extract::DefaultBodyLimit,
    http::Request,
};
use futures::{StreamExt, stream};
use server_fn::{ServerFnError, error::FromServerFnError, request::Req};
use std::{
    convert::Infallible,
    fmt::Debug,
    sync::{Arc, Mutex},
};
use tracing::{
    Event, Level, Metadata, Subscriber,
    field::{Field, Visit},
    span::{Attributes, Id, Record},
};

/// Records the `path` field of every body limit WARN event.
struct WarnPaths(Arc<Mutex<Vec<String>>>);

impl Subscriber for WarnPaths {
    fn enabled(&self, _: &Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, _: &Attributes<'_>) -> Id {
        Id::from_u64(1)
    }

    fn record(&self, _: &Id, _: &Record<'_>) {}

    fn record_follows_from(&self, _: &Id, _: &Id) {}

    fn event(&self, event: &Event<'_>) {
        let metadata = event.metadata();
        if *metadata.level() == Level::WARN
            && metadata.target() == "server_fn::body_limit"
        {
            event.record(&mut PathField(&self.0));
        }
    }

    fn enter(&self, _: &Id) {}

    fn exit(&self, _: &Id) {}
}

struct PathField<'a>(&'a Mutex<Vec<String>>);

impl Visit for PathField<'_> {
    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        if field.name() == "path" {
            self.0.lock().unwrap().push(format!("{value:?}"));
        }
    }
}

fn limited_request(path: &str, body: Body) -> Request<Body> {
    let mut req = Request::builder().uri(path).body(body).unwrap();
    DefaultBodyLimit::max(1024).apply(&mut req);
    req
}

#[tokio::test]
async fn body_limit_error_logs_one_warning_per_request_body() {
    let paths = Arc::new(Mutex::new(Vec::new()));
    let _guard = tracing::subscriber::set_default(WarnPaths(paths.clone()));

    let chunks =
        (0..100).map(|_| Ok::<_, Infallible>(Bytes::from(vec![b'x'; 1000])));
    let req =
        limited_request("/api/stream", Body::from_stream(stream::iter(chunks)));
    let errors: Vec<ServerFnError> =
        <Request<Body> as Req<ServerFnError>>::try_into_stream(req)
            .expect("request should become a stream")
            .filter_map(|chunk| async move { chunk.err() })
            .map(ServerFnError::de)
            .collect()
            .await;
    assert!(
        matches!(
            errors.as_slice(),
            [ServerFnError::Deserialization(msg)]
                if msg.contains("DefaultBodyLimit")
        ),
        "expected one limit error that names the limit: {errors:?}"
    );

    let req = limited_request("/api/bytes", Body::from(vec![b'x'; 100_000]));
    let result =
        <Request<Body> as Req<ServerFnError>>::try_into_bytes(req).await;
    assert!(result.is_err());

    assert_eq!(*paths.lock().unwrap(), ["/api/stream", "/api/bytes"]);
}
