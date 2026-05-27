#![allow(clippy::type_complexity)]

use futures::{Stream, StreamExt, stream::once};
use hydration_context::{SharedContext, SsrSharedContext};
use leptos::{
    IntoView, PrefetchLazyFn, WasmSplitManifest,
    context::provide_context,
    nonce::use_nonce,
    prelude::ReadValue,
    reactive::owner::{Owner, Sandboxed},
};
use leptos_config::LeptosOptions;
use leptos_meta::{Link, ServerMetaContextOutput};
use std::{future::Future, pin::Pin, sync::Arc};

pub type PinnedStream<T> = Pin<Box<dyn Stream<Item = T> + Send>>;
pub type PinnedFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;
pub type BoxedFnOnce<T> = Box<dyn FnOnce() -> T + Send>;

/// Wraps a response body stream and ties cleanup of the reactive [`Owner`] to
/// the stream's `Drop`, rather than to a chained terminal future.
///
/// The previous approach appended an `owner.unset_with_forced_cleanup()` future
/// as the last item of the stream. That only runs if the stream is polled to
/// completion, so a client disconnecting mid-response (slow client, browser
/// cancel, proxy timeout) would drop the body before the terminal future ran,
/// leaking the `Owner` and everything it transitively keeps alive. Cleaning up
/// on `Drop` runs whether the stream finishes or is cancelled.
struct OwnerCleanupStream {
    inner: Pin<Box<dyn Stream<Item = String> + Send>>,
    owner: Option<Owner>,
}

impl OwnerCleanupStream {
    fn new(
        inner: impl Stream<Item = String> + Send + 'static,
        owner: Owner,
    ) -> Self {
        Self {
            inner: Box::pin(inner),
            owner: Some(owner),
        }
    }
}

impl Stream for OwnerCleanupStream {
    type Item = String;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.get_mut().inner.as_mut().poll_next(cx)
    }
}

impl Drop for OwnerCleanupStream {
    fn drop(&mut self) {
        if let Some(owner) = self.owner.take() {
            owner.unset_with_forced_cleanup();
        }
    }
}

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
            bool,
        ) -> PinnedFuture<PinnedStream<String>>,
        supports_ooo: bool,
    ) -> impl Future<Output = Self> + Send
    where
        IV: IntoView + 'static,
    {
        async move {
            let prefetches = PrefetchLazyFn::default();

            let (owner, stream) = build_response(
                app_fn,
                additional_context,
                stream_builder,
                supports_ooo,
            );

            owner.with(|| provide_context(prefetches.clone()));

            let sc = owner.shared_context().unwrap();

            let stream = stream.await.ready_chunks(32).map(|n| n.join(""));

            while let Some(pending) = sc.await_deferred() {
                pending.await;
            }

            if !prefetches.0.read_value().is_empty() {
                use leptos::prelude::*;

                let nonce =
                    use_nonce().map(|n| n.to_string()).unwrap_or_default();
                if let Some(manifest) = use_context::<WasmSplitManifest>() {
                    let (pkg_path, manifest, wasm_split_file) =
                        &*manifest.0.read_value();
                    let prefetches = prefetches.0.read_value();

                    let all_prefetches = prefetches.iter().flat_map(|key| {
                        manifest.get(*key).into_iter().flatten()
                    });

                    for module in all_prefetches {
                        // to_html() on leptos_meta components registers them with the meta context,
                        // rather than returning HTML directly
                        _ = view! {
                            <Link
                                rel="preload"
                                href=format!("{pkg_path}/{module}.wasm")
                                as_="fetch"
                                type_="application/wasm"
                                crossorigin=nonce.clone()
                            />
                        }
                        .to_html();
                    }
                    _ = view! {
                        <Link rel="modulepreload" href=format!("{pkg_path}/{wasm_split_file}") crossorigin=nonce/>
                    }
                    .to_html();
                }
            }

            let mut stream = Box::pin(
                meta_context.inject_meta_context(stream).await.then({
                    let sc = Arc::clone(&sc);
                    move |chunk| {
                        let sc = Arc::clone(&sc);
                        async move {
                            while let Some(pending) = sc.await_deferred() {
                                pending.await;
                            }
                            chunk
                        }
                    }
                }),
            );

            // wait for the first chunk of the stream, then set the status and headers
            let first_chunk = stream.next().await.unwrap_or_default();

            // Cleanup of the owner is tied to `OwnerCleanupStream`'s `Drop`, so
            // it runs whether the body streams to completion or the client
            // disconnects early and the body is dropped.
            let mut res =
                Self::from_stream(Sandboxed::new(OwnerCleanupStream::new(
                    once(async move { first_chunk }).chain(stream),
                    owner,
                )));

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
        // this argument indicates whether a request wants to support out-of-order streaming
        // responses
        bool,
    ) -> PinnedFuture<PinnedStream<String>>,
    is_islands_router_navigation: bool,
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
                stream_builder(app, chunks, is_islands_router_navigation)
            });

            stream.await
        }
    }));
    (owner, stream)
}

