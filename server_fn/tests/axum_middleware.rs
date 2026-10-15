#![cfg(all(feature = "axum", feature = "reqwest"))]

use axum::{
    body::Body,
    http::{Request, Response, StatusCode},
};
use serde::{Deserialize, Serialize};
use server_fn::{
    Http, ServerFn, ServerFnError, client::reqwest::ReqwestClient, codec::Json,
    middleware::Layer,
};
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

static MIDDLEWARE_CALLS: AtomicUsize = AtomicUsize::new(0);
static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

#[derive(Serialize, Deserialize)]
struct Limited;

impl ServerFn for Limited {
    const PATH: &'static str = "/api/axum-middleware-once";
    type Client = ReqwestClient;
    type Server = server_fn::axum::AxumServerFnBackend;
    type Protocol = Http<Json, Json>;
    type Output = ();
    type Error = ServerFnError;
    type InputStreamError = ServerFnError;
    type OutputStreamError = ServerFnError;

    fn middlewares() -> Vec<Arc<dyn Layer<Request<Body>, Response<Body>>>> {
        MIDDLEWARE_CALLS.fetch_add(1, Ordering::SeqCst);
        vec![Arc::new(tower::limit::ConcurrencyLimitLayer::new(1))]
    }

    async fn run_body(self) -> Result<Self::Output, Self::Error> {
        let live = LIVE.fetch_add(1, Ordering::SeqCst) + 1;
        PEAK.fetch_max(live, Ordering::SeqCst);
        tokio::time::sleep(Duration::from_millis(100)).await;
        LIVE.fetch_sub(1, Ordering::SeqCst);
        Ok(())
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn middleware_limit_is_shared_across_requests() {
    server_fn::axum::register_explicit::<Limited>();
    let requests = (0..5).map(|_| async {
        let req = Request::post(Limited::PATH)
            .header("content-type", "application/json")
            .body(Body::from("null"))
            .unwrap();
        server_fn::axum::handle_server_fn(req).await
    });
    let responses = futures::future::join_all(requests).await;
    for response in responses {
        assert_eq!(response.status(), StatusCode::OK);
    }
    assert_eq!(
        (
            PEAK.load(Ordering::SeqCst),
            MIDDLEWARE_CALLS.load(Ordering::SeqCst)
        ),
        (1, 1),
        "(peak concurrency, middleware calls)"
    );
}

static INVENTORIED_MIDDLEWARE_CALLS: AtomicUsize = AtomicUsize::new(0);
static REREGISTERED_MIDDLEWARE_CALLS: AtomicUsize = AtomicUsize::new(0);

#[derive(Serialize, Deserialize)]
struct Inventoried;

impl ServerFn for Inventoried {
    const PATH: &'static str = "/api/axum-inventoried-middleware";
    type Client = ReqwestClient;
    type Server = server_fn::axum::AxumServerFnBackend;
    type Protocol = Http<Json, Json>;
    type Output = ();
    type Error = ServerFnError;
    type InputStreamError = ServerFnError;
    type OutputStreamError = ServerFnError;

    fn middlewares() -> Vec<Arc<dyn Layer<Request<Body>, Response<Body>>>> {
        INVENTORIED_MIDDLEWARE_CALLS.fetch_add(1, Ordering::SeqCst);
        Vec::new()
    }

    async fn run_body(self) -> Result<Self::Output, Self::Error> {
        Ok(())
    }
}

server_fn::inventory::submit! {{
    server_fn::ServerFnTraitObj::new::<Inventoried>(|req| {
        Box::pin(<Inventoried as server_fn::ServerFn>::run_on_server(req))
    })
}}

#[tokio::test]
async fn inventoried_middleware_is_built_once() {
    for _ in 0..2 {
        let req = Request::post(Inventoried::PATH)
            .header("content-type", "application/json")
            .body(Body::from("null"))
            .unwrap();
        let response = server_fn::axum::handle_server_fn(req).await;
        assert_eq!(response.status(), StatusCode::OK);
    }
    assert_eq!(INVENTORIED_MIDDLEWARE_CALLS.load(Ordering::SeqCst), 1);
}

#[derive(Serialize, Deserialize)]
struct Reregistered;

impl ServerFn for Reregistered {
    const PATH: &'static str = "/api/axum-reregistered-middleware";
    type Client = ReqwestClient;
    type Server = server_fn::axum::AxumServerFnBackend;
    type Protocol = Http<Json, Json>;
    type Output = ();
    type Error = ServerFnError;
    type InputStreamError = ServerFnError;
    type OutputStreamError = ServerFnError;

    fn middlewares() -> Vec<Arc<dyn Layer<Request<Body>, Response<Body>>>> {
        REREGISTERED_MIDDLEWARE_CALLS.fetch_add(1, Ordering::SeqCst);
        Vec::new()
    }

    async fn run_body(self) -> Result<Self::Output, Self::Error> {
        Ok(())
    }
}

#[tokio::test]
async fn reregistering_rebuilds_middleware() {
    for _ in 0..2 {
        server_fn::axum::register_explicit::<Reregistered>();
        let req = Request::post(Reregistered::PATH)
            .header("content-type", "application/json")
            .body(Body::from("null"))
            .unwrap();
        let response = server_fn::axum::handle_server_fn(req).await;
        assert_eq!(response.status(), StatusCode::OK);
    }
    assert_eq!(REREGISTERED_MIDDLEWARE_CALLS.load(Ordering::SeqCst), 2);
}
