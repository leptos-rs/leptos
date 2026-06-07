use super::{SerializedDataId, SharedContext};
use crate::{PinnedFuture, PinnedStream};
use futures::{
    Stream, StreamExt,
    future::join_all,
    stream::{self, FuturesUnordered, once},
};
use or_poisoned::OrPoisoned;
use std::{
    borrow::Cow,
    collections::HashSet,
    fmt::{Debug, Write},
    future::Future,
    mem,
    pin::Pin,
    sync::{
        Arc, Mutex, RwLock,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    task::{Context, Poll},
};
use throw_error::{Error, ErrorId};

type AsyncDataBuf = Arc<RwLock<Vec<(SerializedDataId, PinnedFuture<String>)>>>;
type ErrorBuf = Arc<RwLock<Vec<(SerializedDataId, ErrorId, Error)>>>;
type SealedErrors = Arc<RwLock<HashSet<SerializedDataId>>>;

/// The shared context that should be used on the server side.
pub struct SsrSharedContext {
    id: AtomicUsize,
    non_hydration_id: AtomicUsize,
    is_hydrating: AtomicBool,
    sync_buf: RwLock<Vec<ResolvedData>>,
    async_buf: AsyncDataBuf,
    errors: ErrorBuf,
    sealed_error_boundaries: SealedErrors,
    deferred: Mutex<Vec<PinnedFuture<()>>>,
    incomplete: Arc<Mutex<Vec<SerializedDataId>>>,
}

impl Default for SsrSharedContext {
    fn default() -> Self {
        Self::new()
    }
}

impl SsrSharedContext {
    /// Creates a new shared context for rendering HTML on the server.
    pub fn new() -> Self {
        Self {
            id: AtomicUsize::new(0),
            non_hydration_id: AtomicUsize::new(usize::MAX),
            is_hydrating: AtomicBool::new(true),
            sync_buf: RwLock::default(),
            async_buf: AsyncDataBuf::default(),
            errors: ErrorBuf::default(),
            sealed_error_boundaries: SealedErrors::default(),
            deferred: Mutex::default(),
            incomplete: Arc::default(),
        }
    }

    /// Creates a new shared context for rendering HTML on the server in "islands" mode.
    ///
    /// This defaults to a mode in which the app is not hydrated, but allows you to opt into
    /// hydration for certain portions using [`SharedContext::set_is_hydrating`].
    pub fn new_islands() -> Self {
        Self {
            is_hydrating: AtomicBool::new(false),
            ..Self::new()
        }
    }

    /// Consume the data buffers, awaiting all async resources,
    /// returning both sync and async buffers.
    /// Useful to implement custom hydration contexts.
    ///
    /// WARNING: this will clear the internal buffers, it should only be called once.
    /// A second call would return an empty `vec![]`.
    pub async fn consume_buffers(&self) -> Vec<(SerializedDataId, String)> {
        let sync_data = mem::take(&mut *self.sync_buf.write().or_poisoned());
        let async_data = mem::take(&mut *self.async_buf.write().or_poisoned());

        let mut all_data: Vec<(SerializedDataId, String)> =
            sync_data.into_iter().map(|r| (r.0, r.1)).collect();

        // Resolve the async futures concurrently. Note this is *not* a
        // wall-clock win for leptos's own resources: the futures they
        // register via `write_async` are thin "wait until ready, then
        // serialize" wrappers, and the actual work (DB query, fetch) runs on
        // a task the resource spawns itself, so awaiting the wrappers
        // sequentially already completed in max (not sum) latency. But
        // `write_async` is public API, and a registered future that does its
        // work inline is driven only here; `join_all` keeps such futures
        // from serializing behind one another. It preserves input order, so
        // the returned Vec is identical to the sequential version.
        let resolved = join_all(
            async_data
                .into_iter()
                .map(|(id, fut)| async move { (id, fut.await) }),
        )
        .await;
        all_data.extend(resolved);
        all_data
    }
}

impl Debug for SsrSharedContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SsrSharedContext")
            .field("id", &self.id)
            .field("is_hydrating", &self.is_hydrating)
            .field("sync_buf", &self.sync_buf)
            .field("async_buf", &self.async_buf.read().or_poisoned().len())
            .finish()
    }
}

impl SharedContext for SsrSharedContext {
    fn is_browser(&self) -> bool {
        false
    }

    #[track_caller]
    fn next_id(&self) -> SerializedDataId {
        let id = if self.get_is_hydrating() {
            self.id.fetch_add(1, Ordering::Relaxed)
        } else {
            self.non_hydration_id.fetch_sub(1, Ordering::Relaxed)
        };
        SerializedDataId(id)
    }

