//! Types that handle asynchronous data loading via `<Suspense/>`.

use crate::{
    batch, create_isomorphic_effect, create_memo, create_rw_signal,
    create_signal, oco::Oco, queue_microtask, store_value, Memo, ReadSignal,
    ResourceId, RwSignal, SignalSet, SignalUpdate, SignalWith, StoredValue,
    WriteSignal,
};
use futures::Future;
use rustc_hash::FxHashSet;
use std::{cell::RefCell, collections::VecDeque, pin::Pin, rc::Rc};

/// Tracks [`Resource`](crate::Resource)s that are read under a suspense context,
/// i.e., within a [`Suspense`](https://docs.rs/leptos_core/latest/leptos_core/fn.Suspense.html) component.
#[derive(Copy, Clone, Debug)]
pub struct SuspenseContext {
    /// The number of resources that are currently pending.
    pub pending_resources: ReadSignal<usize>,
    set_pending_resources: WriteSignal<usize>,
    // NOTE: For correctness reasons, we really need to move to this
    // However, for API stability reasons, I need to keep the counter-incrementing version too
    pub(crate) pending: RwSignal<FxHashSet<ResourceId>>,
    pub(crate) pending_serializable_resources: RwSignal<FxHashSet<ResourceId>>,
    pub(crate) pending_serializable_resources_count: RwSignal<usize>,
    pub(crate) local_status: StoredValue<Option<LocalStatus>>,
    pub(crate) should_block: StoredValue<bool>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum LocalStatus {
    LocalOnly,
    Mixed,
    SerializableOnly,
}

/// A single, global suspense context that will be checked when resources
/// are read. This won’t be “blocked” by lower suspense components. This is
/// useful for e.g., holding route transitions.
#[derive(Clone, Debug)]
pub struct GlobalSuspenseContext(Rc<RefCell<SuspenseContext>>);

impl GlobalSuspenseContext {
    /// Creates an empty global suspense context.
    pub fn new() -> Self {
        Self(Rc::new(RefCell::new(SuspenseContext::new())))
    }

    /// Runs a function with a reference to the underlying suspense context.
    pub fn with_inner<T>(&self, f: impl FnOnce(&SuspenseContext) -> T) -> T {
        f(&self.0.borrow())
    }

    /// Runs a function with a reference to the underlying suspense context.
    pub fn reset(&self) {
        let mut inner = self.0.borrow_mut();
        _ = std::mem::replace(&mut *inner, SuspenseContext::new());
    }
}

impl Default for GlobalSuspenseContext {
    fn default() -> Self {
        Self::new()
    }
}

impl SuspenseContext {
    /// Whether the suspense contains local resources at this moment,
    /// and therefore can't be serialized
    pub fn has_local_only(&self) -> bool {
        matches!(self.local_status.get_value(), Some(LocalStatus::LocalOnly))
    }

    /// Whether the suspense contains any local resources at this moment.
    pub fn has_any_local(&self) -> bool {
        matches!(
            self.local_status.get_value(),
            Some(LocalStatus::LocalOnly) | Some(LocalStatus::Mixed)
        )
    }

    /// Whether any blocking resources are read under this suspense context,
    /// meaning the HTML stream should not begin until it has resolved.
    pub fn should_block(&self) -> bool {
        self.should_block.get_value()
    }

    /// Returns a `Future` that resolves when this suspense is resolved.
    pub fn to_future(&self) -> impl Future<Output = ()> {
        use futures::StreamExt;

        let pending = self.pending;
        let (tx, mut rx) = futures::channel::mpsc::channel(1);
        let tx = RefCell::new(tx);
        queue_microtask(move || {
            create_isomorphic_effect(move |_| {
                if pending.with(|p| p.is_empty()) {
                    _ = tx.borrow_mut().try_send(());
                }
            });
        });
        async move {
            rx.next().await;
        }
    }

    /// Reactively checks whether there are no pending resources in the suspense.
    pub fn none_pending(&self) -> bool {
        self.pending.with(|p| p.is_empty())
    }
}

impl std::hash::Hash for SuspenseContext {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.pending.id.hash(state);
    }
}

impl PartialEq for SuspenseContext {
    fn eq(&self, other: &Self) -> bool {
        self.pending.id == other.pending.id
    }
}

impl Eq for SuspenseContext {}

impl SuspenseContext {
    /// Creates an empty suspense context.
    pub fn new() -> Self {
        let (pending_resources, set_pending_resources) = create_signal(0); // can be removed when possible
        let pending_serializable_resources =
            create_rw_signal(Default::default());
        let pending_serializable_resources_count = create_rw_signal(0); // can be removed when possible
        let local_status = store_value(None);
        let should_block = store_value(false);
        let pending = create_rw_signal(Default::default());
        Self {
            pending,
            pending_resources,
            set_pending_resources,
            pending_serializable_resources,
            pending_serializable_resources_count,
            local_status,
            should_block,
        }
    }

