use std::{
    cell::{Cell, RefCell},
    fmt::Debug,
    rc::Rc,
};

use slotmap::SlotMap;

use crate::{
    ObserverLink, ResourceId, ResourceState, Scope, ScopeDisposer, ScopeId, ScopeState, SignalId,
    SignalState, TransitionState,
};

pub struct System {
    observer: RefCell<Option<ObserverLink>>,
    tracking: Cell<bool>,
    batch: RefCell<Vec<Box<dyn FnOnce()>>>,
    pub(crate) scopes: RefCell<SlotMap<ScopeId, Rc<ScopeState>>>,
}

impl System {
    pub fn new() -> Self {
        Self {
            observer: Default::default(),
            tracking: Default::default(),
            batch: Default::default(),
            scopes: Default::default(),
        }
    }

    pub fn create_scope(
        &'static self,
        f: impl FnOnce(Scope),
        parent: Option<Scope>,
    ) -> ScopeDisposer {
        let id = {
            self.scopes
                .borrow_mut()
                .insert(Rc::new(ScopeState::new(parent)))
        };
        let scope = Scope { system: self, id };
        f(scope);

        ScopeDisposer(Box::new(move || scope.dispose()))
    }

    pub(crate) fn wrap<T>(
        &self,
        f: impl FnOnce() -> T,
        observer: Option<ObserverLink>,
        tracking: bool,
    ) -> T {
        let prev_observer = self.observer.replace(observer);
        let prev_tracking = self.tracking.get();

        self.tracking.set(tracking);

        let value = f();

        *self.observer.borrow_mut() = prev_observer;
        self.tracking.set(prev_tracking);

        value
    }

    pub(crate) fn untrack<T>(&self, f: impl FnOnce() -> T) -> T {
        self.wrap(f, self.observer(), false)
    }

    pub(crate) fn tracking(&self) -> bool {
        self.tracking.get()
    }

    pub(crate) fn observer(&self) -> Option<ObserverLink> {
        self.observer.borrow().clone()
    }

    pub(crate) fn batching(&self) -> bool {
        !self.batch.borrow().is_empty()
    }

    pub(crate) fn add_to_batch(&self, deferred_fn: impl FnOnce()) {
        let deferred: Box<dyn FnOnce()> = Box::new(deferred_fn);
        // TODO safety
        let deferred: Box<dyn FnOnce() + 'static> = unsafe { std::mem::transmute(deferred) };

        self.batch.borrow_mut().push(deferred);
    }

    pub(crate) fn scope<T>(&self, id: ScopeId, f: impl FnOnce(&ScopeState) -> T) -> T {
        let scope = { self.scopes.borrow().get(id).cloned() };
        if let Some(scope) = scope {
            (f)(&scope)
        } else {
            log::error!(
                "couldn't locate {id:?} in scopes {:#?}",
                self.scopes.borrow()
            );
            panic!("couldn't locate {id:?}");
        }
    }

    pub(crate) fn remove_scope(&self, scope: &ScopeId) {
        self.scopes.borrow_mut().remove(*scope);
    }

    pub(crate) fn signal<T, U>(
        &self,
        id: (ScopeId, SignalId),
        f: impl FnOnce(Rc<SignalState<T>>) -> U,
    ) -> U
    where
        T: 'static,
    {
        self.scope(id.0, |scope| {
            if let Ok(n) = scope.arena[id.1 .0].clone().downcast::<SignalState<T>>() {
                (f)(n)
            } else {
                panic!(
                    "couldn't convert {id:?} to SignalState<{}>",
                    std::any::type_name::<T>()
                );
            }
        })
    }

    pub(crate) fn resource<S, T, U>(
        &self,
        id: (ScopeId, ResourceId),
        f: impl FnOnce(&ResourceState<S, T>) -> U,
    ) -> U
    where
        S: Debug + Clone + 'static,
        T: Debug + Clone + 'static,
    {
        self.scope(id.0, |scope| {
            if let Ok(n) = scope.arena[id.1 .0]
                .clone()
                .downcast::<ResourceState<S, T>>()
            {
                (f)(&n)
            } else {
                panic!(
                    "couldn't convert {id:?} to SignalState<{}>",
                    std::any::type_name::<T>()
                );
            }
        })
    }

    pub(crate) fn running_transition(&self) -> Option<TransitionState> {
        None
        // TODO transition
    }

    pub(crate) fn transition(&self) -> Option<TransitionState> {
        None
        // TODO transition
    }
}

impl Default for System {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for System {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("System")
            .field("observer", &self.observer)
            .field("tracking", &self.tracking)
            .field("batch", &self.batch.borrow().len())
            .field("scopes", &self.scopes)
            .finish()
    }
}

impl PartialEq for System {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Eq for System {}

impl std::hash::Hash for System {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::ptr::hash(&self, state);
    }
}
