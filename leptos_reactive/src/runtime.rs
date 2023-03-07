#![forbid(unsafe_code)]
use crate::{
    hydration::SharedContext,
    node::{NodeId, ReactiveNode, ReactiveNodeState, ReactiveNodeType},
    AnyEffect, AnyMemo, AnyResource, Effect, Memo, MemoState, ReadSignal,
    ResourceId, ResourceState, RwSignal, Scope, ScopeDisposer, ScopeId,
    ScopeProperty, SerializableResource, SignalError, SignalUpdate,
    UnserializableResource, WriteSignal,
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
        with_runtime(self, |runtime| {
            runtime.nodes.borrow_mut().insert(ReactiveNode {
                value,
                node_type: ReactiveNodeType::Signal,
            })
        })
        .expect("tried to create a signal in a runtime that has been disposed")
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
                .map(|value| {
                    signals.insert(ReactiveNode {
                        value: Rc::new(RefCell::new(value)),
                        node_type: ReactiveNodeType::Signal,
                    })
                })
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
        value: Rc<RefCell<dyn Any>>,
        effect: Rc<dyn AnyEffect>,
    ) -> NodeId {
        with_runtime(self, |runtime| {
            let id = runtime.nodes.borrow_mut().insert(ReactiveNode {
                value: Rc::clone(&value),
                node_type: ReactiveNodeType::Effect(Rc::clone(&effect)),
            });

            // run the effect for the first time
            let prev_observer = runtime.observer.take();
            runtime.observer.set(Some(id));

            effect.run(value);

            runtime.observer.set(prev_observer);

            id
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
            ty: PhantomData,
            #[cfg(debug_assertions)]
            defined_at,
        };

        let value = Rc::new(RefCell::new(None::<T>));
        let id = self.create_concrete_effect(value, Rc::new(effect));

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
                // starts at dirty because memos are lazy
                // when created, this has never run
                node_type: ReactiveNodeType::Memo {
                    state: ReactiveNodeState::Dirty,
                    f: Rc::new(MemoState { f, t: PhantomData }),
                },
            })
        })
        .expect("tried to create a memo in a runtime that has been disposed");
        //eprintln!("created memo {id:?}");

        Memo {
            runtime: self,
            id,
            ty: PhantomData,
            #[cfg(debug_assertions)]
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
    pub node_subscribers:
        RefCell<SecondaryMap<NodeId, RefCell<HashSet<NodeId>>>>,
    pub node_sources: RefCell<SecondaryMap<NodeId, RefCell<HashSet<NodeId>>>>,
    pub pending_effects: RefCell<Vec<NodeId>>,
    pub resources: RefCell<SlotMap<ResourceId, AnyResource>>,
}

// In terms of concept and algorithm, this reactive-system implementation
// is significantly inspired by Reactively (https://github.com/modderme123/reactively)
impl Runtime {
    pub(crate) fn latest_value(
        &self,
        node_id: NodeId,
    ) -> Result<Rc<RefCell<dyn Any>>, SignalError> {
        //eprintln!("getting latest value for {node_id:?}");
        let node = { self.nodes.borrow().get(node_id).cloned() };
        match node {
            None => Err(SignalError::Disposed),
            Some(ReactiveNode { value, node_type }) => {
                Ok(match node_type {
                    ReactiveNodeType::Signal | ReactiveNodeType::Effect(_) => {
                        //eprintln!("  is signal, returning value");
                        Rc::clone(&value)
                    }
                    ReactiveNodeType::Memo { state, f } => {
                        match state {
                            ReactiveNodeState::Clean => {
                                //eprintln!("  clean, returning value");
                                Rc::clone(&value)
                            }
                            ReactiveNodeState::Dirty => {
                                //eprintln!("  dirty, rerunning");
                                let changed = self
                                    .with_observer(node_id, || {
                                        f.update(Rc::clone(&value))
                                    });
                                self.mark_clean(node_id);
                                if changed {
                                    self.mark_children_dirty(node_id);
                                }

                                value
                            }
                            ReactiveNodeState::Check => {
                                //eprintln!("  checking parents");
                                let parents = self
                                    .node_sources
                                    .borrow()
                                    .get(node_id)
                                    .map(|parents| {
                                        parents.borrow().clone().into_iter()
                                    })
                                    .into_iter()
                                    .flatten();
                                // check each parent
                                for parent in parents {
                                    //eprintln!("    checking {parent:?}");
                                    _ = self.latest_value(parent);
                                }

                                let state = self.current_state(node_id);
                                //eprintln!("current state of {node_id:?} is {state:?}");
                                let value = if state == ReactiveNodeState::Dirty
                                {
                                    let changed = self
                                        .with_observer(node_id, || {
                                            f.update(Rc::clone(&value))
                                        });
                                    if changed {
                                        self.mark_children_dirty(node_id);
                                    }

                                    value
                                } else {
                                    Rc::clone(&value)
                                };
                                self.mark_clean(node_id);
                                value

                                // check if we're marked dirty
                                // check to see if any of the parents are dirty
                                /*let parents = self
                                    .node_sources
                                    .borrow()
                                    .get(node_id)
                                    .cloned()
                                    .map(|n| n.borrow().clone())
                                    .into_iter()
                                    .flatten();
                                let mut possibly_dirty_parents = parents.flat_map(|node_id| {
                                    let f = Rc::clone(&f);
                                    self.nodes.borrow().get(node_id).cloned().into_iter().filter_map(move |node| match &node.node_type {
                                        ReactiveNodeType::Signal | ReactiveNodeType::Effect(_) => None,
                                        ReactiveNodeType::Memo { state, .. } => {
                                            match state {
                                                ReactiveNodeState::Clean => None,
                                                _ => Some((node_id, Rc::clone(&node.value), f.clone()))
                                        }
                                    }
                                    })
                                })
                                .filter(|(parent_id, parent_value, parent)| {
                                        let changed = self.with_observer(*parent_id, || {
                                            //eprintln!("updating {parent_id:?}");
                                            parent.update(Rc::clone(&parent_value))
                                        });
                                        self.mark_clean(*parent_id);
                                        changed
                                });
                                if possibly_dirty_parents.next().is_some() {
                                    self.with_observer(node_id, || {
                                        f.update(Rc::clone(&value))
                                    });
                                    self.mark_clean(node_id);
                                    Rc::clone(&value)
                                } else {
                                    //eprintln!("    no dirty parents");
                                    Rc::clone(&value)
                                }*/
                            }
                        }
                    }
                })
            }
        }
    }

