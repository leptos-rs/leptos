use crate::{
    debug_warn, AnyEffect, AnySignal, EffectId, ResourceId, ResourceState, Scope, ScopeDisposer,
    ScopeId, ScopeState, SignalId, SignalState, StreamingResourceId, Subscriber,
};

use slotmap::SlotMap;
use std::cell::RefCell;
use std::fmt::Debug;
use std::rc::Rc;
use thiserror::Error;

use crate::hydration::SharedContext;

#[derive(Default, Debug)]
pub(crate) struct Runtime {
    pub(crate) shared_context: RefCell<Option<SharedContext>>,
    pub(crate) stack: RefCell<Vec<Subscriber>>,
    pub(crate) scopes: RefCell<SlotMap<ScopeId, Rc<ScopeState>>>,
}

#[derive(Error, Debug)]
pub(crate) enum ReactiveSystemErr {
    #[error("tried to access a scope that had already been disposed: {0:?}")]
    ScopeDisposed(ScopeId),
    #[error("tried to access an error that was not available: {0:?} {1:?}")]
    Effect(ScopeId, EffectId),
}

impl Runtime {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn scope<T>(&self, id: ScopeId, f: impl FnOnce(&ScopeState) -> T) -> T {
        let scope = { self.scopes.borrow().get(id).cloned() };
        if let Some(scope) = scope {
            (f)(&scope)
        } else {
            debug_warn!("(scope) couldn't locate {id:?}");
            panic!("couldn't locate {id:?}");
        }
    }

    pub fn try_scope<T>(
        &self,
        id: ScopeId,
        f: impl FnOnce(&ScopeState) -> T,
    ) -> Result<T, ReactiveSystemErr> {
        let scope = { self.scopes.borrow().get(id).cloned() };
        if let Some(scope) = scope {
            Ok((f)(&scope))
        } else {
            debug_warn!("(scope) couldn't locate {id:?}");
            Err(ReactiveSystemErr::ScopeDisposed(id))
        }
    }

    pub fn any_effect<T>(&self, id: (ScopeId, EffectId), f: impl FnOnce(&dyn AnyEffect) -> T) -> T {
        self.scope(id.0, |scope| {
            if let Some(n) = scope.effects.get(id.1 .0) {
                (f)(n)
            } else {
                debug_warn!("(any_effect) couldn't locate {id:?}");
                panic!("couldn't locate {id:?}");
            }
        })
    }

    pub fn try_any_effect<T>(
        &self,
        id: (ScopeId, EffectId),
        f: impl FnOnce(&dyn AnyEffect) -> T,
    ) -> Result<Result<T, ReactiveSystemErr>, ReactiveSystemErr> {
        self.try_scope(id.0, |scope| {
            if let Some(n) = scope.effects.get(id.1 .0) {
                Ok((f)(n))
            } else {
                debug_warn!("(try_any_effect) couldn't locate {id:?}");
                Err(ReactiveSystemErr::Effect(id.0, id.1))
            }
        })
    }

    pub fn any_signal<T>(&self, id: (ScopeId, SignalId), f: impl FnOnce(&dyn AnySignal) -> T) -> T {
        self.scope(id.0, |scope| {
            if let Some(n) = scope.signals.get(id.1 .0) {
                (f)(n)
            } else {
                debug_warn!("(any_signal) couldn't locate {id:?}");
                panic!("couldn't locate {id:?}");
            }
        })
    }

    pub fn signal<T, U>(&self, id: (ScopeId, SignalId), f: impl FnOnce(&SignalState<T>) -> U) -> U
    where
        T: 'static,
    {
        self.any_signal(id, |n| {
            if let Some(n) = n.as_any().downcast_ref::<SignalState<T>>() {
                f(n)
            } else {
                panic!(
                    "couldn't convert {id:?} to SignalState<{}>",
                    std::any::type_name::<T>()
                );
            }
        })
    }

