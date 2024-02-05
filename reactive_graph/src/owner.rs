use or_poisoned::OrPoisoned;
use rustc_hash::FxHashMap;
use std::{
    any::{Any, TypeId},
    cell::RefCell,
    fmt::Debug,
    mem,
    sync::{Arc, RwLock, Weak},
};

mod arena;
mod context;
use arena::NodeId;
pub use arena::{Stored, StoredData};
pub use context::*;

#[derive(Debug, Clone, Default)]
pub struct Owner {
    pub(crate) inner: Arc<RwLock<OwnerInner>>,
}

thread_local! {
    static OWNER: RefCell<Option<Owner>> = Default::default();
}

impl Owner {
    pub fn new() -> Self {
        let parent = OWNER
            .with(|o| o.borrow().as_ref().map(|o| Arc::downgrade(&o.inner)));
        Self {
            inner: Arc::new(RwLock::new(OwnerInner {
                parent,
                nodes: Default::default(),
                contexts: Default::default(),
                cleanups: Default::default(),
            })),
        }
    }

    pub fn with<T>(&self, fun: impl FnOnce() -> T) -> T {
        let prev = {
            OWNER.with(|o| {
                mem::replace(&mut *o.borrow_mut(), Some(self.clone()))
            })
        };
        let val = fun();
        OWNER.with(|o| {
            *o.borrow_mut() = prev;
        });
        val
    }

    pub fn with_cleanup<T>(&self, fun: impl FnOnce() -> T) -> T {
        let (cleanups, nodes) = {
            let mut lock = self.inner.write().or_poisoned();
            (mem::take(&mut lock.cleanups), mem::take(&mut lock.nodes))
        };
        for cleanup in cleanups {
            cleanup();
        }

        for node in nodes {
            _ = arena::map().write().or_poisoned().remove(node);
        }

        self.with(fun)
    }

    pub fn on_cleanup(fun: impl FnOnce() + Send + Sync + 'static) {
        if let Some(owner) = Owner::current() {
            owner
                .inner
                .write()
                .or_poisoned()
                .cleanups
                .push(Box::new(fun));
        }
    }

    fn register(&self, node: NodeId) {
        self.inner.write().or_poisoned().nodes.push(node);
    }

    pub fn current() -> Option<Owner> {
        OWNER.with(|o| o.borrow().clone())
    }
}

#[derive(Default)]
pub(crate) struct OwnerInner {
    pub parent: Option<Weak<RwLock<OwnerInner>>>,
    nodes: Vec<NodeId>,
    pub contexts: FxHashMap<TypeId, Box<dyn Any + Send + Sync>>,
    pub cleanups: Vec<Box<dyn FnOnce() + Send + Sync>>,
}

impl Debug for OwnerInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OwnerInner")
            .field("parent", &self.parent)
            .field("nodes", &self.nodes)
            .field("contexts", &self.contexts)
            .field("cleanups", &self.cleanups.len())
            .finish()
    }
}

impl Drop for OwnerInner {
    fn drop(&mut self) {
        for cleanup in mem::take(&mut self.cleanups) {
            cleanup();
        }

        #[cfg(feature = "arena")]
        for node in mem::take(&mut self.nodes) {
            _ = arena::map().write().remove(node);
        }
    }
}