    fn write_async(&self, id: SerializedDataId, fut: PinnedFuture<String>) {
        self.async_buf.write().or_poisoned().push((id, fut))
    }

    fn read_data(&self, _id: &SerializedDataId) -> Option<String> {
        None
    }

    fn await_data(&self, _id: &SerializedDataId) -> Option<String> {
        None
    }

    fn get_is_hydrating(&self) -> bool {
        self.is_hydrating.load(Ordering::SeqCst)
    }

    fn set_is_hydrating(&self, is_hydrating: bool) {
        self.is_hydrating.store(is_hydrating, Ordering::SeqCst)
    }

    fn errors(&self, boundary_id: &SerializedDataId) -> Vec<(ErrorId, Error)> {
        self.errors
            .read()
            .or_poisoned()
            .iter()
            .filter_map(|(boundary, id, error)| {
                if boundary == boundary_id {
                    Some((id.clone(), error.clone()))
                } else {
                    None
                }
            })
            .collect()
    }

    fn register_error(
        &self,
        error_boundary_id: SerializedDataId,
        error_id: ErrorId,
        error: Error,
    ) {
        self.errors.write().or_poisoned().push((
            error_boundary_id,
            error_id,
            error,
        ));
    }

    fn take_errors(&self) -> Vec<(SerializedDataId, ErrorId, Error)> {
        mem::take(&mut *self.errors.write().or_poisoned())
    }

    fn seal_errors(&self, boundary_id: &SerializedDataId) {
        self.sealed_error_boundaries
            .write()
            .or_poisoned()
            .insert(boundary_id.clone());
    }

    fn pending_data(&self) -> Option<PinnedStream<String>> {
        let sync_data = mem::take(&mut *self.sync_buf.write().or_poisoned());

        // snapshot the set of async resource ids known at this moment.
        // The client receives this exact set as the initial
        // `__PENDING_RESOURCES=[...]` literal. Any resource registered
        // *after* this snapshot (e.g. a nested Suspense / child server
        // fn that calls `write_async` from inside a parent resource's
        // future) is announced later as `__PENDING_RESOURCES.push(id);`
        // before its `__RESOLVED_RESOURCES[id] = ...` write, so the
        // client always sees the id in `__PENDING_RESOURCES` before the
        // matching resolution.
        let initial_pending_ids: HashSet<SerializedDataId> = self
            .async_buf
            .read()
            .or_poisoned()
            .iter()
            .map(|(id, _)| id.clone())
            .collect();

        // snapshot incomplete-chunk markers known at this moment. Declaring
        // the array up front lets client-side `get_incomplete_chunk` probes
        // that fire during streaming see a defined (possibly empty) array
        // rather than `undefined`. Later `set_incomplete_chunk` calls are
        // streamed as `__INCOMPLETE_CHUNKS.push(id);` from
        // `AsyncDataStream::poll_next`.
        let initial_incomplete: Vec<SerializedDataId> =
            self.incomplete.lock().or_poisoned().clone();
        let initial_incomplete_count = initial_incomplete.len();

        // 1) initial, synchronous setup chunk
        let mut initial_chunk = String::new();
        // resolved synchronous resources and errors
        initial_chunk.push_str("__RESOLVED_RESOURCES=[");
        for resolved in sync_data {
            resolved.write_to_buf(&mut initial_chunk);
            initial_chunk.push(',');
        }
        initial_chunk.push_str("];");

        initial_chunk.push_str("__SERIALIZED_ERRORS=[");
        for error in mem::take(&mut *self.errors.write().or_poisoned()) {
            // Debug-format first to get a valid, quoted JS string literal
            // (escaping `"`, `\`, control chars), then rewrite every remaining
            // `<` to a single-backslash `<` JS unicode escape. Escaping
            // *after* `{:?}` keeps it one backslash, so the HTML tokenizer
            // never sees `</script>` while the browser's JS string parser
            // still decodes `<` straight back to `<` for the consumer.
            let formatted = format!("{:?}", error.2.to_string());
            let msg = escape_lt(&formatted);
            _ = write!(initial_chunk, "[{}, {}, {}],", error.0.0, error.1, msg);
        }
        initial_chunk.push_str("];");

        // pending async resources known at snapshot time
        initial_chunk.push_str("__PENDING_RESOURCES=[");
        for id in &initial_pending_ids {
            _ = write!(&mut initial_chunk, "{},", id.0);
        }
        initial_chunk.push_str("];");

        // incomplete-chunk markers known at snapshot time. Before this
        // change, `__INCOMPLETE_CHUNKS` was only written by the tail chunk
        // *after every async resource had resolved* — so any hydration
        // probe before that point saw `undefined` and treated
        // fallback-state chunks as complete.
        initial_chunk.push_str("__INCOMPLETE_CHUNKS=[");
        for id in &initial_incomplete {
            _ = write!(&mut initial_chunk, "{},", id.0);
        }
        initial_chunk.push_str("];");

        // resolvers
        initial_chunk.push_str("__RESOURCE_RESOLVERS=[];");

        let incomplete_emitted =
            Arc::new(AtomicUsize::new(initial_incomplete_count));

        let async_data = AsyncDataStream {
            async_buf: Arc::clone(&self.async_buf),
            in_flight: FuturesUnordered::new(),
            errors: Arc::clone(&self.errors),
            sealed_error_boundaries: Arc::clone(&self.sealed_error_boundaries),
            incomplete: Arc::clone(&self.incomplete),
            incomplete_emitted: Arc::clone(&incomplete_emitted),
            initial_pending_ids,
        };

        let incomplete = Arc::clone(&self.incomplete);

        let stream = stream::once(async move { initial_chunk })
            .chain(async_data)
            .chain(once(async move {
                // final flush: emit pushes for any markers that landed after
                // `AsyncDataStream` finished. The internal `incomplete` Vec
                // is the source of truth for server-side `get_incomplete_chunk`
                // queries; we read past the cursor instead of draining so
                // those queries continue to work.
                let lock = incomplete.lock().or_poisoned();
                let from = incomplete_emitted.load(Ordering::Relaxed);
                let mut script = String::new();
                for entry in lock.iter().skip(from) {
                    _ = write!(
                        script,
                        "__INCOMPLETE_CHUNKS.push({});",
                        entry.0
                    );
                }
                script
            }));
        Some(Box::pin(stream))
    }