    pub fn resource<S, T, U>(
        &self,
        id: (ScopeId, ResourceId),
        f: impl FnOnce(&ResourceState<S, T>) -> U,
    ) -> U
    where
        S: Debug + Clone + 'static,
        T: Debug + Clone + 'static,
    {
        self.scope(id.0, |scope| {
            if let Some(n) = scope.resources.get(id.1 .0) {
                if let Some(n) = n.as_any().downcast_ref::<ResourceState<S, T>>() {
                    f(n)
                } else {
                    panic!(
                        "couldn't convert {id:?} to ResourceState<{}, {}>",
                        std::any::type_name::<S>(),
                        std::any::type_name::<T>(),
                    );
                }
            } else {
                panic!("couldn't locate {id:?}");
            }
        })
    }

    pub fn running_effect(&self) -> Option<Subscriber> {
        self.stack.borrow().last().cloned()
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
        let scope = Scope { runtime: self, id };
        f(scope);

        ScopeDisposer(Box::new(move || scope.dispose()))
    }

    pub fn run_scope<T>(&'static self, f: impl FnOnce(Scope) -> T, parent: Option<Scope>) -> T {
        let id = {
            self.scopes
                .borrow_mut()
                .insert(Rc::new(ScopeState::new(parent)))
        };
        let scope = Scope { runtime: self, id };
        let ret = f(scope);

        scope.dispose();

        ret
    }

    pub fn run_scope_undisposed<T>(
        &'static self,
        f: impl FnOnce(Scope) -> T,
        parent: Option<Scope>,
    ) -> (T, ScopeDisposer) {
        let id = {
            self.scopes
                .borrow_mut()
                .insert(Rc::new(ScopeState::new(parent)))
        };
        let scope = Scope { runtime: self, id };
        let ret = f(scope);

        (ret, ScopeDisposer(Box::new(move || scope.dispose())))
    }

    pub fn push_stack(&self, id: Subscriber) {
        self.stack.borrow_mut().push(id);
    }

    pub fn pop_stack(&self) {
        self.stack.borrow_mut().pop();
    }

    pub fn untrack<T>(&self, f: impl FnOnce() -> T) -> T {
        let prev_stack = self.stack.replace(Vec::new());
        let untracked_result = f();
        self.stack.replace(prev_stack);
        untracked_result
    }

    #[cfg(feature = "hydrate")]
    pub fn start_hydration(&self, element: &web_sys::Element) {
        use std::collections::HashMap;
        use wasm_bindgen::{JsCast, UnwrapThrowExt};

        // gather hydratable elements
        let mut registry = HashMap::new();
        if let Ok(templates) = element.query_selector_all("*[data-hk]") {
            for i in 0..templates.length() {
                let node = templates
                    .item(i)
                    .unwrap_throw()
                    .unchecked_into::<web_sys::Element>();
                let key = node.get_attribute("data-hk").unwrap_throw();
                registry.insert(key, node);
            }
        }

        *self.shared_context.borrow_mut() = Some(SharedContext::new_with_registry(registry));
    }

    #[cfg(feature = "hydrate")]
    pub fn end_hydration(&self) {
        if let Some(ref mut sc) = *self.shared_context.borrow_mut() {
            sc.context = None;
        }
    }

    /// Returns IDs for all [Resource]s found on any scope.
    pub(crate) fn all_resources(&self) -> Vec<StreamingResourceId> {
        self.scopes
            .borrow()
            .iter()
            .flat_map(|(scope_id, scope)| {
                scope
                    .resources
                    .iter()
                    .enumerate()
                    .map(move |(resource_id, _)| {
                        StreamingResourceId(scope_id, ResourceId(resource_id))
                    })
            })
            .collect()
    }

    #[cfg(feature = "ssr")]
    pub(crate) fn serialization_resolvers(
        &self,
    ) -> futures::stream::futures_unordered::FuturesUnordered<
        std::pin::Pin<Box<dyn futures::Future<Output = (StreamingResourceId, String)>>>,
    > {
        let f = futures::stream::futures_unordered::FuturesUnordered::new();
        for (id, resource) in self.scopes.borrow().iter().flat_map(|(scope_id, scope)| {
            scope
                .resources
                .iter()
                .enumerate()
                .map(move |(idx, resource)| {
                    (StreamingResourceId(scope_id, ResourceId(idx)), resource)
                })
        }) {
            f.push(resource.to_serialization_resolver(id));
        }
        f
    }
}

impl PartialEq for Runtime {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Eq for Runtime {}

impl std::hash::Hash for Runtime {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::ptr::hash(&self, state);
    }
}
