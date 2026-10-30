#![cfg(all(feature = "actix", feature = "reqwest"))]

use actix_web::{
    FromRequest,
    http::{
        StatusCode,
        header::{HeaderName, HeaderValue},
    },
    test::TestRequest,
    web::Payload,
};
use serde::{Deserialize, Serialize};
use server_fn::{
    Http, ServerFn, ServerFnError,
    client::reqwest::ReqwestClient,
    codec::Json,
    middleware::{BoxedService, Layer, ServerFnErrorSerializer, Service},
    request::actix::ActixRequest,
    response::actix::ActixResponse,
};
use std::{
    future::Future,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

static MIDDLEWARE_CALLS: AtomicUsize = AtomicUsize::new(0);

struct CountLayer;
#[derive(Clone)]
struct CountService {
    inner: BoxedService<ActixRequest, ActixResponse>,
    seen: Arc<AtomicUsize>,
}

impl Layer<ActixRequest, ActixResponse> for CountLayer {
    fn layer(
        &self,
        inner: BoxedService<ActixRequest, ActixResponse>,
    ) -> BoxedService<ActixRequest, ActixResponse> {
        BoxedService::new(
            inner.err_ser,
            CountService {
                inner,
                seen: Arc::new(AtomicUsize::new(0)),
            },
        )
    }
}

impl Service<ActixRequest, ActixResponse> for CountService {
    fn run(
        &mut self,
        req: ActixRequest,
        _err_ser: ServerFnErrorSerializer,
    ) -> Pin<Box<dyn Future<Output = ActixResponse> + Send>> {
        let seen = self.seen.fetch_add(1, Ordering::SeqCst) + 1;
        let response = self.inner.run(req);
        Box::pin(async move {
            let mut response = response.await.take();
            response.headers_mut().insert(
                HeaderName::from_static("x-seen"),
                HeaderValue::from_str(&seen.to_string()).unwrap(),
            );
            ActixResponse::from(response)
        })
    }
}

#[derive(Serialize, Deserialize)]
struct Counted;

impl ServerFn for Counted {
    const PATH: &'static str = "/api/actix-middleware-once";
    type Client = ReqwestClient;
    type Server = server_fn::actix::ActixServerFnBackend;
    type Protocol = Http<Json, Json>;
    type Output = ();
    type Error = ServerFnError;
    type InputStreamError = ServerFnError;
    type OutputStreamError = ServerFnError;

    fn middlewares() -> Vec<Arc<dyn Layer<ActixRequest, ActixResponse>>> {
        MIDDLEWARE_CALLS.fetch_add(1, Ordering::SeqCst);
        vec![Arc::new(CountLayer)]
    }

    async fn run_body(self) -> Result<Self::Output, Self::Error> {
        Ok(())
    }
}

#[actix_web::test]
async fn middleware_state_is_shared_across_requests() {
    server_fn::actix::register_explicit::<Counted>();
    let mut seen = Vec::new();
    for _ in 0..3 {
        let (req, mut payload) = TestRequest::post()
            .uri(Counted::PATH)
            .insert_header(("content-type", "application/json"))
            .set_payload("null")
            .to_http_parts();
        let payload = Payload::from_request(&req, &mut payload).await.unwrap();
        let response = server_fn::actix::handle_server_fn(req, payload).await;
        assert_eq!(response.status(), StatusCode::OK);
        seen.push(
            response
                .headers()
                .get("x-seen")
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned(),
        );
    }
    assert_eq!(
        (seen, MIDDLEWARE_CALLS.load(Ordering::SeqCst)),
        (vec!["1".to_owned(), "2".to_owned(), "3".to_owned()], 1),
        "(shared request counts, middleware calls)"
    );
}
