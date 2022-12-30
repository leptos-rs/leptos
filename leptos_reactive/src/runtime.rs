use crate::{
    hydration::SharedContext, serialization::Serializable, AnyEffect, AnyResource, Effect,
    EffectId, Memo, ReadSignal, ResourceId, ResourceState, RwSignal, Scope, ScopeDisposer, ScopeId,
    ScopeProperty, SignalId, WriteSignal,
};
use cfg_if::cfg_if;
use futures::stream::FuturesUnordered;
use slotmap::{SecondaryMap, SlotMap, SparseSecondaryMap};
use std::{
    any::{Any, TypeId},
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet},
    fmt::Debug,
    future::Future,
    marker::PhantomData,
    pin::Pin,
    rc::Rc,
};

pub(crate) type PinnedFuture<T> = Pin<Box<dyn Future<Output = T>>>;

cfg_if! {
    if #[cfg(any(feature = "csr", feature = "hydrate"))] {
        thread_local! {
            pub(crate) static RUNTIME: Runtime = Runtime::new();
        }
    } else {
        thread_local! {
            pub(crate) static RUNTIMES: RefCell<SlotMap<RuntimeId, Runtime>> = Default::default();
        }
    }
}

/// Get the selected runtime from the thread-local set of runtimes. On the server,
/// this will return the correct runtime. In the browser, there should only be one runtime.
pub(crate) fn with_runtime<T>(id: RuntimeId, f: impl FnOnce(&Runtime) -> T) -> T {
    // in the browser, everything should exist under one runtime
    cfg_if! {
        if #[cfg(any(feature = "csr", feature = "hydrate"))] {
            _ = id;
            RUNTIME.with(|runtime| f(runtime))
        } else {
            RUNTIMES.with(|runtimes| {
                let runtimes = runtimes.borrow();
                let runtime = runtimes
                    .get(id)
                    .expect("Tried to access a Runtime that no longer exists.");
                f(runtime)
            })
        }
    }
}

#[doc(hidden)]
#[must_use = "Runtime will leak memory if Runtime::dispose() is never called."]
/// Creates a new reactive [Runtime]. This should almost always be handled by the framework.
pub fn create_runtime() -> RuntimeId {
    cfg_if! {
        if #[cfg(any(feature = "csr", feature = "hydrate"))] {
            Default::default()
        } else {
            RUNTIMES.with(|runtimes| runtimes.borrow_mut().insert(Runtime::new()))
        }
    }
}

slotmap::new_key_type! {
    /// Unique ID assigned to a [Runtime](crate::Runtime).
    pub struct RuntimeId;
}

