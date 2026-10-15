use crate::error::{ServerFnErrorErr, ServerFnErrorResponseParts};
use std::{future::Future, pin::Pin};

/// An abstraction over a middleware layer, which can be used to add additional
/// middleware layer to a [`Service`].
pub trait Layer<Req, Res>: Send + Sync + 'static {
    /// Adds this layer to the inner service.
    fn layer(&self, inner: BoxedService<Req, Res>) -> BoxedService<Req, Res>;
}

/// A type-erased service, which takes an HTTP request and returns a response.
///
/// Like `tower::util::BoxCloneSyncService`, cloning a `BoxedService` clones
/// the service inside it. The server function integrations apply the
/// middleware layers once and run each request on its own clone, so state
/// that the clones share (for example behind an `Arc`) applies across
/// requests, while plain fields of a service are per clone.
#[non_exhaustive]
pub struct BoxedService<Req, Res> {
    /// A function that converts a [`ServerFnErrorErr`] into [`ServerFnErrorResponseParts`].
    pub err_ser: ServerFnErrorSerializer,
    /// The inner service, which is cloned when the `BoxedService` is cloned.
    service: Box<dyn CloneService<Req, Res>>,
}

impl<Req, Res> Clone for BoxedService<Req, Res> {
    fn clone(&self) -> Self {
        Self {
            err_ser: self.err_ser,
            service: self.service.clone_box(),
        }
    }
}

impl<Req, Res> BoxedService<Req, Res> {
    /// Constructs a type-erased service from this service.
    pub fn new(
        ser: ServerFnErrorSerializer,
        service: impl Service<Req, Res> + Clone + Send + Sync + 'static,
    ) -> Self {
        Self {
            err_ser: ser,
            service: Box::new(service),
        }
    }

    /// Converts a request into a response by running the inner service.
    pub fn run(
        &mut self,
        req: Req,
    ) -> Pin<Box<dyn Future<Output = Res> + Send>> {
        self.service.run(req, self.err_ser)
    }
}

/// A [`Service`] that can be cloned behind a trait object, which lets
/// [`BoxedService`] be cloned.
trait CloneService<Req, Res>: Service<Req, Res> + Send + Sync {
    /// Clones this service into a new boxed trait object.
    fn clone_box(&self) -> Box<dyn CloneService<Req, Res>>;
}

impl<S, Req, Res> CloneService<Req, Res> for S
where
    S: Service<Req, Res> + Clone + Send + Sync + 'static,
{
    fn clone_box(&self) -> Box<dyn CloneService<Req, Res>> {
        Box::new(self.clone())
    }
}

/// Type alias for a function that serializes a [`ServerFnErrorErr`] into
/// [`ServerFnErrorResponseParts`].
pub type ServerFnErrorSerializer =
    fn(ServerFnErrorErr) -> ServerFnErrorResponseParts;

/// A service converts an HTTP request into a response.
pub trait Service<Request, Response> {
    /// Converts a request into a response.
    fn run(
        &mut self,
        req: Request,
        err_ser: ServerFnErrorSerializer,
    ) -> Pin<Box<dyn Future<Output = Response> + Send>>;
}

#[cfg(feature = "axum-no-default")]
mod axum {
    use super::{BoxedService, ServerFnErrorSerializer, Service};
    use crate::{ServerFnError, error::ServerFnErrorErr, response::Res};
    use axum::body::Body;
    use http::{Request, Response};
    use std::{future::Future, pin::Pin};