    fn during_hydration(&self) -> bool {
        false
    }

    fn hydration_complete(&self) {}

    fn defer_stream(&self, wait_for: PinnedFuture<()>) {
        self.deferred.lock().or_poisoned().push(wait_for);
    }

    fn await_deferred(&self) -> Option<PinnedFuture<()>> {
        let deferred = mem::take(&mut *self.deferred.lock().or_poisoned());
        if deferred.is_empty() {
            None
        } else {
            Some(Box::pin(async move {
                join_all(deferred).await;
            }))
        }
    }

    fn set_incomplete_chunk(&self, id: SerializedDataId) {
        self.incomplete.lock().or_poisoned().push(id);
    }

    fn get_incomplete_chunk(&self, id: &SerializedDataId) -> bool {
        self.incomplete
            .lock()
            .or_poisoned()
            .iter()
            .any(|entry| entry == id)
    }
}

/// Pairs a registered resource future with its id so a [`FuturesUnordered`]
/// can yield `(id, data)` on completion. The id is cloned only when the inner
/// future resolves, never on a pending poll.
struct IdFuture {
    id: SerializedDataId,
    fut: PinnedFuture<String>,
}

impl Future for IdFuture {
    type Output = (SerializedDataId, String);

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // `IdFuture` is `Unpin` (the boxed future is `Unpin`), so projecting to
        // the inner future through a plain `&mut` is sound.
        let this = self.get_mut();
        match this.fut.as_mut().poll(cx) {
            Poll::Ready(data) => Poll::Ready((this.id.clone(), data)),
            Poll::Pending => Poll::Pending,
        }
    }
}

struct AsyncDataStream {
    /// Staging buffer for futures registered via `write_async`, including
    /// re-entrant registrations made while another future is being polled.
    /// Each `poll_next` moves these into `in_flight`.
    async_buf: AsyncDataBuf,
    /// The set of in-flight resource futures. `FuturesUnordered` routes each
    /// leaf future's waker individually, so a wake re-polls only the futures
    /// that advanced (O(N) total polls) rather than the whole backlog (O(N^2)).
    in_flight: FuturesUnordered<IdFuture>,
    errors: ErrorBuf,
    sealed_error_boundaries: SealedErrors,
    incomplete: Arc<Mutex<Vec<SerializedDataId>>>,
    /// Number of entries from `incomplete` that have already been emitted
    /// as `__INCOMPLETE_CHUNKS.push(...)` statements (or as the initial
    /// `__INCOMPLETE_CHUNKS=[...]` literal). The final tail closure picks
    /// up wherever the stream left off.
    incomplete_emitted: Arc<AtomicUsize>,
    /// IDs that were already advertised in the initial
    /// `__PENDING_RESOURCES=[...]` literal. Any resource resolved by
    /// `poll_next` whose id is NOT in this set must first emit a
    /// `__PENDING_RESOURCES.push(id);` so the client sees it as
    /// pending before its resolution arrives.
    initial_pending_ids: HashSet<SerializedDataId>,
}

impl Stream for AsyncDataStream {
    type Item = String;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        // `AsyncDataStream` is `Unpin` (every field is `Unpin`), so operating
        // on a plain `&mut Self` for the rest of the method is sound.
        let this = self.get_mut();

