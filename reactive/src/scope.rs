use crate::{Computation, ResourceState, SignalState, System};
use append_only_vec::AppendOnlyVec;
use serde::{Deserialize, Serialize};
use std::{
    any::{Any, TypeId},
    cell::RefCell,
    collections::HashMap,
    fmt::Debug,
    rc::Rc,
};

#[must_use = "Scope will leak memory if the disposer function is never called"]
pub fn create_scope(f: impl FnOnce(Scope) + 'static) -> ScopeDisposer {
    let runtime = Box::leak(Box::new(System::new()));
    runtime.create_scope(f, None)
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Scope {
    pub(crate) system: &'static System,
    pub(crate) id: ScopeId,
}

impl Scope {
    pub fn child_scope(self, f: impl FnOnce(Scope)) -> ScopeDisposer {
        self.system.create_scope(f, Some(self))
    }

    pub fn transition_pending(&self) -> bool {
        // TODO transition self.system.transition().is_some()
        false
    }

    pub fn untrack<T>(&self, f: impl FnOnce() -> T) -> T {
        self.system.untrack(f)
    }
}

// Internals
impl Scope {
    pub(crate) fn push_signal<T>(&self, state: Rc<SignalState<T>>) -> SignalId
    where
        T: Debug + 'static,
    {
        self.system.scope(self.id, |scope| {
            scope.arena.push(state);
            SignalId(scope.arena.len() - 1)
        })
    }

    pub(crate) fn push_computation<T>(&self, state: Rc<Computation<T>>) -> ComputationId
    where
        T: Clone + Debug + 'static,
    {
        self.system.scope(self.id, |scope| {
            scope.arena.push(state);
            ComputationId(scope.arena.len() - 1)
        })
    }

    pub(crate) fn push_resource<S, T>(&self, state: Rc<ResourceState<S, T>>) -> ResourceId
    where
        S: Debug + Clone + 'static,
        T: Debug + Clone + 'static,
    {
        self.system.scope(self.id, |scope| {
            scope.arena.push(state);
            ResourceId(scope.arena.len() - 1)
        })
    }

    pub fn dispose(self) {
        // first, drop child scopes
        self.system.scope(self.id, |scope| {
            for id in scope.children.borrow().iter() {
                self.system.remove_scope(id)
            }
        })
        // removing from the runtime will drop this Scope, and all its Signals/Effects/Memos
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct SignalId(pub(crate) usize);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct ComputationId(pub(crate) usize);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct ResourceId(pub(crate) usize);

pub struct ScopeDisposer(pub(crate) Box<dyn FnOnce()>);

impl ScopeDisposer {
    pub fn dispose(self) {
        (self.0)()
    }
}

impl Debug for ScopeDisposer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ScopeDisposer").finish()
    }
}

slotmap::new_key_type! { pub(crate) struct ScopeId; }

pub(crate) struct ScopeState {
    pub(crate) parent: Option<Scope>,
    pub(crate) contexts: RefCell<HashMap<TypeId, Box<dyn Any>>>,
    pub(crate) children: RefCell<Vec<ScopeId>>,
    pub(crate) arena: AppendOnlyVec<Rc<dyn Any>>,
}

impl Debug for ScopeState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScopeState").finish()
    }
}

impl ScopeState {
    pub(crate) fn new(parent: Option<Scope>) -> Self {
        Self {
            parent,
            contexts: Default::default(),
            children: Default::default(),
            arena: AppendOnlyVec::new(),
        }
    }
}
