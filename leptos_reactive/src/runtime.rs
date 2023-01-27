#![forbid(unsafe_code)]
use crate::{
    hydration::SharedContext, serialization::Serializable, AnyEffect, AnyResource, Effect,
    EffectId, Memo, ReadSignal, ResourceId, ResourceState, RwSignal, Scope, ScopeDisposer, ScopeId,
    ScopeProperty, SignalId, WriteSignal,
};
use cfg_if::cfg_if;
use futures::stream::FuturesUnordered;
use lazy_static::lazy_static;
use parking_lot::RwLock;
use slotmap::{SecondaryMap, SlotMap, SparseSecondaryMap};
use std::{
    any::{Any, TypeId},
    cell::RefCell,
    collections::{HashMap, HashSet},
    fmt::Debug,
    future::Future,
    marker::PhantomData,
    pin::Pin,
    sync::Arc,
};

pub(crate) type PinnedFuture<T> = Pin<Box<dyn Future<Output = T>>>;

cfg_if! {
    if #[cfg(any(feature = "csr", feature = "hydrate"))] {
        lazy_static! {
            pub(crate) static ref RUNTIME: Runtime = Runtime::new();
        }
    } else {
        lazy_static! {
            pub(crate) static ref RUNTIMES: RwLock<SlotMap<RuntimeId, Runtime>> = Default::default();
        }
    }
}

