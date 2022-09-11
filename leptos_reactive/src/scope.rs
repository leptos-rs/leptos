use crate::{
    hydration::SharedContext, AnyEffect, AnyResource, AnySignal, EffectId, EffectState, ReadSignal,
    ResourceId, ResourceState, Runtime, SignalId, SignalState, StreamingResourceId, WriteSignal,
};
use elsa::FrozenVec;
use serde::{de::DeserializeOwned, Serialize};
use std::{
    any::{Any, TypeId},
    cell::RefCell,
    collections::HashMap,
    fmt::Debug,
    rc::Rc,
};

#[must_use = "Scope will leak memory if the disposer function is never called"]
/// Creates a child reactive scope and runs the function within it. This is useful for applications
/// like a list or a router, which may want to create child scopes and dispose of them when
/// they are no longer needed (e.g., a list item has been destroyed or the user has navigated away
/// from the route.)
pub fn create_scope(f: impl FnOnce(Scope) + 'static) -> ScopeDisposer {
    let runtime = Box::leak(Box::new(Runtime::new()));
    runtime.create_scope(f, None)
}

/// Creates a temporary scope, runs the given function, disposes of the scope,
/// and returns the value returned from the function. This is very useful for short-lived
/// applications like SSR, where actual reactivity is not required beyond the end
/// of the synchronous operation.
pub fn run_scope<T>(f: impl FnOnce(Scope) -> T + 'static) -> T {
    // TODO this leaks the runtime — should unsafely upgrade the lifetime, and then drop it after the scope is run
    let runtime = Box::leak(Box::new(Runtime::new()));
    runtime.run_scope(f, None)
}