        let mut resolved = String::new();

        // Move newly-registered futures into the in-flight set and drain every
        // future that is *currently* ready, without re-polling the ones that
        // are still pending. The lock is never held across a `poll`, so a
        // polled future may call back into `SsrSharedContext::write_async`
        // (re-entrant nested Suspense / child resources) without deadlocking;
        // those registrations land in `async_buf` and are picked up by the next
        // iteration of this loop, or by the next `poll_next` call.
        loop {
            {
                let mut buf = this.async_buf.write().or_poisoned();
                for (id, fut) in buf.drain(..) {
                    this.in_flight.push(IdFuture { id, fut });
                }
            }

            let mut any_ready = false;
            while let Poll::Ready(Some((id, data))) =
                this.in_flight.poll_next_unpin(cx)
            {
                any_ready = true;
                // Any resource id that was not in the initial
                // `__PENDING_RESOURCES=[...]` snapshot must be announced before
                // its resolution arrives, so the client never sees a
                // `__RESOLVED_RESOURCES[id] = ...` for an id that never appeared
                // in `__PENDING_RESOURCES`.
                if !this.initial_pending_ids.contains(&id) {
                    _ = write!(resolved, "__PENDING_RESOURCES.push({});", id.0);
                }
                let data = escape_lt(&data);
                _ = write!(
                    resolved,
                    "__RESOLVED_RESOURCES[{}] = {:?};",
                    id.0, data
                );
            }

            // A future resolved above may have registered new work via
            // `write_async`; re-stage and poll it now so an immediately-ready
            // re-entrant child is not stranded waiting for an unrelated wake.
            // Loop only when there is actually something new to stage, which
            // bounds the iteration count by the resource registration depth.
            if any_ready && !this.async_buf.read().or_poisoned().is_empty() {
                continue;
            }
            break;
        }

        // Emit pushes for any incomplete-chunk markers added since the
        // last poll (or set by a future we just polled, e.g. a Suspense
        // boundary that flipped into the fallback state via
        // `SsrSharedContext::set_incomplete_chunk`).
        {
            let lock = this.incomplete.lock().or_poisoned();
            let from = this.incomplete_emitted.load(Ordering::Relaxed);
            for entry in lock.iter().skip(from) {
                _ = write!(resolved, "__INCOMPLETE_CHUNKS.push({});", entry.0);
            }
            this.incomplete_emitted.store(lock.len(), Ordering::Relaxed);
        }

        let sealed = this.sealed_error_boundaries.read().or_poisoned();
        for error in mem::take(&mut *this.errors.write().or_poisoned()) {
            if !sealed.contains(&error.0) {
                // see the initial-chunk path: Debug-format, then single-
                // backslash-escape `<` so the JS parser decodes it back to `<`
                let formatted = format!("{:?}", error.2.to_string());
                let msg = escape_lt(&formatted);
                _ = write!(
                    resolved,
                    "__SERIALIZED_ERRORS.push([{}, {}, {}]);",
                    error.0.0, error.1, msg
                );
            }
        }
        drop(sealed);

        // The stream is exhausted only when nothing is in flight *and* no
        // re-entrant registration is waiting to be staged. Any still-pending
        // in-flight future had its waker registered by the drain loop above, so
        // returning `Pending` here will be re-woken.
        let done = this.in_flight.is_empty()
            && this.async_buf.read().or_poisoned().is_empty();

        if done && resolved.is_empty() {
            return Poll::Ready(None);
        }
        if resolved.is_empty() {
            return Poll::Pending;
        }

        Poll::Ready(Some(resolved))
    }
}

/// Escape `<` to its JS unicode escape `<`, allocating only when the input
/// actually contains a `<`. `str::replace` always allocates a fresh `String`
/// and copies the whole input even when the pattern is absent; `<` is rare in
/// serialized payloads (it only appears when a string field carries markup), so
/// guarding the replacement behind a single byte scan skips an allocation and a
/// full O(len) copy on the common path. `{:?}`/`{}` format `&str` and `String`
/// to identical bytes, so the emitted output is byte-for-byte unchanged.
fn escape_lt(input: &str) -> Cow<'_, str> {
    if input.as_bytes().contains(&b'<') {
        Cow::Owned(input.replace('<', "\\u003c"))
    } else {
        Cow::Borrowed(input)
    }
}

#[derive(Debug)]
struct ResolvedData(SerializedDataId, String);

impl ResolvedData {
    pub fn write_to_buf(&self, buf: &mut String) {
        let ResolvedData(id, ser) = self;
        // escapes < to prevent it being interpreted as another opening HTML tag
        let ser = escape_lt(ser);
        write!(buf, "{}: {:?}", id.0, ser).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::{StreamExt, executor::block_on};
    use std::fmt;

    #[derive(Debug)]
    struct CustomError(&'static str);

    impl fmt::Display for CustomError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(self.0)
        }
    }

