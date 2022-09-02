use crate::{
    AnyEffect, AnySignal, EffectId, ResourceId, ResourceState, Scope, ScopeDisposer, ScopeId,
    ScopeState, SignalId, SignalState, Subscriber, TransitionState,
};
use slotmap::SlotMap;
use std::cell::{Cell, RefCell};
use std::fmt::Debug;
use std::rc::Rc;

#[cfg(feature = "browser")]
use crate::hydration::SharedContext;

#[derive(Default, Debug)]
pub(crate) struct Runtime {
    #[cfg(feature = "browser")]
    pub(crate) shared_context: RefCell<Option<SharedContext>>,
    pub(crate) stack: RefCell<Vec<Subscriber>>,
    pub(crate) scopes: RefCell<SlotMap<ScopeId, Rc<ScopeState>>>,
    pub(crate) transition: RefCell<Option<Rc<TransitionState>>>,
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
            log::error!(
                "couldn't locate {id:?} in scopes {:#?}",
                self.scopes.borrow()
            );
            panic!("couldn't locate {id:?}");
        }
    }

    pub fn any_effect<T>(&self, id: (ScopeId, EffectId), f: impl FnOnce(&dyn AnyEffect) -> T) -> T {
        self.scope(id.0, |scope| {
            if let Some(n) = scope.effects.get(id.1 .0) {
                (f)(n)
            } else {
                panic!("couldn't locate {id:?}");
            }
        })
    }

    pub fn any_signal<T>(&self, id: (ScopeId, SignalId), f: impl FnOnce(&dyn AnySignal) -> T) -> T {
        self.scope(id.0, |scope| {
            if let Some(n) = scope.signals.get(id.1 .0) {
                (f)(n)
            } else {
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
                if let Some(n) = n.downcast_ref::<ResourceState<S, T>>() {
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

    pub fn running_transition(&self) -> Option<Rc<TransitionState>> {
        self.transition.borrow().as_ref().and_then(|t| {
            if t.running.get() {
                Some(Rc::clone(t))
            } else {
                None
            }
        })
    }

    pub fn transition(&self) -> Option<Rc<TransitionState>> {
        self.transition.borrow().as_ref().map(Rc::clone)
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

    pub fn push_stack(&self, id: Subscriber) {
        self.stack.borrow_mut().push(id);
    }

    pub fn pop_stack(&self) {
        self.stack.borrow_mut().pop();
    }

    pub fn remove_scope(&self, scope: &ScopeId) {
        let scope = self.scopes.borrow_mut().remove(*scope);
        drop(scope); // unnecessary, but just to be explicit
    }

    pub fn untrack<T>(&self, f: impl FnOnce() -> T) -> T {
        let prev_stack = self.stack.replace(Vec::new());
        let untracked_result = f();
        self.stack.replace(prev_stack);
        untracked_result
    }

    #[cfg(feature = "browser")]
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

    #[cfg(feature = "browser")]
    pub fn end_hydration(&self) {
        if let Some(ref mut sc) = *self.shared_context.borrow_mut() {
            sc.id = None;
        }
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
