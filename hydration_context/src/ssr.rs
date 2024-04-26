use super::{SerializedDataId, SharedContext};
use crate::{PinnedFuture, PinnedStream};
use any_error::{Error, ErrorId};
use futures::{
    stream::{self, FuturesUnordered},
    StreamExt,
};
use or_poisoned::OrPoisoned;
use std::{
    collections::{HashMap, HashSet},
    fmt::{Debug, Write},
    mem,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, RwLock,
    },
};

#[derive(Default)]
/// The shared context that should be used on the server side.
pub struct SsrSharedContext {
    id: AtomicUsize,
    is_hydrating: AtomicBool,
    sync_buf: RwLock<Vec<ResolvedData>>,
    async_buf: RwLock<Vec<(SerializedDataId, PinnedFuture<String>)>>,
    errors: Arc<RwLock<Vec<(SerializedDataId, ErrorId, Error)>>>,
    sealed_error_boundaries: Arc<RwLock<HashSet<SerializedDataId>>>,
}

impl SsrSharedContext {
    /// Creates a new shared context for rendering HTML on the server.
    pub fn new() -> Self {
        Self {
            is_hydrating: AtomicBool::new(true),
            ..Default::default()
        }
    }

    /// Creates a new shared context for rendering HTML on the server in "islands" mode.
    ///
    /// This defaults to a mode in which the app is not hydrated, but allows you to opt into
    /// hydration for certain portions using [`SharedContext::set_is_hydrating`].
    pub fn new_islands() -> Self {
        Self {
            is_hydrating: AtomicBool::new(false),
            ..Default::default()
        }
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

    fn next_id(&self) -> SerializedDataId {
        let id = self.id.fetch_add(1, Ordering::Relaxed);
        SerializedDataId(id)
    }

    fn write_async(&self, id: SerializedDataId, fut: PinnedFuture<String>) {
        self.async_buf.write().or_poisoned().push((id, fut))
    }

    fn pending_data(&self) -> Option<PinnedStream<String>> {
        let sync_data = mem::take(&mut *self.sync_buf.write().or_poisoned());
        let async_data = mem::take(&mut *self.async_buf.write().or_poisoned());

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
            write!(
                initial_chunk,
                "[{}, {}, {:?}],",
                error.0 .0, error.1, error.2
            );
        }
        initial_chunk.push_str("];");

        // pending async resources
        initial_chunk.push_str("__PENDING_RESOURCES=[");
        for (id, _) in &async_data {
            write!(&mut initial_chunk, "{},", id.0).unwrap();
        }
        initial_chunk.push_str("];");

        // resolvers
        initial_chunk.push_str("__RESOURCE_RESOLVERS=[];");

        // 2) async resources as they resolve
        let async_data = async_data
            .into_iter()
            .map(|(id, data)| {
                let errors = Arc::clone(&self.errors);
                let sealed = Arc::clone(&self.sealed_error_boundaries);
                async move {
                    let data = data.await;
                    let data = data.replace('<', "\\u003c");
                    let mut val =
                        format!("__RESOLVED_RESOURCES[{}] = {:?};", id.0, data);
                    let sealed = sealed.read().or_poisoned();
                    for error in mem::take(&mut *errors.write().or_poisoned()) {
                        if !sealed.contains(&error.0) {
                            write!(
                                val,
                                "__SERIALIZED_ERRORS.push([{}, {}, {:?}]);",
                                error.0 .0,
                                error.1,
                                error.2.to_string()
                            );
                        }
                    }
                    val
                }
            })
            .collect::<FuturesUnordered<_>>();

        let stream =
            stream::once(async move { initial_chunk }).chain(async_data);
        Some(Box::pin(stream))
    }

    fn read_data(&self, _id: &SerializedDataId) -> Option<String> {
        None
    }

    fn await_data(&self, _id: &SerializedDataId) -> Option<String> {
        None
    }

    fn get_is_hydrating(&self) -> bool {
        self.is_hydrating.load(Ordering::Relaxed)
    }

    fn set_is_hydrating(&self, is_hydrating: bool) {
        self.is_hydrating.store(is_hydrating, Ordering::Relaxed)
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
