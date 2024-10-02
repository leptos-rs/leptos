use futures::{stream::once, Stream, StreamExt};
use hydration_context::{SharedContext, SsrSharedContext};
use leptos::{
    nonce::use_nonce,
    reactive::owner::{Owner, Sandboxed},
    IntoView,
};
use leptos_config::LeptosOptions;
use leptos_meta::ServerMetaContextOutput;
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
        app_fn: impl FnOnce() -> IV + Send + 'static,
        meta_context: ServerMetaContextOutput,
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
            let (owner, stream) =
                build_response(app_fn, additional_context, stream_builder);

            let stream = stream.await.ready_chunks(32).map(|n| n.join(""));

            let sc = owner.shared_context().unwrap();
            while let Some(pending) = sc.await_deferred() {
                pending.await;
            }

            let mut stream =
                Box::pin(meta_context.inject_meta_context(stream).await);

            // wait for the first chunk of the stream, then set the status and headers
            let first_chunk = stream.next().await.unwrap_or_default();

            let mut res = Self::from_stream(Sandboxed::new(
                once(async move { first_chunk })
                    .chain(stream)
                    // drop the owner, cleaning up the reactive runtime,
                    // once the stream is over
                    .chain(once(async move {
                        owner.unset();
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
    app_fn: impl FnOnce() -> IV + Send + 'static,
    additional_context: impl FnOnce() + Send + 'static,
    stream_builder: fn(
        IV,
        BoxedFnOnce<PinnedStream<String>>,
    ) -> PinnedFuture<PinnedStream<String>>,
) -> (Owner, PinnedFuture<PinnedStream<String>>)
where
    IV: IntoView + 'static,
{
    let shared_context = Arc::new(SsrSharedContext::new())
        as Arc<dyn SharedContext + Send + Sync>;
    let owner = Owner::new_root(Some(Arc::clone(&shared_context)));
    let stream = Box::pin(Sandboxed::new({
        let owner = owner.clone();
        async move {
            let stream = owner.with(|| {
                additional_context();

                // run app
                let app = app_fn();

                let nonce = use_nonce()
                    .as_ref()
                    .map(|nonce| format!(" nonce=\"{nonce}\""))
                    .unwrap_or_default();

                let shared_context = Owner::current_shared_context().unwrap();

                let chunks = Box::new({
                    let shared_context = shared_context.clone();
                    move || {
                        Box::pin(shared_context.pending_data().unwrap().map(
                            move |chunk| {
                                format!("<script{nonce}>{chunk}</script>")
                            },
                        ))
                            as Pin<Box<dyn Stream<Item = String> + Send>>
                    }
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
            });

            stream.await
        }
    }));
    (owner, stream)
}

pub fn static_file_path(options: &LeptosOptions, path: &str) -> String {
    let trimmed_path = path.trim_start_matches('/');
    let path = if trimmed_path.is_empty() {
        "index"
    } else {
        trimmed_path
    };
    format!("{}/{}.html", options.site_root, path)
}
