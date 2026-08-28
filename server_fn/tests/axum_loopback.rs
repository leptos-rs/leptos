#![cfg(all(feature = "axum", feature = "reqwest"))]

//! Shared Axum loopback harness for native client integration tests. The
//! process-wide server and client URL are initialized once so later HTTP and
//! websocket regressions can share this file under Cargo test and nextest.

use axum::{
    Router,
    http::{HeaderName, StatusCode, header},
    routing::any,
};
use serde::{Deserialize, Serialize};
use server_fn::{
    Http, Protocol, ServerFnError,
    client::{reqwest::ReqwestClient, try_set_server_url},
    codec::{Json, PostUrl},
    mock::BrowserMockServer,
    redirect::{REDIRECT_HEADER, set_redirect_hook},
};
use std::{
    net::SocketAddr,
    sync::{Mutex, OnceLock},
    time::Duration,
};
use tokio::time::timeout;

static SERVER: OnceLock<SocketAddr> = OnceLock::new();
static HOOK: OnceLock<()> = OnceLock::new();
static LOCATIONS: Mutex<Vec<String>> = Mutex::new(Vec::new());

fn server_addr() -> SocketAddr {
    *SERVER.get_or_init(|| {
        let listener = std::net::TcpListener::bind("127.0.0.1:0")
            .expect("loopback listener should bind");
        listener
            .set_nonblocking(true)
            .expect("loopback listener should be nonblocking");
        let addr = listener
            .local_addr()
            .expect("loopback listener should have an address");

        std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("loopback runtime should build");
            runtime.block_on(async move {
                let listener = tokio::net::TcpListener::from_std(listener)
                    .expect("std listener should convert to Tokio");
                let app = Router::new()
                    .route(
                        "/api/{*rest}",
                        any(server_fn::axum::handle_server_fn),
                    )
                    .route("/needs_login", any(needs_login))
                    .route("/after", any(after));
                axum::serve(listener, app)
                    .await
                    .expect("loopback server should run");
            });
        });

        let server_url = Box::leak(format!("http://{addr}").into_boxed_str());
        let _ = try_set_server_url(server_url);
        addr
    })
}

async fn needs_login()
-> (StatusCode, [(HeaderName, &'static str); 2], &'static str) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        [
            (header::LOCATION, "/login"),
            (HeaderName::from_static(REDIRECT_HEADER), ""),
        ],
        "\"not logged in\"",
    )
}

async fn after() -> (StatusCode, [(HeaderName, &'static str); 2], &'static str)
{
    (
        StatusCode::OK,
        [
            (header::LOCATION, "/after"),
            (HeaderName::from_static(REDIRECT_HEADER), ""),
        ],
        "\"ok\"",
    )
}

fn install_redirect_hook() {
    HOOK.get_or_init(|| {
        let result = set_redirect_hook(|location| {
            LOCATIONS.lock().unwrap().push(location.to_string());
        });
        assert!(result.is_ok(), "redirect hook should install once");
    });
}

#[derive(Serialize, Deserialize)]
struct In {
    a: u32,
}

#[tokio::test(flavor = "multi_thread")]
async fn redirect_hook_runs_for_error_and_success_responses() {
    let _ = server_addr();
    install_redirect_hook();
    LOCATIONS.lock().unwrap().clear();

    let error = timeout(
        Duration::from_secs(5),
        <Http<PostUrl, Json> as Protocol<
            In,
            String,
            ReqwestClient,
            BrowserMockServer,
            ServerFnError,
        >>::run_client("/needs_login", In { a: 1 }),
    )
    .await
    .expect("error response should arrive before timeout");
    assert!(error.is_err(), "500 response should decode as an error");
    assert!(
        LOCATIONS
            .lock()
            .unwrap()
            .iter()
            .any(|location| location == "/login"),
        "redirect hook should record /login for the 500 response"
    );

    let output = timeout(
        Duration::from_secs(5),
        <Http<PostUrl, Json> as Protocol<
            In,
            String,
            ReqwestClient,
            BrowserMockServer,
            ServerFnError,
        >>::run_client("/after", In { a: 1 }),
    )
    .await
    .expect("success response should arrive before timeout")
    .expect("200 response should decode successfully");
    assert_eq!(output, "ok");
    assert!(
        LOCATIONS
            .lock()
            .unwrap()
            .iter()
            .any(|location| location == "/after"),
        "redirect hook should record /after for the 200 response"
    );
}
