use axum::body::Body;
use http::Request;
use std::task::{Context, Poll};
use tower::{Layer, Service};

pub struct LoggingLayer;

impl<S> Layer<S> for LoggingLayer {
    type Service = LoggingService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        LoggingService { inner }
    }
}

pub struct LoggingService<T> {
    inner: T,
}

impl<T> Service<Request<Body>> for LoggingService<T>
where
    T: Service<Request<Body>>,
{
    type Response = T::Response;
    type Error = T::Error;
    type Future = T::Future;

    fn poll_ready(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        println!("Running my middleware!");

        self.inner.call(req)
    }
}