    impl std::error::Error for CustomError {}

    /// An error message containing `</script>` must not be able to escape
    /// the surrounding <script> tag in the streamed initial chunk.
    #[test]
    fn error_in_initial_chunk_escapes_script_close_tag() {
        let ctx = SsrSharedContext::new();
        ctx.register_error(
            SerializedDataId(0),
            ErrorId::from(0_usize),
            Error::from(CustomError(
                "boom</script><script>alert('pwned')</script><script>",
            )),
        );

        let mut stream = ctx.pending_data().expect("pending_data on ssr");
        let initial = block_on(stream.next()).expect("at least one chunk");

        assert!(
            !initial.contains("</script>"),
            "initial chunk must not contain a literal `</script>` substring, \
             got: {initial}"
        );
        assert!(
            !initial.contains('<'),
            "initial chunk must not contain a literal `<` character anywhere \
             inside the serialized errors, got: {initial}"
        );
        assert!(
            initial.contains("\\u003c") && !initial.contains("\\\\u003c"),
            "expected a single-backslash `\\u003c` escape in place of `<` (a \
             double backslash would be decoded to a literal `\\u003c` and \
             shown raw to the user), got: {initial}"
        );
    }

    /// `SsrSharedContext::default()` must behave identically to
    /// `SsrSharedContext::new()`. In particular, `is_hydrating` must be
    /// `true` and `non_hydration_id` must start at `usize::MAX`, so the
    /// hydrating counter (starting at 0) and the non-hydrating counter
    /// (decrementing from `usize::MAX`) cannot collide for either path.
    #[test]
    fn default_matches_new_for_id_minting() {
        let new_ctx = SsrSharedContext::new();
        let default_ctx = SsrSharedContext::default();

        // both must be in hydrating mode out of the box
        assert!(new_ctx.get_is_hydrating());
        assert!(default_ctx.get_is_hydrating());

        // hydrating IDs start at 0 and increment
        assert_eq!(default_ctx.next_id(), SerializedDataId(0));
        assert_eq!(default_ctx.next_id(), SerializedDataId(1));

        // non-hydrating IDs decrement from usize::MAX, NOT from 0
        default_ctx.set_is_hydrating(false);
        let first_non = default_ctx.next_id();
        let second_non = default_ctx.next_id();
        assert_eq!(first_non, SerializedDataId(usize::MAX));
        assert_eq!(second_non, SerializedDataId(usize::MAX - 1));

        // and a hydrating id minted later must not collide with the
        // non-hydrating ones above
        default_ctx.set_is_hydrating(true);
        let next_hyd = default_ctx.next_id();
        assert_eq!(next_hyd, SerializedDataId(2));
        assert_ne!(next_hyd, first_non);
        assert_ne!(next_hyd, second_non);
    }

    /// A future being polled by `AsyncDataStream::poll_next` must be able
    /// to call back into `SsrSharedContext::write_async` on the same
    /// context without deadlocking — the poll loop must not hold the
    /// `async_buf` write lock across the user-future poll.
    #[test]
    fn async_stream_does_not_deadlock_on_reentrant_write_async() {
        let ctx = Arc::new(SsrSharedContext::new());

        // A "parent" resource future that, on its first poll, registers a
        // sibling resource on the same context. This mirrors what real
        // Leptos code does when one resource creates another while running
        // (nested Suspense, child resources spawned inside a parent, etc.).
        let ctx_for_fut = Arc::clone(&ctx);
        ctx.write_async(
            SerializedDataId(0),
            Box::pin(async move {
                ctx_for_fut.write_async(
                    SerializedDataId(1),
                    Box::pin(async { String::from("\"child\"") }),
                );
                String::from("\"parent\"")
            }),
        );

        let mut stream = ctx.pending_data().expect("pending_data on ssr");

        // Without the fix, on most platforms this `next().await` would
        // deadlock (the inner write_async re-acquires a write lock the
        // poll loop is already holding). With the fix it must complete.
        let mut chunks = Vec::new();
        while let Some(c) = block_on(stream.next()) {
            chunks.push(c);
        }

        let joined = chunks.join("");
        assert!(
            joined.contains("__RESOLVED_RESOURCES[0]"),
            "parent resource must resolve: {joined}"
        );
        assert!(
            joined.contains("__RESOLVED_RESOURCES[1]"),
            "child resource registered re-entrantly must also resolve: \
             {joined}"
        );
    }