    fn current_state(&self, node: NodeId) -> ReactiveNodeState {
        match self.nodes.borrow().get(node) {
            None => ReactiveNodeState::Clean,
            Some(node) => match &node.node_type {
                ReactiveNodeType::Signal => ReactiveNodeState::Clean,
                ReactiveNodeType::Memo { state, f } => *state,
                ReactiveNodeType::Effect(_) => ReactiveNodeState::Clean,
            },
        }
    }

    fn with_observer<T>(&self, observer: NodeId, f: impl FnOnce() -> T) -> T {
        let prev_observer = self.observer.take();
        self.observer.set(Some(observer));
        let v = f();
        self.observer.set(prev_observer);
        v
    }

    fn mark_clean(&self, node: NodeId) {
        //eprintln!("marking {node:?} clean");
        let mut nodes = self.nodes.borrow_mut();
        let node = nodes.get_mut(node);
        if let Some(ReactiveNode {
            node_type: ReactiveNodeType::Memo { state, .. },
            ..
        }) = node
        {
            *state = ReactiveNodeState::Clean
        }
    }

    pub(crate) fn mark_children_dirty(&self, node: NodeId) {
        //eprintln!("marking {node:?} dirty");
        let mut nodes = self.nodes.borrow_mut();
        let mut pending_effects = self.pending_effects.borrow_mut();
        let subscribers = self.node_subscribers.borrow();
        let children = subscribers.get(node);
        if let Some(children) = children {
            // collect descendants to mark as Check
            let mut descendants = Vec::new();

            // mark immediate children dirty
            for child in children.borrow().iter() {
                Runtime::mark(
                    &mut *nodes,
                    *child,
                    ReactiveNodeState::Dirty,
                    &mut *pending_effects,
                );
                Runtime::gather_descendants(
                    &subscribers,
                    *child,
                    &mut descendants,
                );
            }

            // mark descendants check
            for descendant in descendants {
                Runtime::mark(
                    &mut *nodes,
                    descendant,
                    ReactiveNodeState::Check,
                    &mut *pending_effects,
                );
            }
        }
    }

    fn mark(
        nodes: &mut SlotMap<NodeId, ReactiveNode>,
        node: NodeId,
        level: ReactiveNodeState,
        pending_effects: &mut Vec<NodeId>,
    ) {
        //eprintln!("marking {node:?} {level:?}");
        match nodes.get_mut(node) {
            Some(ReactiveNode {
                node_type: ReactiveNodeType::Memo { state, .. },
                ..
            }) => {
                if level > *state {
                    *state = level;
                }
            }
            Some(ReactiveNode {
                node_type: ReactiveNodeType::Effect(_),
                ..
            }) => {
                pending_effects.push(node);
            }
            _ => {}
        }
    }

    fn gather_descendants(
        subscribers: &SecondaryMap<NodeId, RefCell<HashSet<NodeId>>>,
        node: NodeId,
        descendants: &mut Vec<NodeId>,
    ) {
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
            for effect_id in effects {
                let node = { runtime.nodes.borrow().get(effect_id).cloned() };
                if let Some(ReactiveNode {
                    value,
                    node_type: ReactiveNodeType::Effect(effect),
                }) = node
                {
                    // clear previous dependencies
                    effect_id.cleanup(runtime);

                    // set this as the current observer
                    let prev_observer = runtime.observer.take();
                    runtime.observer.set(Some(effect_id));

                    // run the effect
                    effect.run(value);

                    // restore the previous observer
                    runtime.observer.set(prev_observer);
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