#[must_use = "Scope will leak memory if the disposer function is never called"]
/// Creates a temporary scope and run the given function without disposing of the scope.
/// If you do not dispose of the scope on your own, memory will leak.
pub fn run_scope_undisposed<T>(f: impl FnOnce(Scope) -> T + 'static) -> (T, ScopeDisposer) {
    // TODO this leaks the runtime — should unsafely upgrade the lifetime, and then drop it after the scope is run
    let runtime = Box::leak(Box::new(Runtime::new()));
    runtime.run_scope_undisposed(f, None)
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Scope {
    pub(crate) runtime: &'static Runtime,
    pub(crate) id: ScopeId,
}

impl Scope {
    pub fn id(&self) -> ScopeId {
        self.id
    }

    pub fn child_scope(self, f: impl FnOnce(Scope)) -> ScopeDisposer {
        self.runtime.create_scope(f, Some(self))
    }

    pub fn transition_pending(&self) -> bool {
        self.runtime.transition().is_some()
    }

    pub fn untrack<T>(&self, f: impl FnOnce() -> T) -> T {
        self.runtime.untrack(f)
    }
}

// Internals
impl Scope {
    pub(crate) fn push_signal<T>(&self, state: SignalState<T>) -> SignalId
    where
        T: Debug + 'static,
    {
        self.runtime.scope(self.id, |scope| {
            scope.signals.push(Box::new(state));
            SignalId(scope.signals.len() - 1)
        })
    }

    pub(crate) fn push_effect<T>(&self, state: EffectState<T>) -> EffectId
    where
        T: Debug + 'static,
    {
        self.runtime.scope(self.id, |scope| {
            scope.effects.push(Box::new(state));
            EffectId(scope.effects.len() - 1)
        })
    }

    pub(crate) fn push_resource<S, T>(&self, state: Rc<ResourceState<S, T>>) -> ResourceId
    where
        S: Debug + Clone + 'static,
        T: Debug + Clone + Serialize + DeserializeOwned + 'static,
    {
        self.runtime.scope(self.id, |scope| {
            scope.resources.push(state);
            ResourceId(scope.resources.len() - 1)
        })
    }

    pub fn dispose(self) {
        if let Some(scope) = self.runtime.scopes.borrow_mut().remove(self.id) {
            for id in scope.children.take() {
                Scope {
                    runtime: self.runtime,
                    id,
                }
                .dispose();
            }

            for effect in &scope.effects {
                effect.clear_dependencies();
            }

            for cleanup in scope.cleanups.take() {
                (cleanup)();
            }

            drop(scope);
        }
    }

    #[cfg(any(feature = "csr", feature = "hydrate"))]
    pub fn start_hydration(&self, element: &web_sys::Element) {
        self.runtime.start_hydration(element);
    }

    #[cfg(any(feature = "csr", feature = "hydrate"))]
    pub fn end_hydration(&self) {
        self.runtime.end_hydration();
    }

    #[cfg(any(feature = "csr", feature = "hydrate"))]
    pub fn get_next_element(&self, template: &web_sys::Element) -> web_sys::Element {
        //log::debug!("get_next_element");
        use wasm_bindgen::{JsCast, UnwrapThrowExt};

        let cloned_template = |t: &web_sys::Element| {
            let t = t
                .unchecked_ref::<web_sys::HtmlTemplateElement>()
                .content()
                .clone_node_with_deep(true)
                .unwrap_throw()
                .unchecked_into::<web_sys::Element>()
                .first_element_child()
                .unwrap_throw();
            t
        };

        if let Some(ref mut shared_context) = &mut *self.runtime.shared_context.borrow_mut() {
            if shared_context.context.is_some() {
                let key = shared_context.next_hydration_key();
                let node = shared_context.registry.remove(&key.to_string());

                log::debug!("searching for node with key {key}");

                if let Some(node) = node {
                    log::debug!("found node with key {key}");
                    shared_context.completed.push(node.clone());
                    node
                } else {
                    log::debug!("could not find node with key {key}");
                    cloned_template(template)
                }
            } else {
                cloned_template(template)
            }
        } else {
            cloned_template(template)
        }
    }

    #[cfg(any(feature = "csr", feature = "hydrate"))]
    pub fn get_next_marker(&self, start: &web_sys::Node) -> (web_sys::Node, Vec<web_sys::Node>) {
        let mut end = Some(start.clone());
        let mut count = 0;
        let mut current = Vec::new();
        let mut start = start.clone();

        if self
            .runtime
            .shared_context
            .borrow()
            .as_ref()
            .map(|sc| sc.context.as_ref())
            .is_some()
        {
            while let Some(curr) = end {
                start = curr.clone();
                if curr.node_type() == 8 {
                    // COMMENT
                    let v = curr.node_value();
                    if v == Some("#".to_string()) {
                        count += 1;
                    } else if v == Some("/".to_string()) {
                        count -= 1;
                        if count == 0 {
                            current.push(curr.clone());
                            return (curr, current);
                        }
                    }
                }
                current.push(curr.clone());
                end = curr.next_sibling();
            }
        }

        (start, current)
    }

    pub fn next_hydration_key(&self) -> String {
        let mut sc = self.runtime.shared_context.borrow_mut();
        if let Some(ref mut sc) = *sc {
            sc.next_hydration_key()
        } else {
            let mut new_sc = SharedContext::default();
            let id = new_sc.next_hydration_key();
            *sc = Some(new_sc);
            id
        }
    }

    pub fn with_next_context<T>(&self, f: impl FnOnce() -> T) -> T {
        if self
            .runtime
            .shared_context
            .borrow()
            .as_ref()
            .and_then(|sc| sc.context.as_ref())
            .is_some()
        {
            let c = {
                if let Some(ref mut sc) = *self.runtime.shared_context.borrow_mut() {
                    if let Some(ref mut context) = sc.context {
                        let next = context.next_hydration_context();
                        Some(std::mem::replace(context, next))
                    } else {
                        None
                    }
                } else {
                    None
                }
            };

            let res = self.untrack(f);

            if let Some(ref mut sc) = *self.runtime.shared_context.borrow_mut() {
                sc.context = c;
            }
            res
        } else {
            self.untrack(f)
        }
    }

    /// Returns IDs for all [Resource]s found on any scope.
    pub fn all_resources(&self) -> Vec<StreamingResourceId> {
        self.runtime.all_resources()
    }

    /// Returns IDs for all [Resource]s found on any scope.
    #[cfg(feature = "ssr")]
    pub fn serialization_resolvers(
        &self,
    ) -> futures::stream::futures_unordered::FuturesUnordered<
        std::pin::Pin<Box<dyn futures::Future<Output = (StreamingResourceId, String)>>>,
    > {
        self.runtime.serialization_resolvers()
    }
}

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

slotmap::new_key_type! { pub struct ScopeId; }

pub(crate) struct ScopeState {
    pub(crate) parent: Option<Scope>,
    pub(crate) contexts: RefCell<HashMap<TypeId, Box<dyn Any>>>,
    pub(crate) children: RefCell<Vec<ScopeId>>,
    pub(crate) signals: FrozenVec<Box<dyn AnySignal>>,
    pub(crate) effects: FrozenVec<Box<dyn AnyEffect>>,
    pub(crate) resources: FrozenVec<Rc<dyn AnyResource>>,
    pub(crate) cleanups: RefCell<Vec<Box<dyn FnOnce()>>>,
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
            signals: Default::default(),
            effects: Default::default(),
            resources: Default::default(),
            cleanups: Default::default(),
        }
    }
}