    /// The same escape must be applied to errors emitted later via the
    /// async stream (AsyncDataStream::poll_next).
    #[test]
    fn error_in_async_stream_escapes_script_close_tag() {
        let ctx = SsrSharedContext::new();

        // park one async resource so AsyncDataStream emits a follow-up chunk
        ctx.write_async(
            SerializedDataId(1),
            Box::pin(async { String::from("\"ok\"") }),
        );

        let mut stream = ctx.pending_data().expect("pending_data on ssr");
        // skip the initial setup chunk; we want the next one
        let _initial = block_on(stream.next()).expect("initial chunk");

        // register an error after pending_data() has been called so it is
        // serialized through the streaming path rather than the initial chunk
        ctx.register_error(
            SerializedDataId(2),
            ErrorId::from(7_usize),
            Error::from(CustomError("late</script><script>x</script>")),
        );

        let mut saw_error = false;
        while let Some(chunk) = block_on(stream.next()) {
            if chunk.contains("__SERIALIZED_ERRORS.push") {
                saw_error = true;
                assert!(
                    !chunk.contains("</script>"),
                    "streamed error chunk must not contain `</script>`: \
                     {chunk}"
                );
                assert!(
                    chunk.contains("\\u003c") && !chunk.contains("\\\\u003c"),
                    "streamed error chunk should carry a single-backslash \
                     escaped `<`: {chunk}"
                );
            }
        }
        assert!(
            saw_error,
            "expected at least one streamed __SERIALIZED_ERRORS.push chunk"
        );
    }

    /// `__INCOMPLETE_CHUNKS` must be declared in the initial chunk so that
    /// client-side `get_incomplete_chunk` probes that fire while the stream
    /// is still being delivered see a defined array. Markers that arrive
    /// later (via `set_incomplete_chunk` from inside a Suspense future)
    /// must be streamed as `__INCOMPLETE_CHUNKS.push(id);` *during*
    /// `AsyncDataStream::poll_next`, not only in the final tail chunk.
    #[test]
    fn incomplete_chunks_array_is_declared_early_and_pushed_live() {
        let ctx = Arc::new(SsrSharedContext::new());

        // Suspense-shaped future: while being polled, it marks chunk 42 as
        // incomplete and then resolves with some data. Mirrors what happens
        // when a <Suspense> boundary detects local resources mid-resolution
        // and calls `sc.set_incomplete_chunk(self.id)`.
        let ctx_for_fut = Arc::clone(&ctx);
        ctx.write_async(
            SerializedDataId(1),
            Box::pin(async move {
                ctx_for_fut.set_incomplete_chunk(SerializedDataId(42));
                String::from("\"ok\"")
            }),
        );

        let mut stream = ctx.pending_data().expect("pending_data on ssr");

        let initial = block_on(stream.next()).expect("initial chunk");
        assert!(
            initial.contains("__INCOMPLETE_CHUNKS=[];"),
            "initial chunk must declare an empty __INCOMPLETE_CHUNKS array up \
             front, got: {initial}"
        );

        // Collect everything else; the push must appear *somewhere* in the
        // stream — not exclusively in the final tail chunk.
        let mut rest = String::new();
        let mut chunks_after_initial = 0;
        while let Some(c) = block_on(stream.next()) {
            chunks_after_initial += 1;
            rest.push_str(&c);
        }

        assert!(
            rest.contains("__INCOMPLETE_CHUNKS.push(42);"),
            "expected a live __INCOMPLETE_CHUNKS.push(42) during streaming, \
             got: {rest}"
        );
        assert!(
            chunks_after_initial >= 1,
            "expected at least one streamed chunk after the initial one"
        );
    }

    /// A marker emitted from inside an async resource must be flushed in
    /// the same chunk as the resource that produced it, so a slow client
    /// that hasn't seen the tail yet still observes both together.
    /// A resource registered *after* `pending_data()` snapshots
    /// `__PENDING_RESOURCES` must be announced via
    /// `__PENDING_RESOURCES.push(id);` before its
    /// `__RESOLVED_RESOURCES[id] = ...` write. A resource that *was* in
    /// the initial snapshot must NOT be pushed again.
    #[test]
    fn late_registered_resource_pushes_to_pending_resources() {
        let ctx = Arc::new(SsrSharedContext::new());
        ctx.write_async(
            SerializedDataId(1),
            Box::pin(async { String::from("\"a\"") }),
        );

        let mut stream = ctx.pending_data().expect("pending_data on ssr");
        let initial = block_on(stream.next()).expect("initial chunk");
        assert!(
            initial.contains("__PENDING_RESOURCES=[1,];"),
            "initial chunk must list id 1 in __PENDING_RESOURCES: {initial}"
        );

        // late registration: this id (2) is not in the initial snapshot
        ctx.write_async(
            SerializedDataId(2),
            Box::pin(async { String::from("\"b\"") }),
        );

        let mut rest = String::new();
        while let Some(c) = block_on(stream.next()) {
            rest.push_str(&c);
        }

        // id 2 must be pushed and resolved (in that order)
        let push_2 = rest.find("__PENDING_RESOURCES.push(2);");
        let resolve_2 = rest.find("__RESOLVED_RESOURCES[2]");
        assert!(
            push_2.is_some() && resolve_2.is_some(),
            "late resource id 2 needs both push and resolve, got: {rest}"
        );
        assert!(
            push_2.unwrap() < resolve_2.unwrap(),
            "the push must precede the resolve, got: {rest}"
        );

        // id 1 was already in the initial snapshot — it must NOT be
        // pushed again
        assert!(
            !rest.contains("__PENDING_RESOURCES.push(1);"),
            "id 1 was in the initial snapshot and must not be pushed again, \
             got: {rest}"
        );
    }