    /// Notifies the suspense context that a new resource is now pending.
    pub fn increment(&self, serializable: bool) {
        let setter = self.set_pending_resources;
        let serializable_resources = self.pending_serializable_resources_count;
        let local_status = self.local_status;
        setter.update(|n| *n += 1);
        if serializable {
            serializable_resources.update(|n| *n += 1);
            local_status.update_value(|status| {
                *status = Some(match status {
                    None => LocalStatus::SerializableOnly,
                    Some(LocalStatus::LocalOnly) => LocalStatus::LocalOnly,
                    Some(LocalStatus::Mixed) => LocalStatus::Mixed,
                    Some(LocalStatus::SerializableOnly) => {
                        LocalStatus::SerializableOnly
                    }
                });
            });
        } else {
            local_status.update_value(|status| {
                *status = Some(match status {
                    None => LocalStatus::LocalOnly,
                    Some(LocalStatus::LocalOnly) => LocalStatus::LocalOnly,
                    Some(LocalStatus::Mixed) => LocalStatus::Mixed,
                    Some(LocalStatus::SerializableOnly) => LocalStatus::Mixed,
                });
            });
        }
    }

    /// Notifies the suspense context that a resource has resolved.
    pub fn decrement(&self, serializable: bool) {
        let setter = self.set_pending_resources;
        let serializable_resources = self.pending_serializable_resources_count;
        setter.update(|n| {
            if *n > 0 {
                *n -= 1
            }
        });
        if serializable {
            serializable_resources.update(|n| {
                if *n > 0 {
                    *n -= 1;
                }
            });
        }
    }

    /// Notifies the suspense context that a new resource is now pending.
    pub(crate) fn increment_for_resource(
        &self,
        serializable: bool,
        resource: ResourceId,
    ) {
        let pending = self.pending;
        let serializable_resources = self.pending_serializable_resources;
        let local_status = self.local_status;
        batch(move || {
            pending.update(|n| {
                n.insert(resource);
            });
            if serializable {
                serializable_resources.update(|n| {
                    n.insert(resource);
                });
                local_status.update_value(|status| {
                    *status = Some(match status {
                        None => LocalStatus::SerializableOnly,
                        Some(LocalStatus::LocalOnly) => LocalStatus::LocalOnly,
                        Some(LocalStatus::Mixed) => LocalStatus::Mixed,
                        Some(LocalStatus::SerializableOnly) => {
                            LocalStatus::SerializableOnly
                        }
                    });
                });
            } else {
                local_status.update_value(|status| {
                    *status = Some(match status {
                        None => LocalStatus::LocalOnly,
                        Some(LocalStatus::LocalOnly) => LocalStatus::LocalOnly,
                        Some(LocalStatus::Mixed) => LocalStatus::Mixed,
                        Some(LocalStatus::SerializableOnly) => {
                            LocalStatus::Mixed
                        }
                    });
                });
            }
        });
    }

    /// Notifies the suspense context that a resource has resolved.
    pub fn decrement_for_resource(
        &self,
        serializable: bool,
        resource: ResourceId,
    ) {
        let setter = self.pending;
        let serializable_resources = self.pending_serializable_resources;
        batch(move || {
            setter.update(|n| {
                n.remove(&resource);
            });
            if serializable {
                serializable_resources.update(|n| {
                    n.remove(&resource);
                });
            }
        });
    }

    /// Resets the counter of pending resources.
    pub fn clear(&self) {
        batch(move || {
            self.set_pending_resources.set(0);
            self.pending.update(|p| p.clear());
            self.pending_serializable_resources.update(|p| p.clear());
        });
    }

    /// Tests whether all of the pending resources have resolved.
    pub fn ready(&self) -> Memo<bool> {
        let pending = self.pending;
        create_memo(move |_| {
            pending.try_with(|n| n.is_empty()).unwrap_or(false)
        })
    }
}

impl Default for SuspenseContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Represents a chunk in a stream of HTML.
pub enum StreamChunk {
    /// A chunk of synchronous HTML.
    Sync(Oco<'static, str>),
    /// A future that resolves to be a list of additional chunks.
    Async {
        /// The HTML chunks this contains.
        chunks: Pin<Box<dyn Future<Output = VecDeque<StreamChunk>>>>,
        /// Whether this should block the stream.
        should_block: bool,
    },
}

impl core::fmt::Debug for StreamChunk {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            StreamChunk::Sync(data) => write!(f, "StreamChunk::Sync({data:?})"),
            StreamChunk::Async { .. } => write!(f, "StreamChunk::Async(_)"),
        }
    }
}