    impl<S> super::Service<Request<Body>, Response<Body>> for S
    where
        S: tower::Service<Request<Body>, Response = Response<Body>>
            + Clone
            + Send
            + 'static,
        S::Future: Send + 'static,
        S::Error: std::fmt::Display + Send + 'static,
    {
        fn run(
            &mut self,
            req: Request<Body>,
            err_ser: ServerFnErrorSerializer,
        ) -> Pin<Box<dyn Future<Output = Response<Body>> + Send>> {
            let path = req.uri().path().to_string();
            // Clone-and-replace moves the called instance into the `'static`
            // future so it can be polled ready first.
            let clone = self.clone();
            let mut svc = std::mem::replace(self, clone);
            Box::pin(async move {
                let res =
                    match futures::future::poll_fn(|cx| svc.poll_ready(cx))
                        .await
                    {
                        Ok(()) => svc.call(req).await,
                        Err(e) => Err(e),
                    };
                res.unwrap_or_else(|e| {
                    let err = err_ser(ServerFnErrorErr::MiddlewareError(
                        e.to_string(),
                    ));
                    Response::<Body>::error_response(&path, err)
                })
            })
        }
    }

    impl tower::Service<Request<Body>>
        for BoxedService<Request<Body>, Response<Body>>
    {
        type Response = Response<Body>;
        type Error = ServerFnError;
        type Future = Pin<
            Box<
                dyn std::future::Future<
                        Output = Result<Self::Response, Self::Error>,
                    > + Send,
            >,
        >;

        fn poll_ready(
            &mut self,
            _cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Result<(), Self::Error>> {
            Ok(()).into()
        }

        fn call(&mut self, req: Request<Body>) -> Self::Future {
            let inner = self.service.run(req, self.err_ser);
            Box::pin(async move { Ok(inner.await) })
        }
    }

    impl<L> super::Layer<Request<Body>, Response<Body>> for L
    where
        L: tower_layer::Layer<BoxedService<Request<Body>, Response<Body>>>
            + Sync
            + Send
            + 'static,
        L::Service: Service<Request<Body>, Response<Body>>
            + Clone
            + Send
            + Sync
            + 'static,
    {
        fn layer(
            &self,
            inner: BoxedService<Request<Body>, Response<Body>>,
        ) -> BoxedService<Request<Body>, Response<Body>> {
            BoxedService::new(inner.err_ser, self.layer(inner))
        }
    }
}

#[cfg(feature = "actix-no-default")]
mod actix {
    use crate::{
        error::ServerFnErrorErr,
        middleware::ServerFnErrorSerializer,
        request::actix::ActixRequest,
        response::{Res, actix::ActixResponse},
    };
    use actix_web::{HttpRequest, HttpResponse};
    use std::{future::Future, pin::Pin};

    impl<S> super::Service<HttpRequest, HttpResponse> for S
    where
        S: actix_web::dev::Service<HttpRequest, Response = HttpResponse>,
        S::Future: Send + 'static,
        S::Error: std::fmt::Display + Send + 'static,
    {
        fn run(
            &mut self,
            req: HttpRequest,
            err_ser: ServerFnErrorSerializer,
        ) -> Pin<Box<dyn Future<Output = HttpResponse> + Send>> {
            let path = req.uri().path().to_string();
            let inner = self.call(req);
            Box::pin(async move {
                inner.await.unwrap_or_else(|e| {
                    let err = err_ser(ServerFnErrorErr::MiddlewareError(
                        e.to_string(),
                    ));
                    ActixResponse::error_response(&path, err).take()
                })
            })
        }
    }

    impl<S> super::Service<ActixRequest, ActixResponse> for S
    where
        S: actix_web::dev::Service<HttpRequest, Response = HttpResponse>,
        S::Future: Send + 'static,
        S::Error: std::fmt::Display + Send + 'static,
    {
        fn run(
            &mut self,
            req: ActixRequest,
            err_ser: ServerFnErrorSerializer,
        ) -> Pin<Box<dyn Future<Output = ActixResponse> + Send>> {
            let path = req.0.0.uri().path().to_string();
            let inner = self.call(req.0.take().0);
            Box::pin(async move {
                ActixResponse::from(inner.await.unwrap_or_else(|e| {
                    let err = err_ser(ServerFnErrorErr::MiddlewareError(
                        e.to_string(),
                    ));
                    ActixResponse::error_response(&path, err).take()
                }))
            })
        }
    }
}

#[cfg(all(test, feature = "axum"))]
mod tests {
    use super::{BoxedService, ServerFnErrorSerializer};
    use crate::error::{ServerFnErrorErr, ServerFnErrorResponseParts};
    use axum::body::Body;
    use bytes::Bytes;
    use http::{Request, Response, StatusCode};
    use std::{
        future::Future,
        pin::Pin,
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
        task::{Context, Poll},
    };