/// Returns `true` if `path` could escape the site root once interpolated into
/// an on-disk path.
///
/// The static handlers build a filesystem path by concatenating the request
/// path onto the site root (`{site_root}/{path}.html`). The request path is not
/// percent-decoded upstream, so we reject a literal `..` segment, the
/// percent-encoded dot/separator sequences a later decoding step could turn
/// into one (`%2e`, `%2f`, `%5c`), and a backslash (a path separator on
/// Windows).
fn is_path_traversal(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    if lower.contains("%2e") || lower.contains("%2f") || lower.contains("%5c") {
        return true;
    }
    path.split('/')
        .any(|segment| segment == ".." || segment.contains('\\'))
}

/// Builds the on-disk path for the static file backing `path`, or returns
/// `None` if `path` would escape `options.site_root` via directory traversal.
///
/// Callers must treat `None` as a rejected request (respond `404` on the read
/// side, refuse to write on the generation side); the helper performs no
/// filesystem access and never returns a path outside the site root.
pub fn static_file_path(options: &LeptosOptions, path: &str) -> Option<String> {
    if is_path_traversal(path) {
        return None;
    }
    let trimmed_path = path.trim_start_matches('/');
    let path = if trimmed_path.is_empty() {
        "index"
    } else {
        trimmed_path
    };
    Some(format!("{}/{}.html", options.site_root, path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use leptos_config::LeptosOptions;

    fn opts() -> LeptosOptions {
        LeptosOptions::builder()
            .output_name("ignored")
            .site_root("/var/www/site/static")
            .build()
    }

    #[test]
    fn static_file_path_rejects_traversal() {
        let options = opts();
        // literal `..` segments
        assert_eq!(static_file_path(&options, "/../../etc/passwd"), None);
        assert_eq!(static_file_path(&options, "/posts/../../secret"), None);
        assert_eq!(static_file_path(&options, ".."), None);
        // percent-encoded dot / separators (path is not decoded upstream)
        assert_eq!(static_file_path(&options, "/..%2f..%2fetc"), None);
        assert_eq!(static_file_path(&options, "/%2e%2e/secret"), None);
        assert_eq!(static_file_path(&options, "/foo%2Fbar"), None);
        // backslash separator (Windows)
        assert_eq!(static_file_path(&options, "/..\\..\\secret"), None);
    }

    #[test]
    fn static_file_path_allows_legitimate_paths() {
        let options = opts();
        assert_eq!(
            static_file_path(&options, "/"),
            Some("/var/www/site/static/index.html".into())
        );
        assert_eq!(
            static_file_path(&options, "/posts/my-first-post"),
            Some("/var/www/site/static/posts/my-first-post.html".into())
        );
        // a single dot is harmless (stays in the same directory)
        assert_eq!(
            static_file_path(&options, "/a/./b"),
            Some("/var/www/site/static/a/./b.html".into())
        );
    }
}
