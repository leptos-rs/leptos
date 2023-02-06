#![forbid(unsafe_code)]
use crate::{create_signal, queue_microtask, ReadSignal, Scope, WriteSignal};

/// Tracks [Resource](crate::Resource)s that are read under a suspense context,
/// i.e., within a [`Suspense`](https://docs.rs/leptos_core/latest/leptos_core/fn.Suspense.html) component.
#[derive(Copy, Clone, Debug)]
pub struct SuspenseContext {
    /// The number of resources that are currently pending.
    pub pending_resources: ReadSignal<usize>,
    set_pending_resources: WriteSignal<usize>,
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
        Self {
            pending_resources,
            set_pending_resources,
        }
    }

    /// Notifies the suspense context that a new resource is now pending.
    pub fn increment(&self) {
        let setter = self.set_pending_resources;
        queue_microtask(move || {
            setter.update(|n| *n += 1);
        });
    }

    /// Notifies the suspense context that a resource has resolved.
    pub fn decrement(&self) {
        let setter = self.set_pending_resources;
        queue_microtask(move || {
            setter.update(|n| {
                if *n > 0 {
                    *n -= 1
                }
            });
        });
    }

    /// Tests whether all of the pending resources have resolved.
    pub fn ready(&self) -> bool {
        self.pending_resources
            .try_with(|n| *n == 0)
            .unwrap_or(false)
    }
}
