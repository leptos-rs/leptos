use axum::body::Body;
use http::Request;
use pin_project_lite::pin_project;
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
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
    type Future = LoggingServiceFuture<T::Future>;

    fn poll_ready(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        println!("1. Running my middleware!");

        LoggingServiceFuture {
            inner: self.inner.call(req),
        }
    }
}

pin_project! {
    pub struct LoggingServiceFuture<T> {
        #[pin]
        inner: T,
    }
}

impl<T> Future for LoggingServiceFuture<T>
where
    T: Future,
{
    type Output = T::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        match this.inner.poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(output) => {
                println!("3. Running my middleware!");
                Poll::Ready(output)
            }
        }
    }
}
