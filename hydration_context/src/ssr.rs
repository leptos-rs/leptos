use super::{SerializedDataId, SharedContext};
use crate::{PinnedFuture, PinnedStream};
use futures::{
    Stream, StreamExt,
    future::join_all,
    stream::{self, once},
};
use or_poisoned::OrPoisoned;
use std::{
    collections::HashSet,
    fmt::{Debug, Write},
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

        let mut all_data = Vec::new();
        for resolved in sync_data {
            all_data.push((resolved.0, resolved.1));
        }
        for (id, fut) in async_data {
            let data = fut.await;
            all_data.push((id, data));
        }
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
            let msg =
                format!("{:?}", error.2.to_string()).replace('<', "\\u003c");
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

struct AsyncDataStream {
    async_buf: AsyncDataBuf,
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
        let mut resolved = String::new();

        // Drain the buffer under a *short* critical section, then drop the
        // lock before polling user futures. Holding the write lock across
        // `fut.poll(cx)` would deadlock if any polled future transitively
        // calls back into `SsrSharedContext::write_async` (which also takes
        // a write lock on the same RwLock).
        let data = mem::take(&mut *self.async_buf.write().or_poisoned());

        let mut still_pending = Vec::with_capacity(data.len());
        for (id, mut fut) in data {
            match fut.as_mut().poll(cx) {
                Poll::Pending => still_pending.push((id, fut)),
                Poll::Ready(data) => {
                    // Any resource id that was not in the initial
                    // `__PENDING_RESOURCES=[...]` snapshot must be
                    // announced before its resolution arrives, so the
                    // client never sees `__RESOLVED_RESOURCES[id] = ...`
                    // for an id that never appeared in
                    // `__PENDING_RESOURCES`.
                    if !self.initial_pending_ids.contains(&id) {
                        _ = write!(
                            resolved,
                            "__PENDING_RESOURCES.push({});",
                            id.0
                        );
                    }
                    let data = data.replace('<', "\\u003c");
                    _ = write!(
                        resolved,
                        "__RESOLVED_RESOURCES[{}] = {:?};",
                        id.0, data
                    );
                }
            }
        }

        // Re-acquire the write lock briefly to push back unfinished futures.
        // Any futures registered *during* the polling above are already in
        // `async_buf`; appending the still-pending ones preserves them.
        let buf_empty = {
            let mut async_buf = self.async_buf.write().or_poisoned();
            async_buf.extend(still_pending);
            async_buf.is_empty()
        };

        // Emit pushes for any incomplete-chunk markers added since the
        // last poll (or set by a future we just polled, e.g. a Suspense
        // boundary that flipped into the fallback state via
        // `SsrSharedContext::set_incomplete_chunk`).
        {
            let lock = self.incomplete.lock().or_poisoned();
            let from = self.incomplete_emitted.load(Ordering::Relaxed);
            for entry in lock.iter().skip(from) {
                _ = write!(resolved, "__INCOMPLETE_CHUNKS.push({});", entry.0);
            }
            self.incomplete_emitted.store(lock.len(), Ordering::Relaxed);
        }

        let sealed = self.sealed_error_boundaries.read().or_poisoned();
        for error in mem::take(&mut *self.errors.write().or_poisoned()) {
            if !sealed.contains(&error.0) {
                // see the initial-chunk path: Debug-format, then single-
                // backslash-escape `<` so the JS parser decodes it back to `<`
                let msg = format!("{:?}", error.2.to_string())
                    .replace('<', "\\u003c");
                _ = write!(
                    resolved,
                    "__SERIALIZED_ERRORS.push([{}, {}, {}]);",
                    error.0.0, error.1, msg
                );
            }
        }

        if buf_empty && resolved.is_empty() {
            return Poll::Ready(None);
        }
        if resolved.is_empty() {
            return Poll::Pending;
        }

        Poll::Ready(Some(resolved))
    }
}

#[derive(Debug)]
struct ResolvedData(SerializedDataId, String);

impl ResolvedData {
    pub fn write_to_buf(&self, buf: &mut String) {
        let ResolvedData(id, ser) = self;
        // escapes < to prevent it being interpreted as another opening HTML tag
        let ser = ser.replace('<', "\\u003c");
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
}
