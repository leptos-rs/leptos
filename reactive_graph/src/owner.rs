//! The reactive ownership model, which manages effect cancelation, cleanups, and arena allocation.

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

mod arc_stored_value;
mod arena;
mod arena_item;
mod context;
mod storage;
mod stored_value;
use self::arena::Arena;
pub use arc_stored_value::ArcStoredValue;
#[cfg(feature = "sandboxed-arenas")]
pub use arena::sandboxed::Sandboxed;
#[cfg(feature = "sandboxed-arenas")]
use arena::ArenaMap;
use arena::NodeId;
pub use arena_item::*;
pub use context::*;
pub use storage::*;
#[allow(deprecated)] // allow exporting deprecated fn
pub use stored_value::{store_value, FromLocal, StoredValue};

/// A reactive owner, which manages
/// 1) the cancelation of [`Effect`](crate::effect::Effect)s,
/// 2) providing and accessing environment data via [`provide_context`] and [`use_context`],
/// 3) running cleanup functions defined via [`Owner::on_cleanup`], and
/// 4) an arena storage system to provide `Copy` handles via [`ArenaItem`], which is what allows
///    types like [`RwSignal`](crate::signal::RwSignal), [`Memo`](crate::computed::Memo), and so on to be `Copy`.
///
/// Every effect and computed reactive value has an associated `Owner`. While it is running, this
/// is marked as the current `Owner`. Whenever it re-runs, this `Owner` is cleared by calling
/// [`Owner::with_cleanup`]. This runs cleanup functions, cancels any [`Effect`](crate::effect::Effect)s created during the
/// last run, drops signals stored in the arena, and so on, because those effects and signals will
/// be re-created as needed during the next run.
///
/// When the owner is ultimately dropped, it will clean up its owned resources in the same way.
///
/// The "current owner" is set on the thread-local basis: whenever one of these reactive nodes is
/// running, it will set the current owner on its thread with [`Owner::with`] or [`Owner::set`],
/// allowing other reactive nodes implicitly to access the fact that it is currently the owner.
///
/// For a longer discussion of the ownership model, [see
/// here](https://book.leptos.dev/appendix_life_cycle.html).
#[derive(Debug, Clone, Default)]
#[must_use]
pub struct Owner {
    pub(crate) inner: Arc<RwLock<OwnerInner>>,
    #[cfg(feature = "hydration")]
    pub(crate) shared_context: Option<Arc<dyn SharedContext + Send + Sync>>,
}

impl PartialEq for Owner {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}

thread_local! {
    static OWNER: RefCell<Option<Owner>> = Default::default();
}

impl Owner {
    /// Returns a unique identifier for this owner, which can be used to identify it for debugging
    /// purposes.
    ///
    /// Intended for debugging only; this is not guaranteed to be stable between runs.
    pub fn debug_id(&self) -> usize {
        Arc::as_ptr(&self.inner) as usize
    }

    /// Returns the list of parents, grandparents, and ancestors, with values corresponding to
    /// [`Owner::debug_id`] for each.
    ///
    /// Intended for debugging only; this is not guaranteed to be stable between runs.
    pub fn ancestry(&self) -> Vec<usize> {
        let mut ancestors = Vec::new();
        let mut curr_parent = self
            .inner
            .read()
            .or_poisoned()
            .parent
            .as_ref()
            .and_then(|n| n.upgrade());
        while let Some(parent) = curr_parent {
            ancestors.push(Arc::as_ptr(&parent) as usize);
            curr_parent = parent
                .read()
                .or_poisoned()
                .parent
                .as_ref()
                .and_then(|n| n.upgrade());
        }
        ancestors
    }