impl RuntimeId {
    /// Removes the runtime, disposing all its child [Scope](crate::Scope)s.
    pub fn dispose(self) {
        cfg_if! {
            if #[cfg(not(any(feature = "csr", feature = "hydrate")))] {
                let runtime = RUNTIMES.with(move |runtimes| runtimes.borrow_mut().remove(self));
                drop(runtime);
            }
        }
    }

    pub(crate) fn raw_scope_and_disposer(self) -> (Scope, ScopeDisposer) {
        with_runtime(self, |runtime| {
            let id = { runtime.scopes.borrow_mut().insert(Default::default()) };
            let scope = Scope { runtime: self, id };
            let disposer = ScopeDisposer(Box::new(move || scope.dispose()));
            (scope, disposer)
        })
    }

    pub(crate) fn run_scope_undisposed<T>(
        self,
        f: impl FnOnce(Scope) -> T,
        parent: Option<Scope>,
    ) -> (T, ScopeId, ScopeDisposer) {
        with_runtime(self, |runtime| {
            let id = { runtime.scopes.borrow_mut().insert(Default::default()) };
            if let Some(parent) = parent {
                runtime.scope_parents.borrow_mut().insert(id, parent.id);
            }
            let scope = Scope { runtime: self, id };
            let val = f(scope);
            let disposer = ScopeDisposer(Box::new(move || scope.dispose()));
            (val, id, disposer)
        })
    }

    pub(crate) fn run_scope<T>(self, f: impl FnOnce(Scope) -> T, parent: Option<Scope>) -> T {
        let (ret, _, disposer) = self.run_scope_undisposed(f, parent);
        disposer.dispose();
        ret
    }

    pub(crate) fn create_signal<T>(self, value: T) -> (ReadSignal<T>, WriteSignal<T>)
    where
        T: Any + 'static,
    {
        let id = with_runtime(self, |runtime| {
            runtime
                .signals
                .borrow_mut()
                .insert(Rc::new(RefCell::new(value)))
        });
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

    pub(crate) fn create_rw_signal<T>(self, value: T) -> RwSignal<T>
    where
        T: Any + 'static,
    {
        let id = with_runtime(self, |runtime| {
            runtime
                .signals
                .borrow_mut()
                .insert(Rc::new(RefCell::new(value)))
        });
        RwSignal {
            runtime: self,
            id,
            ty: PhantomData,
        }
    }

    pub(crate) fn create_effect<T>(self, f: impl Fn(Option<T>) -> T + 'static) -> EffectId
    where
        T: Any + 'static,
    {
        with_runtime(self, |runtime| {
            let effect = Effect {
                f,
                value: RefCell::new(None),
            };
            let id = { runtime.effects.borrow_mut().insert(Rc::new(effect)) };
            id.run::<T>(self);
            id
        })
    }

    pub(crate) fn create_memo<T>(self, f: impl Fn(Option<&T>) -> T + 'static) -> Memo<T>
    where
        T: PartialEq + Any + 'static,
    {
        let (read, write) = self.create_signal(None);

        self.create_effect(move |_| {
            let (new, changed) = read.with_no_subscription(|p| {
                let new = f(p.as_ref());
                let changed = Some(&new) != p.as_ref();
                (new, changed)
            });

            if changed {
                write.update(|n| *n = Some(new));
            }
        });

        Memo(read)
    }
}

#[derive(Default)]
pub(crate) struct Runtime {
    pub shared_context: RefCell<SharedContext>,
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
    pub effects: RefCell<SlotMap<EffectId, Rc<dyn AnyEffect>>>,
    pub effect_sources: RefCell<SecondaryMap<EffectId, RefCell<HashSet<SignalId>>>>,
    pub resources: RefCell<SlotMap<ResourceId, AnyResource>>,
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

    pub(crate) fn create_unserializable_resource<S, T>(
        &self,
        state: Rc<ResourceState<S, T>>,
    ) -> ResourceId
    where
        S: Clone + 'static,
        T: 'static,
    {
        self.resources
            .borrow_mut()
            .insert(AnyResource::Unserializable(state))
    }

    pub(crate) fn create_serializable_resource<S, T>(
        &self,
        state: Rc<ResourceState<S, T>>,
    ) -> ResourceId
    where
        S: Clone + 'static,
        T: Serializable + 'static,
    {
        self.resources
            .borrow_mut()
            .insert(AnyResource::Serializable(state))
    }

    pub(crate) fn resource<S, T, U>(
        &self,
        id: ResourceId,
        f: impl FnOnce(&ResourceState<S, T>) -> U,
    ) -> U
    where
        S: 'static,
        T: 'static,
    {
        let resources = self.resources.borrow();
        let res = resources.get(id);
        if let Some(res) = res {
            let res_state = match res {
                AnyResource::Unserializable(res) => res.as_any(),
                AnyResource::Serializable(res) => res.as_any(),
            }
            .downcast_ref::<ResourceState<S, T>>();

            if let Some(n) = res_state {
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
    pub(crate) fn all_resources(&self) -> Vec<ResourceId> {
        self.resources
            .borrow()
            .iter()
            .map(|(resource_id, _)| resource_id)
            .collect()
    }

    /// Returns IDs for all [Resource]s found on any scope, pending from the server.
    pub(crate) fn pending_resources(&self) -> Vec<ResourceId> {
        self.resources
            .borrow()
            .iter()
            .filter_map(|(resource_id, res)| if matches!(res, AnyResource::Serializable(_)) {
                Some(resource_id)
            } else {
                None
            })
            .collect()
    }

    pub(crate) fn serialization_resolvers(
        &self,
    ) -> FuturesUnordered<PinnedFuture<(ResourceId, String)>> {
        let f = FuturesUnordered::new();
        for (id, resource) in self.resources.borrow().iter() {
            if let AnyResource::Serializable(resource) = resource {
                f.push(resource.to_serialization_resolver(id));
            }
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