    #[derive(Clone)]
    struct Inner;

    impl super::Service<Request<Body>, Response<Body>> for Inner {
        fn run(
            &mut self,
            _req: Request<Body>,
            _err_ser: ServerFnErrorSerializer,
        ) -> Pin<Box<dyn Future<Output = Response<Body>> + Send>> {
            Box::pin(async { Response::new(Body::from("ok")) })
        }
    }

    fn ser(_: ServerFnErrorErr) -> ServerFnErrorResponseParts {
        ServerFnErrorResponseParts::builder()
            .body(Bytes::new())
            .content_type("text/plain")
            .status_code(StatusCode::INTERNAL_SERVER_ERROR)
            .build()
    }

    #[tokio::test]
    async fn concurrency_limit_layer_is_polled_ready_before_call() {
        let layer = tower::limit::ConcurrencyLimitLayer::new(1);
        let mut svc = <tower::limit::ConcurrencyLimitLayer as super::Layer<
            Request<Body>,
            Response<Body>,
        >>::layer(&layer, BoxedService::new(ser, Inner));

        let response = svc.run(Request::new(Body::empty())).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = svc.run(Request::new(Body::empty())).await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[derive(Clone)]
    struct NotReady;

    impl tower::Service<Request<Body>> for NotReady {
        type Response = Response<Body>;
        type Error = &'static str;
        type Future =
            futures::future::Ready<Result<Self::Response, Self::Error>>;

        fn poll_ready(
            &mut self,
            _cx: &mut Context<'_>,
        ) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Err("not ready"))
        }

        fn call(&mut self, _req: Request<Body>) -> Self::Future {
            futures::future::ready(Ok(Response::new(Body::from("ok"))))
        }
    }

    #[tokio::test]
    async fn readiness_error_becomes_middleware_error_response() {
        let mut svc = BoxedService::new(ser, NotReady);

        let response = svc.run(Request::new(Body::empty())).await;

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[derive(Clone)]
    struct Counter {
        per_clone: usize,
        shared: Arc<AtomicUsize>,
    }

    impl super::Service<Request<Body>, Response<Body>> for Counter {
        fn run(
            &mut self,
            _req: Request<Body>,
            _err_ser: ServerFnErrorSerializer,
        ) -> Pin<Box<dyn Future<Output = Response<Body>> + Send>> {
            self.per_clone += 1;
            self.shared.fetch_add(1, Ordering::SeqCst);
            let body = Body::from(self.per_clone.to_string());
            Box::pin(async move { Response::new(body) })
        }
    }

    async fn count(
        svc: &mut BoxedService<Request<Body>, Response<Body>>,
    ) -> String {
        let body = svc.run(Request::new(Body::empty())).await.into_body();
        let bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    #[tokio::test]
    async fn clone_copies_plain_state_and_shares_arc_state() {
        let shared = Arc::new(AtomicUsize::new(0));
        let mut first = BoxedService::new(
            ser,
            Counter {
                per_clone: 0,
                shared: Arc::clone(&shared),
            },
        );
        assert_eq!(count(&mut first).await, "1");

        let mut second = first.clone();
        assert_eq!(count(&mut first).await, "2");
        assert_eq!(count(&mut first).await, "3");
        assert_eq!(count(&mut second).await, "2");
        assert_eq!(shared.load(Ordering::SeqCst), 4);
    }
}