    #[test]
    fn incomplete_chunk_push_flushes_with_resolving_future() {
        let ctx = Arc::new(SsrSharedContext::new());

        let ctx_for_fut = Arc::clone(&ctx);
        ctx.write_async(
            SerializedDataId(7),
            Box::pin(async move {
                ctx_for_fut.set_incomplete_chunk(SerializedDataId(7));
                String::from("\"resolved\"")
            }),
        );

        let mut stream = ctx.pending_data().expect("pending_data on ssr");
        let _initial = block_on(stream.next()).expect("initial chunk");
        let live = block_on(stream.next()).expect("live chunk");

        let push_pos = live.find("__INCOMPLETE_CHUNKS.push(7);");
        let resolve_pos = live.find("__RESOLVED_RESOURCES[7]");
        assert!(
            push_pos.is_some() && resolve_pos.is_some(),
            "both push and resolve must appear in the live chunk: {live}"
        );
    }

    /// A payload without `<` must be returned borrowed (no allocation); a
    /// payload containing `<` must have every occurrence rewritten to a
    /// single-backslash `<` escape.
    #[test]
    fn escape_lt_guards_allocation_and_preserves_output() {
        let clean = "{\"k\":\"value\",\"n\":42}";
        assert!(
            matches!(escape_lt(clean), Cow::Borrowed(s) if s == clean),
            "a payload without `<` must be returned borrowed (no allocation)"
        );

        let dirty = "a<b<c";
        assert!(
            matches!(escape_lt(dirty), Cow::Owned(ref s) if s == "a\\u003cb\\u003cc"),
            "every `<` must be rewritten to a single-backslash \\u003c escape"
        );
    }

    /// The guarded escape must produce byte-for-byte the same serialized
    /// output as the previous unconditional `str::replace` implementation,
    /// for payloads both with and without `<`.
    #[test]
    fn resolved_data_serialization_matches_unconditional_replace() {
        for ser in [
            "{\"x\":1}",
            "has<angle",
            "</script><script>x</script>",
            "no-bracket-here",
            "",
        ] {
            let mut guarded = String::new();
            ResolvedData(SerializedDataId(3), ser.to_string())
                .write_to_buf(&mut guarded);

            // byte-identical reference: the previous unconditional formulation
            let reference =
                format!("{}: {:?}", 3usize, ser.replace('<', "\\u003c"));

            assert_eq!(
                guarded, reference,
                "serialized output changed for input {ser:?}"
            );
        }
    }

