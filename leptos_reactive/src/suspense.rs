//! Types that handle asynchronous data loading via `<Suspense/>`.

#![forbid(unsafe_code)]
use crate::{
    create_isomorphic_effect, create_rw_signal, create_signal, queue_microtask,
    signal::SignalGet, store_value, ReadSignal, RwSignal, Scope, SignalSet,
    SignalUpdate, StoredValue, WriteSignal,
};
use futures::Future;
use std::{
    borrow::Cow, cell::RefCell, collections::VecDeque, pin::Pin, rc::Rc,
};

/// Tracks [`Resource`](crate::Resource)s that are read under a suspense context,
/// i.e., within a [`Suspense`](https://docs.rs/leptos_core/latest/leptos_core/fn.Suspense.html) component.
#[derive(Copy, Clone, Debug)]
pub struct SuspenseContext {
    /// The number of resources that are currently pending.
    pub pending_resources: ReadSignal<usize>,
    set_pending_resources: WriteSignal<usize>,
    pub(crate) pending_serializable_resources: RwSignal<usize>,
    pub(crate) has_local_only: StoredValue<bool>,
    pub(crate) should_block: StoredValue<bool>,
}

/// A single, global suspense context that will be checked when resources
/// are read. This won’t be “blocked” by lower suspense components. This is
/// useful for e.g., holding route transitions.
#[derive(Clone, Debug)]
pub struct GlobalSuspenseContext(Rc<RefCell<SuspenseContext>>);

impl GlobalSuspenseContext {
    /// Creates an empty global suspense context.
    pub fn new(cx: Scope) -> Self {
        Self(Rc::new(RefCell::new(SuspenseContext::new(cx))))
    }

    /// Runs a function with a reference to the underlying suspense context.
    pub fn with_inner<T>(&self, f: impl FnOnce(&SuspenseContext) -> T) -> T {
        f(&self.0.borrow())
    }

    /// Runs a function with a reference to the underlying suspense context.
    pub fn reset(&self, cx: Scope) {
        let mut inner = self.0.borrow_mut();
        _ = std::mem::replace(&mut *inner, SuspenseContext::new(cx));
    }
}

impl SuspenseContext {
    /// Whether the suspense contains local resources at this moment,
    /// and therefore can't be serialized
    pub fn has_local_only(&self) -> bool {
        self.has_local_only.get_value()
    }

    /// Whether any blocking resources are read under this suspense context,
    /// meaning the HTML stream should not begin until it has resolved.
    pub fn should_block(&self) -> bool {
        self.should_block.get_value()
    }

    /// Returns a `Future` that resolves when this suspense is resolved.
    pub fn to_future(&self, cx: Scope) -> impl Future<Output = ()> {
        use futures::StreamExt;

        let pending_resources = self.pending_resources;
        let (tx, mut rx) = futures::channel::mpsc::channel(1);
        let tx = RefCell::new(tx);
        queue_microtask(move || {
            create_isomorphic_effect(cx, move |_| {
                if pending_resources.get() == 0 {
                    _ = tx.borrow_mut().try_send(());
                }
            })
        });
        async move {
            rx.next().await;
        }
    }
}

impl std::hash::Hash for SuspenseContext {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.pending_resources.id.hash(state);
    }
}

impl PartialEq for SuspenseContext {
    fn eq(&self, other: &Self) -> bool {
        self.pending_resources.id == other.pending_resources.id
    }
}

impl Eq for SuspenseContext {}

impl SuspenseContext {
    /// Creates an empty suspense context.
    pub fn new(cx: Scope) -> Self {
        let (pending_resources, set_pending_resources) = create_signal(cx, 0);
        let pending_serializable_resources = create_rw_signal(cx, 0);
        let has_local_only = store_value(cx, true);
        let should_block = store_value(cx, false);
        Self {
            pending_resources,
            set_pending_resources,
            pending_serializable_resources,
            has_local_only,
            should_block,
        }
    }

    /// Notifies the suspense context that a new resource is now pending.
    pub fn increment(&self, serializable: bool) {
        let setter = self.set_pending_resources;
        let serializable_resources = self.pending_serializable_resources;
        let has_local_only = self.has_local_only;
        queue_microtask(move || {
            setter.update(|n| *n += 1);
            if serializable {
                serializable_resources.update(|n| *n += 1);
                has_local_only.set_value(false);
            }
        });
    }

    /// Notifies the suspense context that a resource has resolved.
    pub fn decrement(&self, serializable: bool) {
        let setter = self.set_pending_resources;
        let serializable_resources = self.pending_serializable_resources;
        queue_microtask(move || {
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
        });
    }

    /// Resets the counter of pending resources.
    pub fn clear(&self) {
        self.set_pending_resources.set(0);
        self.pending_serializable_resources.set(0);
    }

    /// Tests whether all of the pending resources have resolved.
    pub fn ready(&self) -> bool {
        self.pending_resources
            .try_with(|n| *n == 0)
            .unwrap_or(false)
    }
}

/// Represents a chunk in a stream of HTML.
pub enum StreamChunk {
    /// A chunk of synchronous HTML.
    Sync(Cow<'static, str>),
    /// A future that resolves to be a list of additional chunks.
    Async {
        /// The HTML chunks this contains.
        chunks: Pin<Box<dyn Future<Output = VecDeque<StreamChunk>>>>,
        /// Whether this should block the stream.
        should_block: bool,
    },
}

impl std::fmt::Debug for StreamChunk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StreamChunk::Sync(data) => write!(f, "StreamChunk::Sync({data:?})"),
            StreamChunk::Async { .. } => write!(f, "StreamChunk::Async(_)"),
        }
    }
}
