use axum::http::{Request, Response};
use futures::Future;
use rust_embed::Embed;
use std::{
    convert::Infallible,
    pin::Pin,
    task::{Context, Poll},
};
use tower::Service;
use tower_http::services::fs::DefaultServeDirFallback;

#[derive(Embed)]
#[folder = "$LEPTOS_SITE_ROOT"]
#[allow_missing = true]
struct SiteRoot;

/// Service for serving error pages generated with the provided application shell.
#[derive(Clone, Debug)]
pub struct EmbededSiteRoot<F = DefaultServeDirFallback> {
    fallback: Option<F>,
}

impl EmbededSiteRoot {
    pub fn new() -> Self {
        EmbededSiteRoot { fallback: None }
    }
}

impl<F> EmbededSiteRoot<F> {
    pub fn fallback<F2>(self, fallback: F2) -> EmbededSiteRoot<F2> {
        EmbededSiteRoot {
            fallback: Some(fallback),
        }
    }
}

impl<ReqBody, F, ResBody> Service<Request<ReqBody>> for EmbededSiteRoot<F>
where
    F: Service<
            Request<ReqBody>,
            Response = Response<ResBody>,
            Error = Infallible,
        > + Clone,
    F::Future: Send + 'static,
{
    type Response = Response<ReqBody>;
    type Error = Infallible;
    type Future = Pin<
        Box<
            dyn Future<Output = Result<Response<ReqBody>, Infallible>>
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

    fn call(&mut self, _req: Request<ReqBody>) -> Self::Future {
        todo!()
    }
}
