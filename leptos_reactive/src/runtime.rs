use crate::{
    hydration::SharedContext, AnyEffect, AnyResource, Effect, EffectId, Memo, ReadSignal,
    ResourceId, ResourceState, RwSignal, Scope, ScopeDisposer, ScopeId, ScopeProperty, SignalId,
    WriteSignal,
};
use serde::{de::DeserializeOwned, Serialize};
use slotmap::{SecondaryMap, SlotMap, SparseSecondaryMap};
use std::{
    any::{Any, TypeId},
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet},
    fmt::Debug,
    marker::PhantomData,
    rc::Rc,
};

#[derive(Default)]
pub(crate) struct Runtime {
    pub shared_context: RefCell<Option<SharedContext>>,
    pub observer: Cell<Option<EffectId>>,
    pub scopes: RefCell<SlotMap<ScopeId, RefCell<Vec<ScopeProperty>>>>,
    pub scope_parents: RefCell<SparseSecondaryMap<ScopeId, ScopeId>>,
    pub scope_children: RefCell<SparseSecondaryMap<ScopeId, Vec<ScopeId>>>,
    #[allow(clippy::type_complexity)]
    pub scope_contexts: RefCell<SparseSecondaryMap<ScopeId, HashMap<TypeId, Box<dyn Any>>>>,
    #[allow(clippy::type_complexity)]
    pub scope_cleanups: RefCell<SparseSecondaryMap<ScopeId, Vec<Box<dyn FnOnce()>>>>,
    pub signals: RefCell<SlotMap<SignalId, Rc<RefCell<dyn Any>>>>,
    pub signal_subscribers: RefCell<SecondaryMap<SignalId, RefCell<HashSet<EffectId>>>>,
    pub effects: RefCell<SlotMap<EffectId, Rc<RefCell<dyn AnyEffect>>>>,
    pub effect_sources: RefCell<SecondaryMap<EffectId, RefCell<HashSet<SignalId>>>>,
    #[cfg(feature = "resource")]
    pub resources: RefCell<SlotMap<ResourceId, Rc<dyn AnyResource>>>,
}

impl Debug for Runtime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Runtime")
            .field("shared_context", &self.shared_context)
            .field("observer", &self.observer)
            .field("scopes", &self.scopes)
            .field("scope_parents", &self.scope_parents)
            .field("scope_children", &self.scope_children)
            .field("signals", &self.signals)
            .field("signal_subscribers", &self.signal_subscribers)
            .field("effects", &self.effects.borrow().len())
            .field("effect_sources", &self.effect_sources)
            .finish()
    }
}

impl Runtime {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn run_scope_undisposed<T>(
        &'static self,
        f: impl FnOnce(Scope) -> T,
        parent: Option<Scope>,
    ) -> (T, ScopeId, ScopeDisposer) {
        let id = { self.scopes.borrow_mut().insert(Default::default()) };
        if let Some(parent) = parent {
            self.scope_parents.borrow_mut().insert(id, parent.id);
        }
        let scope = Scope { runtime: self, id };
        let val = f(scope);
        let disposer = ScopeDisposer(Box::new(move || scope.dispose()));
        (val, id, disposer)
    }

    pub fn run_scope<T>(&'static self, f: impl FnOnce(Scope) -> T, parent: Option<Scope>) -> T {
        let (ret, _, disposer) = self.run_scope_undisposed(f, parent);
        disposer.dispose();
        ret
    }

    pub(crate) fn create_signal<T>(&'static self, value: T) -> (ReadSignal<T>, WriteSignal<T>)
    where
        T: Any + 'static,
    {
        let id = self
            .signals
            .borrow_mut()
            .insert(Rc::new(RefCell::new(value)));
        (
            ReadSignal {
                runtime: self,
                id,
                ty: PhantomData,
            },
            WriteSignal {
                runtime: self,
                id,
                ty: PhantomData,
            },
        )
    }

    pub(crate) fn create_rw_signal<T>(&'static self, value: T) -> RwSignal<T>
    where
        T: Any + 'static,
    {
        let id = self
            .signals
            .borrow_mut()
            .insert(Rc::new(RefCell::new(value)));
        RwSignal {
            runtime: self,
            id,
            ty: PhantomData,
        }
    }

    pub(crate) fn create_effect<T>(
        &'static self,
        f: impl FnMut(Option<T>) -> T + 'static,
    ) -> EffectId
    where
        T: Any + 'static,
    {
        let effect = Effect { f, value: None };
        let id = {
            self.effects
                .borrow_mut()
                .insert(Rc::new(RefCell::new(effect)))
        };
        id.run::<T>(self);
        id
    }

    pub(crate) fn create_memo<T>(
        &'static self,
        mut f: impl FnMut(Option<T>) -> T + 'static,
    ) -> Memo<T>
    where
        T: Clone + PartialEq + Any + 'static,
    {
        let (read, write) = self.create_signal(None);
        self.create_effect(move |prev| {
            let new = { f(prev.clone()) };

            if prev.as_ref() != Some(&new) {
                write.update(|n| *n = Some(new.clone()));
            }
            new
        });

        Memo(read)
    }

    #[cfg(feature = "resource")]
    pub(crate) fn create_resource<S, T>(&self, state: Rc<ResourceState<S, T>>) -> ResourceId
    where
        S: Debug + Clone + 'static,
        T: Debug + Clone + Serialize + DeserializeOwned + 'static,
    {
        self.resources.borrow_mut().insert(state)
    }

    #[cfg(feature = "hydrate")]
    pub fn start_hydration(&self, element: &web_sys::Element) {
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

    #[cfg(feature = "resource")]
    pub(crate) fn resource<S, T, U>(
        &self,
        id: ResourceId,
        f: impl FnOnce(&ResourceState<S, T>) -> U,
    ) -> U
    where
        S: Debug + Clone + 'static,
        T: Debug + Clone + 'static,
    {
        let resources = self.resources.borrow();
        let res = resources.get(id);
        if let Some(res) = res {
            if let Some(n) = res.as_any().downcast_ref::<ResourceState<S, T>>() {
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
    }

    /// Returns IDs for all [Resource]s found on any scope.
    #[cfg(feature = "resource")]
    pub(crate) fn all_resources(&self) -> Vec<ResourceId> {
        self.resources
            .borrow()
            .iter()
            .map(|(resource_id, _)| resource_id)
            .collect()
    }

    #[cfg(all(feature = "ssr", feature = "resource"))]
    pub(crate) fn serialization_resolvers(
        &self,
    ) -> futures::stream::futures_unordered::FuturesUnordered<
        std::pin::Pin<Box<dyn futures::Future<Output = (ResourceId, String)>>>,
    > {
        let f = futures::stream::futures_unordered::FuturesUnordered::new();
        for (id, resource) in self.resources.borrow().iter() {
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