    /// Creates a new `Owner` and registers it as a child of the current `Owner`, if there is one.
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
        let this = Self {
            inner: Arc::new(RwLock::new(OwnerInner {
                parent: parent.clone(),
                nodes: Default::default(),
                contexts: Default::default(),
                cleanups: Default::default(),
                children: Default::default(),
                #[cfg(feature = "sandboxed-arenas")]
                arena: parent
                    .as_ref()
                    .and_then(|parent| parent.upgrade())
                    .map(|parent| parent.read().or_poisoned().arena.clone())
                    .unwrap_or_default(),
            })),
            #[cfg(feature = "hydration")]
            shared_context,
        };
        if let Some(parent) = parent.and_then(|n| n.upgrade()) {
            parent
                .write()
                .or_poisoned()
                .children
                .push(Arc::downgrade(&this.inner));
        }
        this
    }

    /// Creates a new "root" context with the given [`SharedContext`], which allows sharing data
    /// between the server and client.
    ///
    /// Only one `SharedContext` needs to be created per request, and will be automatically shared
    /// by any other `Owner`s created under this one.
    #[cfg(feature = "hydration")]
    #[track_caller]
    pub fn new_root(
        shared_context: Option<Arc<dyn SharedContext + Send + Sync>>,
    ) -> Self {
        let this = Self {
            inner: Arc::new(RwLock::new(OwnerInner {
                parent: None,
                nodes: Default::default(),
                contexts: Default::default(),
                cleanups: Default::default(),
                children: Default::default(),
                #[cfg(feature = "sandboxed-arenas")]
                arena: Default::default(),
            })),
            #[cfg(feature = "hydration")]
            shared_context,
        };
        this.set();
        this
    }

    /// Creates a new `Owner` that is the child of the current `Owner`, if any.
    pub fn child(&self) -> Self {
        let parent = Some(Arc::downgrade(&self.inner));
        #[cfg(feature = "sandboxed-arenas")]
        let arena = self.inner.read().or_poisoned().arena.clone();
        let child = Self {
            inner: Arc::new(RwLock::new(OwnerInner {
                parent,
                nodes: Default::default(),
                contexts: Default::default(),
                cleanups: Default::default(),
                children: Default::default(),
                #[cfg(feature = "sandboxed-arenas")]
                arena,
            })),
            #[cfg(feature = "hydration")]
            shared_context: self.shared_context.clone(),
        };
        self.inner
            .write()
            .or_poisoned()
            .children
            .push(Arc::downgrade(&child.inner));
        child
    }

    /// Sets this as the current `Owner`.
    pub fn set(&self) {
        OWNER.with_borrow_mut(|owner| *owner = Some(self.clone()));
        #[cfg(feature = "sandboxed-arenas")]
        Arena::set(&self.inner.read().or_poisoned().arena);
    }

    /// Runs the given function with this as the current `Owner`.
    pub fn with<T>(&self, fun: impl FnOnce() -> T) -> T {
        let prev = {
            OWNER.with(|o| {
                mem::replace(&mut *o.borrow_mut(), Some(self.clone()))
            })
        };
        #[cfg(feature = "sandboxed-arenas")]
        Arena::set(&self.inner.read().or_poisoned().arena);
        let val = fun();
        OWNER.with(|o| {
            *o.borrow_mut() = prev;
        });
        val
    }

    /// Cleans up this owner, the given function with this as the current `Owner`.
    pub fn with_cleanup<T>(&self, fun: impl FnOnce() -> T) -> T {
        self.cleanup();
        self.with(fun)
    }

    /// Cleans up this owner in the following order:
    /// 1) Runs `cleanup` on all children,
    /// 2) Runs all cleanup functions registered with [`Owner::on_cleanup`],
    /// 3) Drops the values of any arena-allocated [`ArenaItem`]s.
    pub fn cleanup(&self) {
        self.inner.cleanup();
    }

    /// Registers a function to be run the next time the current owner is cleaned up.
    ///
    /// Because the ownership model is associated with reactive nodes, each "decision point" in an
    /// application tends to have a separate `Owner`: as a result, these cleanup functions often
    /// fill the same need as an "on unmount" function in other UI approaches, etc.
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

    /// Returns the current `Owner`, if any.
    pub fn current() -> Option<Owner> {
        OWNER.with(|o| o.borrow().clone())
    }

    /// Returns the [`SharedContext`] associated with this owner, if any.
    #[cfg(feature = "hydration")]
    pub fn shared_context(
        &self,
    ) -> Option<Arc<dyn SharedContext + Send + Sync>> {
        self.shared_context.clone()
    }

    /// Removes this from its state as the thread-local owner and drops it.
    pub fn unset(self) {
        OWNER.with_borrow_mut(|owner| {
            if owner.as_ref() == Some(&self) {
                mem::take(owner);
            }
        })
    }

    /// Returns the current [`SharedContext`], if any.
    #[cfg(feature = "hydration")]
    pub fn current_shared_context(
    ) -> Option<Arc<dyn SharedContext + Send + Sync>> {
        OWNER.with(|o| {
            o.borrow()
                .as_ref()
                .and_then(|current| current.shared_context.clone())
        })
    }

    /// Runs the given function, after indicating that the current [`SharedContext`] should be
    /// prepared to handle any data created in the function.
    #[cfg(feature = "hydration")]
    pub fn with_hydration<T>(fun: impl FnOnce() -> T + 'static) -> T {
        fn inner<T>(fun: Box<dyn FnOnce() -> T>) -> T {
            let sc = OWNER.with_borrow(|o| {
                o.as_ref()
                    .and_then(|current| current.shared_context.clone())
            });
            match sc {
                None => fun(),
                Some(sc) => {
                    let prev = sc.get_is_hydrating();
                    sc.set_is_hydrating(true);
                    let value = fun();
                    sc.set_is_hydrating(prev);
                    value
                }
            }
        }

        inner(Box::new(fun))
    }

    /// Runs the given function, after indicating that the current [`SharedContext`] should /// not handle data created in this function.
    #[cfg(feature = "hydration")]
    pub fn with_no_hydration<T>(fun: impl FnOnce() -> T + 'static) -> T {
        fn inner<T>(fun: Box<dyn FnOnce() -> T>) -> T {
            let sc = OWNER.with_borrow(|o| {
                o.as_ref()
                    .and_then(|current| current.shared_context.clone())
            });
            match sc {
                None => fun(),
                Some(sc) => {
                    let prev = sc.get_is_hydrating();
                    sc.set_is_hydrating(false);
                    let value = fun();
                    sc.set_is_hydrating(prev);
                    value
                }
            }
        }

        inner(Box::new(fun))
    }
}

