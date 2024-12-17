use axum::{
    body::Body,
    http::{Request, Response},
};
use leptos::prelude::expect_context;
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tower::{Layer, Service};

use crate::{
    server_types::{HandlerStructAlias, ServerState},
    traits::SubDomainTrait1,
};
use pin_project_lite::pin_project;

#[derive(Clone)]
pub struct SubDomain1Layer;

impl<S> Layer<S> for SubDomain1Layer {
    type Service = SubDomain1MiddleWare<S>;

    fn layer(&self, inner: S) -> Self::Service {
        SubDomain1MiddleWare { inner }
    }
}

pub struct SubDomain1MiddleWare<S> {
    inner: S,
}

impl<S, ReqBody> Service<Request<ReqBody>> for SubDomain1MiddleWare<S>
where
    S: Service<Request<ReqBody>, Response = Response<Body>>,
    S::Error: std::fmt::Debug,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = SubDomain1Future<S::Future>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        let req_fut = self.inner.call(req);
        SubDomain1Future { req_fut }
    }
}
pin_project! {
    pub struct SubDomain1Future<F> {
        #[pin]
        req_fut: F,
    }
}

impl<F, Err> Future for SubDomain1Future<F>
where
    F: Future<Output = Result<Response<Body>, Err>>,
{
    type Output = Result<Response<Body>, Err>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let subdomain_1 = expect_context::<ServerState<HandlerStructAlias>>()
            .handler
            .sub_domain_1;
        let mut subdomain_1_fut = subdomain_1.sub_domain_1_method();
        match Pin::as_mut(&mut subdomain_1_fut).poll(cx) {
            Poll::Ready(Ok(_)) => {
                println!("Middleware for Subdomain 1 Passed, calling request...");
                this.req_fut.poll(cx)
            }
            Poll::Ready(Err(_)) => Poll::Ready(Ok(Response::builder()
                .status(http::StatusCode::FORBIDDEN)
                .body(Body::from("Access denied"))
                .unwrap())),
            Poll::Pending => Poll::Pending,
        }
    }
}