    /// When resources resolve at distinct times, a wake must re-poll only the
    /// future that advanced, not the whole pending backlog. The previous
    /// flat-`Vec` design re-polled every still-pending future on every wake
    /// (N + N-1 + ... = O(N^2) leaf polls); the `FuturesUnordered` design
    /// routes each leaf waker individually, giving O(N).
    #[test]
    fn async_stream_repolls_only_woken_futures_not_whole_backlog() {
        use std::task::Waker;

        // A leaf future that parks until armed. While parked it records its own
        // (FuturesUnordered proxy) waker and returns Pending WITHOUT self-waking,
        // so it is re-polled only when *that* waker fires. Every poll bumps a
        // shared counter.
        struct Gate {
            polls: Arc<AtomicUsize>,
            armed: Arc<AtomicBool>,
            waker: Arc<Mutex<Option<Waker>>>,
        }
        impl Future for Gate {
            type Output = String;
            fn poll(
                self: Pin<&mut Self>,
                cx: &mut Context<'_>,
            ) -> Poll<String> {
                self.polls.fetch_add(1, Ordering::Relaxed);
                if self.armed.load(Ordering::Relaxed) {
                    Poll::Ready(String::from("\"x\""))
                } else {
                    *self.waker.lock().or_poisoned() = Some(cx.waker().clone());
                    Poll::Pending
                }
            }
        }

        const N: usize = 64;
        let polls = Arc::new(AtomicUsize::new(0));
        let ctx = SsrSharedContext::new();
        let mut armed = Vec::with_capacity(N);
        let mut wakers = Vec::with_capacity(N);
        for i in 0..N {
            let a = Arc::new(AtomicBool::new(false));
            let w = Arc::new(Mutex::new(None));
            armed.push(Arc::clone(&a));
            wakers.push(Arc::clone(&w));
            ctx.write_async(
                SerializedDataId(i),
                Box::pin(Gate {
                    polls: Arc::clone(&polls),
                    armed: a,
                    waker: w,
                }),
            );
        }

        // No-op task waker; FuturesUnordered wakes it, but the test drives
        // polling by hand so it can wake exactly one leaf future per round.
        let mut stream = ctx.pending_data().expect("pending_data on ssr");
        let mut cx = Context::from_waker(Waker::noop());

        // initial chunk (does not touch the leaf futures)
        assert!(matches!(
            stream.poll_next_unpin(&mut cx),
            Poll::Ready(Some(_))
        ));
        // first AsyncDataStream poll: stage + poll all N once -> all Pending
        assert!(matches!(stream.poll_next_unpin(&mut cx), Poll::Pending));
        assert_eq!(
            polls.load(Ordering::Relaxed),
            N,
            "first poll must touch each future exactly once"
        );

        // resolve futures one at a time, waking only that future each round
        for i in 0..N {
            armed[i].store(true, Ordering::Relaxed);
            if let Some(w) = wakers[i].lock().or_poisoned().take() {
                w.wake();
            }
            let needle = format!("__RESOLVED_RESOURCES[{i}]");
            loop {
                match stream.poll_next_unpin(&mut cx) {
                    Poll::Ready(Some(chunk)) => {
                        assert!(
                            chunk.contains(&needle),
                            "round {i} expected {needle}, got: {chunk}"
                        );
                        break;
                    }
                    Poll::Ready(None) => break,
                    Poll::Pending => {}
                }
            }
        }

        let total = polls.load(Ordering::Relaxed);
        // O(N): ~N (initial) + ~N (one targeted re-poll each) = ~2N. The old
        // flat-Vec design would be ~N(N+1)/2 here.
        assert!(
            total <= 4 * N,
            "expected O(N) leaf polls (~2N) for N={N}, got {total}; a \
             quadratic re-poll would be ~{}",
            N * (N + 1) / 2
        );
    }

    /// `consume_buffers` must poll its async resources concurrently, not one
    /// after another. A future that parks once on first poll lets the test
    /// observe how many resources are in flight simultaneously: concurrent
    /// resolution peaks at N, sequential awaiting peaks at 1. The returned
    /// order must still match the registration order.
    #[test]
    fn consume_buffers_polls_async_resources_concurrently() {
        // On its first poll a probe marks itself in-flight (and self-wakes so a
        // second poll happens); on its second poll it leaves in-flight and
        // resolves. The peak simultaneous in-flight count distinguishes
        // concurrent (`join_all`) from sequential (`for fut in .. { fut.await }`)
        // resolution.
        struct Probe {
            in_flight: Arc<AtomicUsize>,
            peak: Arc<AtomicUsize>,
            started: bool,
        }
        impl Future for Probe {
            type Output = String;
            fn poll(
                self: Pin<&mut Self>,
                cx: &mut Context<'_>,
            ) -> Poll<String> {
                let this = self.get_mut();
                if this.started {
                    this.in_flight.fetch_sub(1, Ordering::Relaxed);
                    Poll::Ready(String::from("\"x\""))
                } else {
                    this.started = true;
                    let now =
                        this.in_flight.fetch_add(1, Ordering::Relaxed) + 1;
                    this.peak.fetch_max(now, Ordering::Relaxed);
                    cx.waker().wake_by_ref();
                    Poll::Pending
                }
            }
        }

        const N: usize = 8;
        let in_flight = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));
        let ctx = SsrSharedContext::new();
        for i in 0..N {
            ctx.write_async(
                SerializedDataId(i),
                Box::pin(Probe {
                    in_flight: Arc::clone(&in_flight),
                    peak: Arc::clone(&peak),
                    started: false,
                }),
            );
        }

        let data = block_on(ctx.consume_buffers());

        assert_eq!(data.len(), N);
        for (i, (id, _)) in data.iter().enumerate() {
            assert_eq!(
                *id,
                SerializedDataId(i),
                "consume_buffers must preserve registration order"
            );
        }
        assert_eq!(
            peak.load(Ordering::Relaxed),
            N,
            "all async resources must be in flight simultaneously; sequential \
             awaiting would peak at 1"
        );
    }
}
