use axum::{
    body::{Body, Bytes, HttpBody},
    http::{Request, Response, StatusCode, header},
    response::IntoResponse,
};
use futures::Future;
use rust_embed::RustEmbed;
use std::{
    convert::Infallible,
    pin::Pin,
    task::{Context, Poll},
};
use tower::Service;
use tower_http::services::fs::DefaultServeDirFallback;

/// Service for serving error pages generated with the provided application shell.
#[derive(Clone, Debug)]
pub struct EmbeddedSiteRoot<SR, F = DefaultServeDirFallback> {
    fallback: Option<F>,
    site_root: SR,
}

impl<SR> EmbeddedSiteRoot<SR> {
    pub fn new(site_root: SR) -> Self {
        EmbeddedSiteRoot {
            fallback: None,
            site_root,
        }
    }
}

impl<SR, F> EmbeddedSiteRoot<SR, F> {
    pub fn fallback<F2>(self, fallback: F2) -> EmbeddedSiteRoot<SR, F2> {
        EmbeddedSiteRoot {
            fallback: Some(fallback),
            site_root: self.site_root,
        }
    }
}

impl<ReqBody, SR, F, ResBody> Service<Request<ReqBody>>
    for EmbeddedSiteRoot<SR, F>
where
    F: Service<
            Request<ReqBody>,
            Response = Response<ResBody>,
            Error = Infallible,
        > + Clone
        + Send
        + 'static,
    F::Future: Send + 'static,
    SR: RustEmbed,
    ReqBody: Send + 'static,
    ResBody: HttpBody<Data = Bytes> + Send + 'static,
    <ResBody as HttpBody>::Error: Into<
        Box<
            dyn std::error::Error
                + std::marker::Send
                + std::marker::Sync
                + 'static,
        >,
    >,
{
    type Response = Response<Body>;
    type Error = Infallible;
    type Future = Pin<
        Box<
            dyn Future<Output = Result<Response<Body>, Infallible>>
                + Send
                + 'static,
        >,
    >;

    fn poll_ready(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        if let Some(fallback) = &mut self.fallback {
            fallback.poll_ready(cx)
        } else {
            Poll::Ready(Ok(()))
        }
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        match SR::get(req.uri().path()) {
            Some(embedded) => Box::pin(async move {
                let mime = mime_guess::from_path(req.uri().path())
                    .first_or_octet_stream();
                Ok(([(header::CONTENT_TYPE, mime.as_ref())], embedded.data)
                    .into_response())
            }),
            None => {
                if let Some(mut fallback) = self.fallback.clone() {
                    Box::pin(async move {
                        fallback.call(req).await.map(|b| b.into_response())
                    })
                } else {
                    Box::pin(async move {
                        Ok((StatusCode::NOT_FOUND, "404 Not Found")
                            .into_response())
                    })
                }
            }
        }
    }
}
