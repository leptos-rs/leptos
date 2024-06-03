#[cfg(feature = "hydration")]
use hydration_context::SharedContext;
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
pub use arena::{StoredData, StoredValue};
pub use context::*;

#[derive(Debug, Clone, Default)]
#[must_use]
pub struct Owner {
    pub(crate) inner: Arc<RwLock<OwnerInner>>,
    #[cfg(feature = "hydration")]
    pub(crate) shared_context: Option<Arc<dyn SharedContext + Send + Sync>>,
}

thread_local! {
    static OWNER: RefCell<Option<Owner>> = Default::default();
}

impl Owner {
    pub fn debug_id(&self) -> usize {
        Arc::as_ptr(&self.inner) as usize
    }
    pub fn new() -> Self {
        #[cfg(not(feature = "hydration"))]
        let parent = OWNER
            .with(|o| o.borrow().as_ref().map(|o| Arc::downgrade(&o.inner)));
        #[cfg(feature = "hydration")]
        let (parent, shared_context) = OWNER
            .with(|o| {
                o.borrow().as_ref().map(|o| {
                    (Some(Arc::downgrade(&o.inner)), o.shared_context.clone())
                })
            })
            .unwrap_or((None, None));
        Self {
            inner: Arc::new(RwLock::new(OwnerInner {
                parent,
                nodes: Default::default(),
                contexts: Default::default(),
                cleanups: Default::default(),
            })),
            #[cfg(feature = "hydration")]
            shared_context,
        }
    }

    #[cfg(feature = "hydration")]
    pub fn new_root(
        shared_context: Arc<dyn SharedContext + Send + Sync>,
    ) -> Self {
        Self {
            inner: Arc::new(RwLock::new(OwnerInner {
                parent: None,
                nodes: Default::default(),
                contexts: Default::default(),
                cleanups: Default::default(),
            })),
            #[cfg(feature = "hydration")]
            shared_context: Some(shared_context),
        }
    }

    pub fn child(&self) -> Self {
        let parent = Some(Arc::downgrade(&self.inner));
        Self {
            inner: Arc::new(RwLock::new(OwnerInner {
                parent,
                nodes: Default::default(),
                contexts: Default::default(),
                cleanups: Default::default(),
            })),
            #[cfg(feature = "hydration")]
            shared_context: self.shared_context.clone(),
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

    #[cfg(feature = "hydration")]
    pub fn current_shared_context(
    ) -> Option<Arc<dyn SharedContext + Send + Sync>> {
        OWNER.with(|o| {
            o.borrow()
                .as_ref()
                .and_then(|current| current.shared_context.clone())
        })
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

        for node in mem::take(&mut self.nodes) {
            _ = arena::map().write().or_poisoned().remove(node);
        }
    }
}