/// Get the selected runtime from the thread-local set of runtimes. On the server,
/// this will return the correct runtime. In the browser, there should only be one runtime.
pub(crate) fn with_runtime<T>(id: RuntimeId, f: impl FnOnce(&Runtime) -> T) -> Result<T, ()> {
    // in the browser, everything should exist under one runtime
    cfg_if! {
        if #[cfg(any(feature = "csr", feature = "hydrate"))] {
            _ = id;
            Ok(f(&RUNTIME))
        } else {
            let runtimes = RUNTIMES.read();
            match runtimes.get(id) {
                None => Err(()),
                Some(runtime) => Ok(f(runtime))
            }
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
            RUNTIMES.with(|runtimes| runtimes.write().insert(Runtime::new()))
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
                let runtime = RUNTIMES.with(move |runtimes| runtimes.write().remove(self));
                drop(runtime);
            }
        }
    }

    pub(crate) fn raw_scope_and_disposer(self) -> (Scope, ScopeDisposer) {
        with_runtime(self, |runtime| {
            let id = { runtime.scopes.write().insert(Default::default()) };
            let scope = Scope { runtime: self, id };
            let disposer = ScopeDisposer(Box::new(move || scope.dispose()));
            (scope, disposer)
        })
        .expect("tried to create raw scope in a runtime that has already been disposed")
    }

    pub(crate) fn run_scope_undisposed<T>(
        self,
        f: impl FnOnce(Scope) -> T,
        parent: Option<Scope>,
    ) -> (T, ScopeId, ScopeDisposer) {
        with_runtime(self, |runtime| {
            let id = { runtime.scopes.write().insert(Default::default()) };
            if let Some(parent) = parent {
                runtime.scope_parents.write().insert(id, parent.id);
            }
            let scope = Scope { runtime: self, id };
            let val = f(scope);
            let disposer = ScopeDisposer(Box::new(move || scope.dispose()));
            (val, id, disposer)
        })
        .expect("tried to run scope in a runtime that has been disposed")
    }

    pub(crate) fn run_scope<T>(self, f: impl FnOnce(Scope) -> T, parent: Option<Scope>) -> T {
        let (ret, _, disposer) = self.run_scope_undisposed(f, parent);
        disposer.dispose();
        ret
    }

    #[track_caller]
    pub(crate) fn create_signal<T>(self, value: T) -> (ReadSignal<T>, WriteSignal<T>)
    where
        T: Any + 'static,
    {
        let id = with_runtime(self, |runtime| {
            runtime.signals.write().insert(Arc::new(RwLock::new(value)))
        })
        .expect("tried to create a signal in a runtime that has been disposed");
        (
            ReadSignal {
                runtime: self,
                id,
                ty: PhantomData,
                #[cfg(debug_assertions)]
                defined_at: std::panic::Location::caller(),
            },
            WriteSignal {
                runtime: self,
                id,
                ty: PhantomData,
                #[cfg(debug_assertions)]
                defined_at: std::panic::Location::caller(),
            },
        )
    }

    pub(crate) fn create_rw_signal<T>(self, value: T) -> RwSignal<T>
    where
        T: Any + 'static,
    {
        let id = with_runtime(self, |runtime| {
            runtime.signals.write().insert(Arc::new(RwLock::new(value)))
        })
        .expect("tried to create a signal in a runtime that has been disposed");
        RwSignal {
            runtime: self,
            id,
            ty: PhantomData,
            #[cfg(debug_assertions)]
            defined_at: std::panic::Location::caller(),
        }
    }

    #[track_caller]
    pub(crate) fn create_effect<T>(self, f: impl Fn(Option<T>) -> T + 'static) -> EffectId
    where
        T: Any + 'static,
    {
        #[cfg(debug_assertions)]
        let defined_at = std::panic::Location::caller();

        with_runtime(self, |runtime| {
            let effect = Effect {
                f,
                value: RwLock::new(None),
                #[cfg(debug_assertions)]
                defined_at,
            };
            let id = { runtime.effects.write().insert(Arc::new(effect)) };
            id.run::<T>(self);
            id
        })
        .expect("tried to create an effect in a runtime that has been disposed")
    }

    #[track_caller]
    pub(crate) fn create_memo<T>(self, f: impl Fn(Option<&T>) -> T + 'static) -> Memo<T>
    where
        T: PartialEq + Any + 'static,
    {
        #[cfg(debug_assertions)]
        let defined_at = std::panic::Location::caller();

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

        Memo(
            read,
            #[cfg(debug_assertions)]
            defined_at,
        )
    }
}

#[derive(Default)]
pub(crate) struct Runtime {
    pub shared_context: RwLock<SharedContext>,
    pub scopes: RwLock<SlotMap<ScopeId, RwLock<Vec<ScopeProperty>>>>,
    pub scope_parents: RwLock<SparseSecondaryMap<ScopeId, ScopeId>>,
    pub scope_children: RwLock<SparseSecondaryMap<ScopeId, Vec<ScopeId>>>,
    #[allow(clippy::type_complexity)]
    pub scope_contexts: RwLock<SparseSecondaryMap<ScopeId, HashMap<TypeId, Box<dyn Any>>>>,
    #[allow(clippy::type_complexity)]
    pub scope_cleanups: RwLock<SparseSecondaryMap<ScopeId, Vec<Box<dyn FnOnce()>>>>,
    pub signals: RwLock<SlotMap<SignalId, Arc<RwLock<dyn Any>>>>,
    pub signal_subscribers: RwLock<SecondaryMap<SignalId, RwLock<HashSet<EffectId>>>>,
    pub effects: RwLock<SlotMap<EffectId, Arc<dyn AnyEffect>>>,
    pub effect_sources: RwLock<SecondaryMap<EffectId, RwLock<HashSet<SignalId>>>>,
    pub resources: RwLock<SlotMap<ResourceId, AnyResource>>,
}

// track current observer thread-locally
// because effects run synchronously, the current observer
// *in this thread* will not change during the execution of an effect.
// but if we track this across threads, it's possible for overlapping
// executions to cause the stack to be out of order
// so we store at most one current observer per runtime, per thread
thread_local! {
    static OBSERVER: RefCell<HashMap<RuntimeId, EffectId>> = Default::default();
}

pub(crate) struct LocalObserver {}

impl LocalObserver {
    pub fn take(runtime: RuntimeId) -> Option<EffectId> {
        OBSERVER.with(|observer| observer.borrow_mut().remove(&runtime))
    }

    pub fn get(runtime: RuntimeId) -> Option<EffectId> {
        OBSERVER.with(|observer| observer.borrow().get(&runtime).copied())
    }

    pub fn set(runtime: RuntimeId, effect: Option<EffectId>) {
        OBSERVER.with(|observer| {
            if let Some(value) = effect {
                observer.borrow_mut().insert(runtime, value)
            } else {
                observer.borrow_mut().remove(&runtime)
            }
        });
    }
}

impl Debug for Runtime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Runtime")
            .field("shared_context", &self.shared_context)
            .field("scopes", &self.scopes)
            .field("scope_parents", &self.scope_parents)
            .field("scope_children", &self.scope_children)
            .field("signals", &self.signals)
            .field("signal_subscribers", &self.signal_subscribers)
            .field("effects", &self.effects.read().len())
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
        state: Arc<ResourceState<S, T>>,
    ) -> ResourceId
    where
        S: Clone + 'static,
        T: 'static,
    {
        self.resources
            .write()
            .insert(AnyResource::Unserializable(state))
    }

    pub(crate) fn create_serializable_resource<S, T>(
        &self,
        state: Arc<ResourceState<S, T>>,
    ) -> ResourceId
    where
        S: Clone + 'static,
        T: Serializable + 'static,
    {
        self.resources
            .write()
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
        let resources = self.resources.read();
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
            .read()
            .iter()
            .map(|(resource_id, _)| resource_id)
            .collect()
    }

    /// Returns IDs for all [Resource]s found on any scope, pending from the server.
    pub(crate) fn pending_resources(&self) -> Vec<ResourceId> {
        self.resources
            .read()
            .iter()
            .filter_map(|(resource_id, res)| {
                if matches!(res, AnyResource::Serializable(_)) {
                    Some(resource_id)
                } else {
                    None
                }
            })
            .collect()
    }

    pub(crate) fn serialization_resolvers(
        &self,
    ) -> FuturesUnordered<PinnedFuture<(ResourceId, String)>> {
        let f = FuturesUnordered::new();
        for (id, resource) in self.resources.read().iter() {
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
