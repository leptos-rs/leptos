use futures::{stream::once, Stream, StreamExt};
use hydration_context::SsrSharedContext;
use leptos::{
    nonce::use_nonce,
    reactive_graph::{
        computed::ScopedFuture,
        owner::{Owner, Sandboxed},
    },
    IntoView,
};
use leptos_meta::ServerMetaContext;
use std::{future::Future, pin::Pin, sync::Arc};

pub type PinnedStream<T> = Pin<Box<dyn Stream<Item = T> + Send>>;
pub type PinnedFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;
pub type BoxedFnOnce<T> = Box<dyn FnOnce() -> T + Send>;

pub trait ExtendResponse: Sized {
    type ResponseOptions: Send;

    fn from_stream(stream: impl Stream<Item = String> + Send + 'static)
        -> Self;

    fn extend_response(&mut self, opt: &Self::ResponseOptions);

    fn set_default_content_type(&mut self, content_type: &str);

    fn from_app<IV>(
        app_fn: impl Fn() -> IV + Send + 'static,
        meta_context: ServerMetaContext,
        additional_context: impl FnOnce() + Send + 'static,
        res_options: Self::ResponseOptions,
        stream_builder: fn(
            IV,
            BoxedFnOnce<PinnedStream<String>>,
        ) -> PinnedFuture<PinnedStream<String>>,
    ) -> impl Future<Output = Self> + Send
    where
        IV: IntoView + 'static,
    {
        async move {
            let (owner, stream) = build_response(
                app_fn,
                meta_context,
                additional_context,
                stream_builder,
            );
            let mut stream = stream.await;

            // wait for the first chunk of the stream, then set the status and headers
            let first_chunk = stream.next().await.unwrap_or_default();

            let mut res = Self::from_stream(Sandboxed::new(
                once(async move { first_chunk })
                    .chain(stream)
                    // drop the owner, cleaning up the reactive runtime,
                    // once the stream is over
                    .chain(once(async move {
                        drop(owner);
                        Default::default()
                    })),
            ));

            res.extend_response(&res_options);

            // Set the Content Type headers on all responses. This makes Firefox show the page source
            // without complaining
            res.set_default_content_type("text/html; charset=utf-8");

            res
        }
    }
}

pub fn build_response<IV>(
    app_fn: impl Fn() -> IV + Send + 'static,
    meta_context: ServerMetaContext,
    additional_context: impl FnOnce() + Send + 'static,
    stream_builder: fn(
        IV,
        BoxedFnOnce<PinnedStream<String>>,
    ) -> PinnedFuture<PinnedStream<String>>,
) -> (Owner, PinnedFuture<PinnedStream<String>>)
where
    IV: IntoView + 'static,
{
    let owner = Owner::new_root(Some(Arc::new(SsrSharedContext::new())));
    let stream = Box::pin(Sandboxed::new({
        let owner = owner.clone();
        async move {
            let stream = owner
                .with(|| {
                    additional_context();

                    // run app
                    let app = app_fn();

                    let nonce = use_nonce()
                        .as_ref()
                        .map(|nonce| format!(" nonce=\"{nonce}\""))
                        .unwrap_or_default();

                    let shared_context =
                        Owner::current_shared_context().unwrap();
                    let chunks = Box::new(move || {
                        Box::pin(shared_context.pending_data().unwrap().map(
                            move |chunk| {
                                format!("<script{nonce}>{chunk}</script>")
                            },
                        ))
                            as Pin<Box<dyn Stream<Item = String> + Send>>
                    });

                    // convert app to appropriate response type
                    // and chain the app stream, followed by chunks
                    // in theory, we could select here, and intersperse them
                    // the problem is that during the DOM walk, that would be mean random <script> tags
                    // interspersed where we expect other children
                    //
                    // we also don't actually start hydrating until after the whole stream is complete,
                    // so it's not useful to send those scripts down earlier.
                    stream_builder(app, chunks)
                })
                .await;
            Box::pin(meta_context.inject_meta_context(stream).await)
                as PinnedStream<String>
        }
    }));
    (owner, stream)
}
