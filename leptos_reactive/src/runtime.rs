#![forbid(unsafe_code)]
use crate::{
    hydration::SharedContext, AnyEffect, AnyResource, Effect, 
    ReadSignal, ResourceId, ResourceState, RwSignal, Scope, ScopeDisposer,
    ScopeId, ScopeProperty, SerializableResource, SignalUpdate,
    UnserializableResource, WriteSignal, node::{NodeId, ReactiveNode, ReactiveNodeType, ReactiveNodeState}, Memo,
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
    rc::Rc
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
pub(crate) fn with_runtime<T>(
    id: RuntimeId,
    f: impl FnOnce(&Runtime) -> T,
) -> Result<T, ()> {
    // in the browser, everything should exist under one runtime
    cfg_if! {
        if #[cfg(any(feature = "csr", feature = "hydrate"))] {
            _ = id;
            Ok(RUNTIME.with(|runtime| f(runtime)))
        } else {
            RUNTIMES.with(|runtimes| {
                let runtimes = runtimes.borrow();
                match runtimes.get(id) {
                    None => Err(()),
                    Some(runtime) => Ok(f(runtime))
                }
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
        .expect(
            "tried to create raw scope in a runtime that has already been \
             disposed",
        )
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
        .expect("tried to run scope in a runtime that has been disposed")
    }

    pub(crate) fn run_scope<T>(
        self,
        f: impl FnOnce(Scope) -> T,
        parent: Option<Scope>,
    ) -> T {
        let (ret, _, disposer) = self.run_scope_undisposed(f, parent);
        disposer.dispose();
        ret
    }

    #[track_caller]
    pub(crate) fn create_concrete_signal(
        self,
        value: Rc<RefCell<dyn Any>>,
    ) -> NodeId {
        with_runtime(self, |runtime| runtime.nodes.borrow_mut().insert(ReactiveNode {
            value,
            node_type: ReactiveNodeType::Signal
        }))
            .expect(
                "tried to create a signal in a runtime that has been disposed",
            )
    }

    #[track_caller]
    pub(crate) fn create_signal<T>(
        self,
        value: T,
    ) -> (ReadSignal<T>, WriteSignal<T>)
    where
        T: Any + 'static,
    {
        let id = self.create_concrete_signal(
            Rc::new(RefCell::new(value)) as Rc<RefCell<dyn Any>>
        );

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

    #[track_caller]
    pub(crate) fn create_many_signals_with_map<T, U>(
        self,
        cx: Scope,
        values: impl IntoIterator<Item = T>,
        map_fn: impl Fn((ReadSignal<T>, WriteSignal<T>)) -> U,
    ) -> Vec<U>
    where
        T: Any + 'static,
    {
        with_runtime(self, move |runtime| {
            let mut signals = runtime.nodes.borrow_mut();
            let properties = runtime.scopes.borrow();
            let mut properties = properties
                .get(cx.id)
                .expect(
                    "tried to add signals to a scope that has been disposed",
                )
                .borrow_mut();
            let values = values.into_iter();
            let size = values.size_hint().0;
            signals.reserve(size);
            properties.reserve(size);
            values
                .map(|value| signals.insert(ReactiveNode {
                    value: Rc::new(RefCell::new(value)),
                    node_type: ReactiveNodeType::Signal
                }))
                .map(|id| {
                    properties.push(ScopeProperty::Signal(id));
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
                })
                .map(map_fn)
                .collect()
        })
        .expect("tried to create a signal in a runtime that has been disposed")
    }

    #[track_caller]
    pub(crate) fn create_rw_signal<T>(self, value: T) -> RwSignal<T>
    where
        T: Any + 'static,
    {
        let id = self.create_concrete_signal(
            Rc::new(RefCell::new(value)) as Rc<RefCell<dyn Any>>
        );
        RwSignal {
            runtime: self,
            id,
            ty: PhantomData,
            #[cfg(debug_assertions)]
            defined_at: std::panic::Location::caller(),
        }
    }

    #[track_caller]
    pub(crate) fn create_concrete_effect(
        self,
        effect: Rc<dyn AnyEffect>,
    ) -> NodeId {
        with_runtime(self, |runtime| {
            runtime.nodes.borrow_mut().insert(ReactiveNode {
                value: effect.default_value(),
                node_type: ReactiveNodeType::Effect(effect)
            })
        })
        .expect("tried to create an effect in a runtime that has been disposed")
    }

    #[track_caller]
    pub(crate) fn create_effect<T>(
        self,
        f: impl Fn(Option<T>) -> T + 'static,
    ) -> NodeId
    where
        T: Any + 'static,
    {
        #[cfg(debug_assertions)]
        let defined_at = std::panic::Location::caller();

        let effect = Effect {
            f,
            value: RefCell::new(None),
            #[cfg(debug_assertions)]
            defined_at,
        };

        let id = self.create_concrete_effect(Rc::new(effect));
        id.run_effect(self);
        id
    }

    #[track_caller]
    pub(crate) fn create_memo<T>(
        self,
        f: impl Fn(Option<&T>) -> T + 'static,
    ) -> Memo<T>
    where
        T: PartialEq + Any + 'static,
    {
        #[cfg(debug_assertions)]
        let defined_at = std::panic::Location::caller();

        let id = with_runtime(self, |runtime| {
            runtime.nodes.borrow_mut().insert(ReactiveNode {
                value: Rc::new(RefCell::new(None::<T>)),
                node_type: ReactiveNodeType::Memo,
            })
        })
        .expect("tried to create an effect in a runtime that has been disposed");

        Memo {
            runtime: self,
            id,
            ty: PhantomData,
            defined_at,
        }
    }
}

#[derive(Default)]
pub(crate) struct Runtime {
    pub shared_context: RefCell<SharedContext>,
    pub observer: Cell<Option<NodeId>>,
    pub scopes: RefCell<SlotMap<ScopeId, RefCell<Vec<ScopeProperty>>>>,
    pub scope_parents: RefCell<SparseSecondaryMap<ScopeId, ScopeId>>,
    pub scope_children: RefCell<SparseSecondaryMap<ScopeId, Vec<ScopeId>>>,
    #[allow(clippy::type_complexity)]
    pub scope_contexts:
        RefCell<SparseSecondaryMap<ScopeId, HashMap<TypeId, Box<dyn Any>>>>,
    #[allow(clippy::type_complexity)]
    pub scope_cleanups:
        RefCell<SparseSecondaryMap<ScopeId, Vec<Box<dyn FnOnce()>>>>,
    pub nodes: RefCell<SlotMap<NodeId, ReactiveNode>>,
    pub node_states: RefCell<SecondaryMap<NodeId, ReactiveNodeState>>,
    pub node_subscribers:
        RefCell<SecondaryMap<NodeId, RefCell<HashSet<NodeId>>>>,
    pub node_sources:
        RefCell<SecondaryMap<NodeId, RefCell<HashSet<NodeId>>>>,
    pub pending_effects: RefCell<Vec<NodeId>>,
    pub resources: RefCell<SlotMap<ResourceId, AnyResource>>,
}

impl Runtime {
    pub(crate) fn mark_dirty(&self, node: NodeId) {
        let nodes = self.nodes.borrow();
        let mut pending_effects = self.pending_effects.borrow_mut();
        let mut states = self.node_states.borrow_mut();
        let subscribers = self.node_subscribers.borrow();
        let children = subscribers.get(node);
        if let Some(children) = children {
            // collect descendants to mark as Check
            let mut descendants = Vec::new();

            // mark immediate children dirty
            for child in children.borrow().iter() {
                states.insert(*child, ReactiveNodeState::Dirty);
                Runtime::gather_descendants(&subscribers, *child, &mut descendants);
                match nodes.get(*child) {
                    Some(ReactiveNode { node_type: ReactiveNodeType::Effect(_), ..}) => {
                       pending_effects.push(*child); 
                    }
                    _ => {}
                }
            }

            // mark descendants check
            for descendant in descendants {
                match states.get_mut(descendant) {
                    // if already dirty or check, noop
                    Some(ReactiveNodeState::Dirty) | Some(ReactiveNodeState::Check) => {}
                    // if clean, mark dirty 
                    Some(state) => {
                        *state = ReactiveNodeState::Dirty;
                    }
                    None => {
                        states.insert(descendant, ReactiveNodeState::Check);
                    }
                };

                match nodes.get(descendant) {
                    Some(ReactiveNode { node_type: ReactiveNodeType::Effect(_), ..}) => {
                       pending_effects.push(descendant); 
                    }
                    _ => {}
                }
            }
        }
    }

    fn gather_descendants(subscribers: &SecondaryMap<NodeId, RefCell<HashSet<NodeId>>>, node: NodeId, descendants: &mut Vec<NodeId>) {
        if let Some(children) = subscribers.get(node) {
            for child in children.borrow().iter() {
                descendants.push(*child);
                Runtime::gather_descendants(subscribers, *child, descendants);
            }
        }
    }

    pub(crate) fn run_effects(runtime_id: RuntimeId) {
        with_runtime(runtime_id, |runtime| {
            let effects = runtime.pending_effects.take();
            for effect in effects {
                let should_run = {
                    matches!(
                        runtime.nodes.borrow().get(effect),
                        Some(ReactiveNode { node_type: ReactiveNodeType::Effect(_), ..})
                    )
                };
                if should_run {
                    effect.run_effect(runtime_id);
                }
            }
        });
    }
}

impl Debug for Runtime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Runtime")
            .field("shared_context", &self.shared_context)
            .field("observer", &self.observer)
            .field("scopes", &self.scopes)
            .field("scope_parents", &self.scope_parents)
            .field("scope_children", &self.scope_children)
            .finish()
    }
}

impl Runtime {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn create_unserializable_resource(
        &self,
        state: Rc<dyn UnserializableResource>,
    ) -> ResourceId {
        self.resources
            .borrow_mut()
            .insert(AnyResource::Unserializable(state))
    }

    pub(crate) fn create_serializable_resource(
        &self,
        state: Rc<dyn SerializableResource>,
    ) -> ResourceId {
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

    /// Returns IDs for all [resources](crate::Resource) found on any scope.
    pub(crate) fn all_resources(&self) -> Vec<ResourceId> {
        self.resources
            .borrow()
            .iter()
            .map(|(resource_id, _)| resource_id)
            .collect()
    }

    /// Returns IDs for all [resources](crate::Resource) found on any
    /// scope, pending from the server.
    pub(crate) fn pending_resources(&self) -> Vec<ResourceId> {
        self.resources
            .borrow()
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
        cx: Scope,
    ) -> FuturesUnordered<PinnedFuture<(ResourceId, String)>> {
        let f = FuturesUnordered::new();
        for (id, resource) in self.resources.borrow().iter() {
            if let AnyResource::Serializable(resource) = resource {
                f.push(resource.to_serialization_resolver(cx, id));
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
