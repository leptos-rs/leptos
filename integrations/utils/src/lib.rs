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

            let stream = stream.await.ready_chunks(32).map(flush_ready_chunks);

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

/// Collapses one `ready_chunks` batch of stream chunks into a single item.
///
/// Batching already-ready chunks is an I/O win: each item becomes one HTTP
/// frame downstream. But while streaming, chunks usually become ready one
/// resource resolution at a time, so the typical batch holds exactly one
/// chunk — and `join("")` would allocate and copy even then. Move the only
/// chunk out instead, and only pay for concatenation when there is something
/// to concatenate.
fn flush_ready_chunks(mut chunks: Vec<String>) -> String {
    if chunks.len() == 1 {
        chunks.pop().unwrap_or_default()
    } else {
        chunks.join("")
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

/// Returns whether an `Accept` header value indicates the client will accept
/// an HTML response — i.e. an ordinary browser navigation or a plain `<form>`
/// submission, as opposed to a programmatic client expecting structured data.
///
/// Unlike a naive `contains("text/html")` check, each comma-separated media
/// range is parsed with the [`mime`] crate and an explicit `q=0` refusal is
/// honoured. So `text/html;q=0` (the client refusing HTML) and
/// `application/x-text/html-fake` (an unrelated, unparseable range) are both
/// correctly treated as *not* accepting HTML.
pub fn accept_header_includes_html(accept: &str) -> bool {
    // necessary-condition gate: any matching `text/html` range must contain
    // the substring "html", so a header without it can never match; this
    // skips the per-range mime parse for the common `application/json`
    // server-fn case. Only negatives short-circuit — anything that could
    // match still flows through the full parser below.
    if !accept
        .as_bytes()
        .windows(4)
        .any(|w| w.eq_ignore_ascii_case(b"html"))
    {
        return false;
    }
    accept.split(',').any(|range| {
        let Ok(media) = range.trim().parse::<mime::Mime>() else {
            return false;
        };
        if media.type_() != mime::TEXT || media.subtype() != mime::HTML {
            return false;
        }
        // honour an explicit `q=0`, which means the client refuses HTML
        match media.get_param("q") {
            Some(q) => {
                q.as_str().parse::<f32>().map(|w| w > 0.0).unwrap_or(true)
            }
            None => true,
        }
    })
}

/// Assembles a request URL of the form `scheme://host{path}`, appending
/// `?{query}` only when `query` is non-empty, into a single `String` whose
/// capacity is reserved up front from the component lengths.
///
/// Both server integrations reconstruct the full request URL on every SSR
/// render so that `Url::origin()` is correct server-side and matches the client
/// after hydration. Routing the components through intermediate `format!`
/// strings (a separate `scheme://host` origin, then the full URL) allocates
/// twice for nothing; the only copy that has to happen is the final one into
/// `RequestUrl`'s `Arc<str>`. When `path` already carries the query string
/// (axum exposes it as a single `path_and_query`), the caller passes an empty
/// `query`.
pub fn build_request_url(
    scheme: &str,
    host: &str,
    path: &str,
    query: &str,
) -> String {
    let mut url = String::with_capacity(
        scheme.len()
            + "://".len()
            + host.len()
            + path.len()
            + if query.is_empty() { 0 } else { 1 + query.len() },
    );
    url.push_str(scheme);
    url.push_str("://");
    url.push_str(host);
    url.push_str(path);
    if !query.is_empty() {
        url.push('?');
        url.push_str(query);
    }
    url
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
    // %2e/%2E, %2f/%2F, %5c/%5C — only the hex letter is case-insensitive,
    // so fold case on that byte alone (`| 0x20` lowercases ASCII letters)
    // instead of lowercasing a copy of the whole path
    if path.as_bytes().windows(3).any(|w| {
        w[0] == b'%'
            && ((w[1] == b'2' && matches!(w[2] | 0x20, b'e' | b'f'))
                || (w[1] == b'5' && (w[2] | 0x20) == b'c'))
    }) {
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

/// Writes `contents` to `path` atomically: the file appears with its full
/// contents or not at all.
///
/// The bytes are written to a uniquely-named temp file in the same directory,
/// which is then renamed over the target. `rename` is atomic on POSIX
/// filesystems (and replaces the destination on Windows), so a concurrent
/// reader or a crash never observes a half-written or truncated file. Writing
/// in place (e.g. `tokio::fs::write`) truncates the target up front, which would
/// leave an empty file visible (and served with `try_exists == true`) if the
/// process died mid-write.
///
/// Missing parent directories are created first. If the rename fails, the temp
/// file is removed on a best-effort basis so failures don't leave stray temp
/// files accumulating in the site root.
#[cfg(feature = "fs")]
pub async fn write_file_atomic(
    path: &std::path::Path,
    contents: &[u8],
) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let tmp_path = {
        static COUNTER: std::sync::atomic::AtomicU64 =
            std::sync::atomic::AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let mut file_name = path
            .file_name()
            .map(|name| name.to_os_string())
            .unwrap_or_default();
        file_name.push(format!(".tmp.{}.{n}", std::process::id()));
        path.with_file_name(file_name)
    };

    tokio::fs::write(&tmp_path, contents).await?;
    if let Err(err) = tokio::fs::rename(&tmp_path, path).await {
        let _ = tokio::fs::remove_file(&tmp_path).await;
        return Err(err);
    }

    Ok(())
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

    #[test]
    fn flush_ready_chunks_moves_singletons_and_joins_batches() {
        assert_eq!(flush_ready_chunks(Vec::new()), "");
        assert_eq!(flush_ready_chunks(vec!["a".to_string()]), "a");
        assert_eq!(
            flush_ready_chunks(vec![
                "a".to_string(),
                "b".to_string(),
                "c".to_string()
            ]),
            "abc"
        );
    }

    #[test]
    fn accept_header_plain_navigation_is_html() {
        // typical browser navigation
        assert!(accept_header_includes_html(
            "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8"
        ));
        assert!(accept_header_includes_html("text/html"));
        assert!(accept_header_includes_html("text/html; charset=utf-8"));
        assert!(accept_header_includes_html("text/html;q=0.1"));
    }

    #[test]
    fn accept_header_explicit_refusal_is_not_html() {
        // `q=0` means the client explicitly does not want HTML
        assert!(!accept_header_includes_html(
            "text/html;q=0, application/json"
        ));
        assert!(!accept_header_includes_html("text/html;q=0.0"));
    }

    #[test]
    fn accept_header_substring_is_not_html() {
        // these contain the literal substring "text/html" but are not it
        assert!(!accept_header_includes_html("application/x-text/html-fake"));
        assert!(!accept_header_includes_html("application/json"));
        assert!(!accept_header_includes_html("*/*"));
    }

    // `write_file_atomic` must create the file (and any missing parents) with
    // its full contents and leave no temp file behind, so a crash mid-write can
    // never expose a truncated or empty file to a reader.
    #[cfg(feature = "fs")]
    #[tokio::test]
    async fn write_file_atomic_writes_full_contents_without_leftovers() {
        let dir = std::env::temp_dir().join(format!(
            "leptos_integration_utils_atomic_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));

        // a missing parent directory is created on the way to the target
        let target = dir.join("nested").join("page.html");
        let contents = b"<html><body>hello</body></html>";
        write_file_atomic(&target, contents).await.unwrap();

        // the target was written in full
        assert_eq!(std::fs::read(&target).unwrap(), contents);

        // no temp file was left behind alongside it
        let leftovers = std::fs::read_dir(target.parent().unwrap())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().contains(".tmp."))
            .count();
        assert_eq!(leftovers, 0);

        std::fs::remove_dir_all(&dir).ok();
    }
}
