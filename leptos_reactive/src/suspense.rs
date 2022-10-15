#[cfg(feature = "ssr")]
use std::{cell::RefCell, future::Future, pin::Pin, rc::Rc};

use crate::{create_signal, spawn::queue_microtask, ReadSignal, Scope, WriteSignal};

#[derive(Copy, Clone, Debug)]
pub struct SuspenseContext {
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
    pub fn new(cx: Scope) -> Self {
        let (pending_resources, set_pending_resources) = create_signal(cx, 0);
        Self {
            pending_resources,
            set_pending_resources,
        }
    }

    pub fn increment(&self) {
        let setter = self.set_pending_resources;
        queue_microtask(move || setter.update(|n| *n += 1));
    }

    pub fn decrement(&self) {
        let setter = self.set_pending_resources;
        queue_microtask(move || {
            setter.update(|n| {
                if *n > 0 {
                    *n -= 1
                }
            })
        });
    }

    pub fn ready(&self) -> bool {
        self.pending_resources.get() == 0
    }
}
