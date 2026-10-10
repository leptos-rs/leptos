#![cfg(all(feature = "axum", feature = "reqwest"))]

//! Shared Axum loopback harness for native client integration tests. The
//! process-wide server and client URL are initialized once so later HTTP and
//! websocket regressions can share this file under Cargo test and nextest.

use axum::{
    Router,
    http::{HeaderName, StatusCode, header},
    routing::any,
};
use futures::{StreamExt, stream};
use serde::{Deserialize, Serialize};
use server_fn::{
    BoxedStream, Http, Protocol, ServerFn, ServerFnError, Websocket,
    client::{reqwest::ReqwestClient, try_set_server_url},
    codec::{Json, JsonEncoding, PostUrl},
    mock::BrowserMockServer,
    redirect::{REDIRECT_HEADER, set_redirect_hook},
};
use std::{
    future::Future,
    net::SocketAddr,
    ops::Deref,
    sync::{
        Mutex, OnceLock,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio::time::timeout;

static SERVER: OnceLock<SocketAddr> = OnceLock::new();
static HOOK: OnceLock<()> = OnceLock::new();
static LOCATIONS: Mutex<Vec<String>> = Mutex::new(Vec::new());
static LIVE: AtomicUsize = AtomicUsize::new(0);
static PULLED: AtomicUsize = AtomicUsize::new(0);
static WEBSOCKET_TEST: tokio::sync::Mutex<()> =
    tokio::sync::Mutex::const_new(());

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct Msg {
    v: u32,
}

struct LiveGuard;

impl Drop for LiveGuard {
    fn drop(&mut self) {
        LIVE.fetch_sub(1, Ordering::SeqCst);
    }
}

async fn idle(
    input: BoxedStream<Msg, ServerFnError>,
) -> Result<BoxedStream<Msg, ServerFnError>, ServerFnError> {
    LIVE.fetch_add(1, Ordering::SeqCst);
    let guard = LiveGuard;
    Ok(stream::pending::<Result<Msg, ServerFnError>>()
        .map(move |item| {
            let _keep = (&guard, &input);
            item
        })
        .into())
}

async fn sum(
    input: BoxedStream<Msg, ServerFnError>,
) -> Result<BoxedStream<Msg, ServerFnError>, ServerFnError> {
    Ok(stream::once(async move {
        let mut total = 0;
        let mut input = input;
        while let Some(Ok(message)) = input.next().await {
            total += message.v;
        }
        Ok(Msg { v: total })
    })
    .into())
}

struct Idle {
    input: BoxedStream<Msg, ServerFnError>,
}

impl Deref for Idle {
    type Target = BoxedStream<Msg, ServerFnError>;

    fn deref(&self) -> &Self::Target {
        &self.input
    }
}

impl From<Idle> for BoxedStream<Msg, ServerFnError> {
    fn from(value: Idle) -> Self {
        value.input
    }
}

impl From<BoxedStream<Msg, ServerFnError>> for Idle {
    fn from(input: BoxedStream<Msg, ServerFnError>) -> Self {
        Self { input }
    }
}

impl ServerFn for Idle {
    const PATH: &'static str = "/api/idle";

    type Client = ReqwestClient;
    type Server = server_fn::axum::AxumServerFnBackend;
    type Protocol = Websocket<JsonEncoding, JsonEncoding>;
    type Output = BoxedStream<Msg, ServerFnError>;
    type Error = ServerFnError;
    type InputStreamError = ServerFnError;
    type OutputStreamError = ServerFnError;

    fn run_body(
        self,
    ) -> impl Future<Output = Result<Self::Output, Self::Error>> + Send {
        idle(self.input)
    }
}

struct Sum {
    input: BoxedStream<Msg, ServerFnError>,
}

impl Deref for Sum {
    type Target = BoxedStream<Msg, ServerFnError>;

    fn deref(&self) -> &Self::Target {
        &self.input
    }
}

impl From<Sum> for BoxedStream<Msg, ServerFnError> {
    fn from(value: Sum) -> Self {
        value.input
    }
}

impl From<BoxedStream<Msg, ServerFnError>> for Sum {
    fn from(input: BoxedStream<Msg, ServerFnError>) -> Self {
        Self { input }
    }
}

impl ServerFn for Sum {
    const PATH: &'static str = "/api/sum";

    type Client = ReqwestClient;
    type Server = server_fn::axum::AxumServerFnBackend;
    type Protocol = Websocket<JsonEncoding, JsonEncoding>;
    type Output = BoxedStream<Msg, ServerFnError>;
    type Error = ServerFnError;
    type InputStreamError = ServerFnError;
    type OutputStreamError = ServerFnError;

    fn run_body(
        self,
    ) -> impl Future<Output = Result<Self::Output, Self::Error>> + Send {
        sum(self.input)
    }
}

fn server_addr() -> SocketAddr {
    *SERVER.get_or_init(|| {
        server_fn::axum::register_explicit::<Idle>();
        server_fn::axum::register_explicit::<Sum>();
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

#[tokio::test(flavor = "multi_thread")]
async fn server_forwarder_stops_when_client_disconnects() {
    let _guard = WEBSOCKET_TEST.lock().await;
    let before = LIVE.load(Ordering::SeqCst);
    let addr = server_addr();
    let (ws, _) =
        tokio_tungstenite::connect_async(format!("ws://{addr}{}", Idle::PATH))
            .await
            .expect("websocket client should connect");

    timeout(Duration::from_secs(5), async {
        while LIVE.load(Ordering::SeqCst) != before + 1 {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("idle output stream should become live");

    drop(ws);

    timeout(Duration::from_secs(5), async {
        while LIVE.load(Ordering::SeqCst) != before {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("idle output stream should be dropped after disconnect");
}

#[tokio::test(flavor = "multi_thread")]
async fn client_signals_end_of_input_so_the_server_can_reply() {
    let _ = server_addr();
    let input =
        stream::iter([Ok(Msg { v: 1 }), Ok(Msg { v: 2 }), Ok(Msg { v: 3 })]);
    let mut out =
        <Websocket<JsonEncoding, JsonEncoding> as Protocol<
            Sum,
            BoxedStream<Msg, ServerFnError>,
            ReqwestClient,
            server_fn::axum::AxumServerFnBackend,
            ServerFnError,
        >>::run_client(Sum::PATH, Sum::from(BoxedStream::from(input)))
        .await
        .expect("websocket client should connect");

    let result = timeout(Duration::from_secs(5), out.next())
        .await
        .expect("sum response should arrive before timeout");
    assert_eq!(result, Some(Ok(Msg { v: 6 })));
}

#[tokio::test(flavor = "multi_thread")]
async fn dropping_the_output_stream_closes_the_socket_and_stops_the_pump() {
    let _guard = WEBSOCKET_TEST.lock().await;
    let _ = server_addr();
    let before = LIVE.load(Ordering::SeqCst);
    PULLED.store(0, Ordering::SeqCst);
    let input = stream::repeat(Ok(Msg { v: 0 })).inspect(|_| {
        PULLED.fetch_add(1, Ordering::SeqCst);
    });
    let out =
        <Websocket<JsonEncoding, JsonEncoding> as Protocol<
            Idle,
            BoxedStream<Msg, ServerFnError>,
            ReqwestClient,
            server_fn::axum::AxumServerFnBackend,
            ServerFnError,
        >>::run_client(Idle::PATH, Idle::from(BoxedStream::from(input)))
        .await
        .expect("websocket client should connect");

    timeout(Duration::from_secs(5), async {
        while LIVE.load(Ordering::SeqCst) != before + 1 {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("idle output stream should become live");

    drop(out);

    timeout(Duration::from_secs(5), async {
        while LIVE.load(Ordering::SeqCst) != before {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("idle output stream should be dropped after client closes");

    tokio::time::sleep(Duration::from_millis(200)).await;
    let first = PULLED.load(Ordering::SeqCst);
    tokio::time::sleep(Duration::from_millis(200)).await;
    let second = PULLED.load(Ordering::SeqCst);
    assert_eq!(first, second, "input pump should stop polling");
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