/// Registers a function to be run the next time the current owner is cleaned up.
///
/// Because the ownership model is associated with reactive nodes, each "decision point" in an
/// application tends to have a separate `Owner`: as a result, these cleanup functions often
/// fill the same need as an "on unmount" function in other UI approaches, etc.
///
/// This is an alias for [`Owner::on_cleanup`].
pub fn on_cleanup(fun: impl FnOnce() + Send + Sync + 'static) {
    Owner::on_cleanup(fun)
}

#[derive(Default)]
pub(crate) struct OwnerInner {
    pub parent: Option<Weak<RwLock<OwnerInner>>>,
    nodes: Vec<NodeId>,
    pub contexts: FxHashMap<TypeId, Box<dyn Any + Send + Sync>>,
    pub cleanups: Vec<Box<dyn FnOnce() + Send + Sync>>,
    pub children: Vec<Weak<RwLock<OwnerInner>>>,
    #[cfg(feature = "sandboxed-arenas")]
    arena: Arc<RwLock<ArenaMap>>,
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
        for child in std::mem::take(&mut self.children) {
            if let Some(child) = child.upgrade() {
                child.cleanup();
            }
        }

        for cleanup in mem::take(&mut self.cleanups) {
            cleanup();
        }

        let nodes = mem::take(&mut self.nodes);
        if !nodes.is_empty() {
            #[cfg(not(feature = "sandboxed-arenas"))]
            Arena::with_mut(|arena| {
                for node in nodes {
                    _ = arena.remove(node);
                }
            });
            #[cfg(feature = "sandboxed-arenas")]
            {
                let mut arena = self.arena.write().or_poisoned();
                for node in nodes {
                    _ = arena.remove(node);
                }
            }
        }
    }
}

trait Cleanup {
    fn cleanup(&self);
}

impl Cleanup for RwLock<OwnerInner> {
    fn cleanup(&self) {
        let (cleanups, nodes, children) = {
            let mut lock = self.write().or_poisoned();
            (
                mem::take(&mut lock.cleanups),
                mem::take(&mut lock.nodes),
                mem::take(&mut lock.children),
            )
        };
        for child in children {
            if let Some(child) = child.upgrade() {
                child.cleanup();
            }
        }
        for cleanup in cleanups {
            cleanup();
        }

        if !nodes.is_empty() {
            #[cfg(not(feature = "sandboxed-arenas"))]
            Arena::with_mut(|arena| {
                for node in nodes {
                    _ = arena.remove(node);
                }
            });
            #[cfg(feature = "sandboxed-arenas")]
            {
                let arena = self.read().or_poisoned().arena.clone();
                let mut arena = arena.write().or_poisoned();
                for node in nodes {
                    _ = arena.remove(node);
                }
            }
        }
    }
}
