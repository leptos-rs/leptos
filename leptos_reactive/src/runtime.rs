#[cfg(debug_assertions)]
use crate::SpecialNonReactiveZone;
use crate::{
    hydration::SharedContext,
    node::{
        Disposer, NodeId, ReactiveNode, ReactiveNodeState, ReactiveNodeType,
    },
    AnyComputation, AnyResource, EffectState, Memo, MemoState, ReadSignal,
    ResourceId, ResourceState, RwSignal, SerializableResource, StoredValueId,
    Trigger, UnserializableResource, WriteSignal,
};
use cfg_if::cfg_if;
use core::hash::BuildHasherDefault;
use futures::stream::FuturesUnordered;
use indexmap::IndexSet;
use pin_project::pin_project;
use rustc_hash::{FxHashMap, FxHasher};
use slotmap::{SecondaryMap, SlotMap, SparseSecondaryMap};
use std::{
    any::{Any, TypeId},
    cell::{Cell, RefCell},
    fmt::Debug,
    future::Future,
    marker::PhantomData,
    pin::Pin,
    rc::Rc,
    task::Poll,
};
use thiserror::Error;

pub(crate) type PinnedFuture<T> = Pin<Box<dyn Future<Output = T>>>;

cfg_if! {
    if #[cfg(any(feature = "csr", feature = "hydrate"))] {
        thread_local! {
            pub(crate) static RUNTIME: Runtime = Runtime::new();
        }
    } else {
        thread_local! {
            pub(crate) static RUNTIMES: RefCell<SlotMap<RuntimeId, Runtime>> = Default::default();

            pub(crate) static CURRENT_RUNTIME: Cell<Option<RuntimeId>> = Default::default();
        }
    }
}

// Stores the reactive runtime associated with the current Tokio task
#[cfg(feature = "ssr")]
tokio::task_local! {
    pub(crate) static TASK_RUNTIME: Option<RuntimeId>;
}

type FxIndexSet<T> = IndexSet<T, BuildHasherDefault<FxHasher>>;

// The data structure that owns all the signals, memos, effects,
// and other data included in the reactive system.
#[derive(Default)]
pub(crate) struct Runtime {
    pub shared_context: RefCell<SharedContext>,
    pub owner: Cell<Option<NodeId>>,
    pub observer: Cell<Option<NodeId>>,
    #[allow(clippy::type_complexity)]
    pub on_cleanups:
        RefCell<SparseSecondaryMap<NodeId, Vec<Box<dyn FnOnce()>>>>,
    pub stored_values: RefCell<SlotMap<StoredValueId, Rc<RefCell<dyn Any>>>>,
    pub nodes: RefCell<SlotMap<NodeId, ReactiveNode>>,
    pub node_subscribers:
        RefCell<SecondaryMap<NodeId, RefCell<FxIndexSet<NodeId>>>>,
    pub node_sources:
        RefCell<SecondaryMap<NodeId, RefCell<FxIndexSet<NodeId>>>>,
    pub node_owners: RefCell<SecondaryMap<NodeId, NodeId>>,
    pub node_properties:
        RefCell<SparseSecondaryMap<NodeId, Vec<ScopeProperty>>>,
    #[allow(clippy::type_complexity)]
    pub contexts:
        RefCell<SparseSecondaryMap<NodeId, FxHashMap<TypeId, Box<dyn Any>>>>,
    pub pending_effects: RefCell<Vec<NodeId>>,
    pub resources: RefCell<SlotMap<ResourceId, AnyResource>>,
    pub batching: Cell<bool>,
}

/// The current reactive runtime.
pub fn current_runtime() -> RuntimeId {
    Runtime::current()
}

/// Sets the current reactive runtime.
#[inline(always)]
#[allow(unused_variables)]
pub fn set_current_runtime(runtime: RuntimeId) {
    #[cfg(not(any(feature = "csr", feature = "hydrate")))]
    Runtime::set_runtime(Some(runtime));
}

/// A reactive owner.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Owner(pub(crate) NodeId);

impl Owner {
    /// Returns the current reactive owner.
    pub fn current() -> Option<Owner> {
        with_runtime(|runtime| runtime.owner.get())
            .ok()
            .flatten()
            .map(Owner)
    }

    /// Returns a unique handle for this owner for FFI purposes.
    pub fn as_ffi(&self) -> u64 {
        use slotmap::Key;

        self.0.data().as_ffi()
    }

    /// Parses a unique handler back into an owner.
    ///
    /// Iff `value` is value received from `k.as_ffi()`, returns a key equal to `k`.
    /// Otherwise the behavior is safe but unspecified.
    pub fn from_ffi(ffi: u64) -> Self {
        use slotmap::KeyData;

        Self(NodeId::from(KeyData::from_ffi(ffi)))
    }
}

// This core Runtime impl block handles all the work of marking and updating
// the reactive graph.
//
// In terms of concept and algorithm, this reactive-system implementation
// is significantly inspired by Reactively (https://github.com/modderme123/reactively)
impl Runtime {
    #[inline(always)]
    pub fn current() -> RuntimeId {
        cfg_if! {
            if #[cfg(any(feature = "csr", feature = "hydrate"))] {
                Default::default()
            } else if #[cfg(feature = "ssr")] {
                // either use the runtime associated with the current task,
                // or the current runtime
                TASK_RUNTIME.try_with(|trt| *trt)
                    .ok()
                    .flatten()
                    .unwrap_or_else(|| CURRENT_RUNTIME.with(|id| id.get()).unwrap_or_default())
            } else {
                CURRENT_RUNTIME.with(|id| id.get()).unwrap_or_default()
            }
        }
    }

    #[cfg(not(any(feature = "csr", feature = "hydrate")))]
    #[inline(always)]
    pub(crate) fn set_runtime(id: Option<RuntimeId>) {
        CURRENT_RUNTIME.with(|curr| curr.set(id))
    }

    pub(crate) fn update_if_necessary(&self, node_id: NodeId) {
        if self.current_state(node_id) == ReactiveNodeState::Check {
            let sources = {
                let sources = self.node_sources.borrow();

                // rather than cloning the entire FxIndexSet, only allocate a `Vec` for the node ids
                sources.get(node_id).map(|n| {
                    let sources = n.borrow();
                    // in case Vec::from_iterator specialization doesn't work, do it manually
                    let mut sources_vec = Vec::with_capacity(sources.len());
                    sources_vec.extend(sources.iter().cloned());
                    sources_vec
                })
            };

            for source in sources.into_iter().flatten() {
                self.update_if_necessary(source);
                if self.current_state(node_id) >= ReactiveNodeState::Dirty {
                    // as soon as a single parent has marked us dirty, we can
                    // stop checking them to avoid over-re-running
                    break;
                }
            }
        }

        // if we're dirty at this point, update
        if self.current_state(node_id) >= ReactiveNodeState::Dirty {
            self.cleanup_node(node_id);

            // now, update the value
            self.update(node_id);
        }

        // now we're clean
        self.mark_clean(node_id);
    }

    pub(crate) fn update(&self, node_id: NodeId) {
        let node = {
            let nodes = self.nodes.borrow();
            nodes.get(node_id).cloned()
        };

        if let Some(node) = node {
            // memos and effects rerun
            // signals simply have their value
            let changed = match node.node_type {
                ReactiveNodeType::Signal | ReactiveNodeType::Trigger => true,
                ReactiveNodeType::Memo { ref f }
                | ReactiveNodeType::Effect { ref f } => {
                    let value = node.value();
                    // set this node as the observer
                    self.with_observer(node_id, move || {
                        // clean up sources of this memo/effect
                        self.cleanup_sources(node_id);

                        f.run(value)
                    })
                }
            };

            // mark children dirty
            if changed {
                let subs = self.node_subscribers.borrow();

                if let Some(subs) = subs.get(node_id) {
                    let mut nodes = self.nodes.borrow_mut();
                    for sub_id in subs.borrow().iter() {
                        if let Some(sub) = nodes.get_mut(*sub_id) {
                            sub.state = ReactiveNodeState::Dirty;
                        }
                    }
                }
            }

            // mark clean
            self.mark_clean(node_id);
        }
    }

    pub(crate) fn dispose_node(&self, node_id: NodeId) {
        self.cleanup_node(node_id);

        // each of the subs needs to remove the node from its dependencies
        // so that it doesn't try to read the (now disposed) signal
        let subs = self.node_subscribers.borrow_mut().remove(node_id);
        if let Some(subs) = subs {
            let source_map = self.node_sources.borrow();
            for effect in subs.borrow().iter() {
                if let Some(effect_sources) = source_map.get(*effect) {
                    effect_sources.borrow_mut().swap_remove(&node_id);
                }
            }
        }

        self.node_sources.borrow_mut().remove(node_id);
        let node = { self.nodes.borrow_mut().remove(node_id) };
        drop(node);
    }
    fn cleanup_node(&self, node_id: NodeId) {
        self.run_on_cleanups(node_id);
        self.dispose_children(node_id);
    }
    /// Dispose of all of the children of the node recursively and completely.
    fn dispose_children(&self, node_id: NodeId) {
        let properties = { self.node_properties.borrow_mut().remove(node_id) };
        if let Some(properties) = properties {
            for property in properties {
                self.dispose_property(property);
            }
        }
    }
    fn dispose_property(&self, property: ScopeProperty) {
        // for signals, triggers, memos, effects, shared node cleanup
        match property {
            ScopeProperty::Signal(node)
            | ScopeProperty::Trigger(node)
            | ScopeProperty::Effect(node) => {
                self.dispose_node(node);
            }
            ScopeProperty::Resource(id) => {
                let value = self.resources.borrow_mut().remove(id);
                drop(value);
            }
            ScopeProperty::StoredValue(id) => {
                let value = self.stored_values.borrow_mut().remove(id);
                drop(value);
            }
        }
    }
    fn run_on_cleanups(&self, node_id: NodeId) {
        let c = { self.on_cleanups.borrow_mut().remove(node_id) };
        let prev_observer = self.observer.take(); // untrack around all cleanups
        if let Some(cleanups) = c {
            for cleanup in cleanups {
                cleanup();
            }
        }
        self.observer.set(prev_observer);
    }

    pub(crate) fn cleanup_sources(&self, node_id: NodeId) {
        let sources = self.node_sources.borrow();
        if let Some(sources) = sources.get(node_id) {
            let subs = self.node_subscribers.borrow();
            for source in sources.borrow().iter() {
                if let Some(source) = subs.get(*source) {
                    // Using `.shift_remove()` here guarantees that dependencies
                    // of a signal are always triggered in the same order.
                    // This is important for cases in which, for example, the first effect
                    // conditionally checks that the signal value is `Some(_)`, and the
                    // second one unwraps its value; if they maintain this order, then the check
                    // will always run first, and will cancel the unwrap if it is None. But if the
                    // order can be inverted (by using .swap_remove() here), the unwrap will
                    // run first on a subsequent run.
                    //
                    // Maintaining execution order is the intention of using an IndexSet here anyway,
                    // but using .swap_remove() would undermine that goal.
                    source.borrow_mut().shift_remove(&node_id);
                }
            }
        }
    }

    fn current_state(&self, node: NodeId) -> ReactiveNodeState {
        match self.nodes.borrow().get(node) {
            None => ReactiveNodeState::Clean,
            Some(node) => node.state,
        }
    }

    fn with_observer<T>(&self, observer: NodeId, f: impl FnOnce() -> T) -> T {
        // take previous observer and owner
        let prev_observer = self.observer.take();
        let prev_owner = self.owner.take();

        self.owner.set(Some(observer));
        self.observer.set(Some(observer));
        let v = f();
        self.observer.set(prev_observer);
        self.owner.set(prev_owner);
        v
    }

    fn mark_clean(&self, node: NodeId) {
        let mut nodes = self.nodes.borrow_mut();
        if let Some(node) = nodes.get_mut(node) {
            node.state = ReactiveNodeState::Clean;
        }
    }

    pub(crate) fn mark_dirty(&self, node: NodeId) {
        let mut nodes = self.nodes.borrow_mut();

        if let Some(current_node) = nodes.get_mut(node) {
            if current_node.state == ReactiveNodeState::DirtyMarked {
                return;
            }

            let mut pending_effects = self.pending_effects.borrow_mut();
            let subscribers = self.node_subscribers.borrow();
            let current_observer = self.observer.get();

            // mark self dirty
            Runtime::mark(
                node,
                current_node,
                ReactiveNodeState::Dirty,
                &mut pending_effects,
                current_observer,
            );

            /*
             * Depth-first DAG traversal that uses a stack of iterators instead of
             * buffering the entire to-visit list. Visited nodes are either marked as
             * `Check` or `DirtyMarked`.
             *
             * Because `RefCell`, borrowing the iterators all at once is difficult,
             * so a self-referential struct is used instead. self_cell produces safe
             * code, but it would not be recommended to use this outside of this
             * algorithm.
             */

            type Dependent<'a> = indexmap::set::Iter<'a, NodeId>;

            self_cell::self_cell! {
                struct RefIter<'a> {
                    owner: std::cell::Ref<'a, FxIndexSet<NodeId>>,

                    #[not_covariant] // avoids extra codegen, harmless to mark it as such
                    dependent: Dependent,
                }
            }

            /// Due to the limitations of self-referencing, we cannot borrow the
            /// stack and iter simultaneously within the closure or the loop,
            /// therefore this must be used to command the outside scope
            /// of what to do.
            enum IterResult<'a> {
                Continue,
                Empty,
                NewIter(RefIter<'a>),
            }

            let mut stack = Vec::new();

            if let Some(children) = subscribers.get(node) {
                stack.push(RefIter::new(children.borrow(), |children| {
                    children.iter()
                }));
            }

            while let Some(iter) = stack.last_mut() {
                let res = iter.with_dependent_mut(|_, iter| {
                    let Some(mut child) = iter.next().copied() else {
                        return IterResult::Empty;
                    };

                    while let Some(node) = nodes.get_mut(child) {
                        if node.state == ReactiveNodeState::Check
                            || node.state == ReactiveNodeState::DirtyMarked
                        {
                            return IterResult::Continue;
                        }

                        Runtime::mark(
                            child,
                            node,
                            ReactiveNodeState::Check,
                            &mut pending_effects,
                            current_observer,
                        );

                        if let Some(children) = subscribers.get(child) {
                            let children = children.borrow();

                            if !children.is_empty() {
                                // avoid going through an iterator in the simple pseudo-recursive case
                                if children.len() == 1 {
                                    child = children[0];
                                    continue;
                                }

                                return IterResult::NewIter(RefIter::new(
                                    children,
                                    |children| children.iter(),
                                ));
                            }
                        }

                        break;
                    }

                    IterResult::Continue
                });

                match res {
                    IterResult::Continue => continue,
                    IterResult::NewIter(iter) => stack.push(iter),
                    IterResult::Empty => {
                        stack.pop();
                    }
                }
            }
        }
    }

    #[inline(always)] // small function, used in hot loop
    fn mark(
        //nodes: &mut SlotMap<NodeId, ReactiveNode>,
        node_id: NodeId,
        node: &mut ReactiveNode,
        level: ReactiveNodeState,
        pending_effects: &mut Vec<NodeId>,
        current_observer: Option<NodeId>,
    ) {
        if level > node.state {
            node.state = level;
        }

        if matches!(node.node_type, ReactiveNodeType::Effect { .. } if current_observer != Some(node_id))
        {
            pending_effects.push(node_id)
        }

        if node.state == ReactiveNodeState::Dirty {
            node.state = ReactiveNodeState::DirtyMarked;
        }
    }

    pub(crate) fn run_effects(&self) {
        if !self.batching.get() {
            let effects = self.pending_effects.take();
            for effect_id in effects {
                self.update_if_necessary(effect_id);
            }
        }
    }

    #[track_caller]
    pub(crate) fn register_property(
        &self,
        property: ScopeProperty,
        #[cfg(debug_assertions)] defined_at: &'static std::panic::Location<
            'static,
        >,
    ) {
        let mut properties = self.node_properties.borrow_mut();
        if let Some(owner) = self.owner.get() {
            if let Some(entry) = properties.entry(owner) {
                let entry = entry.or_default();
                entry.push(property);
            }

            if let Some(node) = property.to_node_id() {
                let mut owners = self.node_owners.borrow_mut();
                owners.insert(node, owner);
            }
        } else {
            crate::macros::debug_warn!(
                "At {defined_at}, you are creating a reactive value outside \
                 the reactive root.",
            );
        }
    }

    pub(crate) fn get_context<T: Clone + 'static>(
        &self,
        node: NodeId,
        ty: TypeId,
    ) -> Option<T> {
        let contexts = self.contexts.borrow();

        let context = contexts.get(node);
        let local_value = context.and_then(|context| {
            context
                .get(&ty)
                .and_then(|val| val.downcast_ref::<T>())
                .cloned()
        });
        match local_value {
            Some(val) => Some(val),
            None => {
                #[cfg(all(
                    feature = "hydrate",
                    feature = "experimental-islands"
                ))]
                {
                    self.get_island_context(
                        self.shared_context
                            .borrow()
                            .islands
                            .get(&Owner(node))
                            .cloned(),
                        node,
                        ty,
                    )
                }
                #[cfg(not(all(
                    feature = "hydrate",
                    feature = "experimental-islands"
                )))]
                {
                    self.node_owners
                        .borrow()
                        .get(node)
                        .and_then(|parent| self.get_context(*parent, ty))
                }
            }
        }
    }

    #[cfg(all(feature = "hydrate", feature = "experimental-islands"))]
    pub(crate) fn get_island_context<T: Clone + 'static>(
        &self,
        el: Option<web_sys::HtmlElement>,
        node: NodeId,
        ty: TypeId,
    ) -> Option<T> {
        let contexts = self.contexts.borrow();

        let context = contexts.get(node);
        let local_value = context.and_then(|context| {
            context
                .get(&ty)
                .and_then(|val| val.downcast_ref::<T>())
                .cloned()
        });

        match (el, local_value) {
            (_, Some(val)) => Some(val),
            // if we're already in the island's scope, island-hop
            (Some(el), None) => {
                use js_sys::Reflect;
                use wasm_bindgen::{intern, JsCast, JsValue};
                let parent_el = el
                    .parent_element()
                    .expect("to have parent")
                    .unchecked_ref::<web_sys::HtmlElement>()
                    .closest("leptos-children")
                    .expect("to find island")
                    //.flatten()
                    .and_then(|el| el.dyn_into::<web_sys::HtmlElement>().ok());
                match parent_el
                    .clone()
                    .and_then(|el| {
                        Reflect::get(&el, &JsValue::from_str(intern("$$owner")))
                            .ok()
                    })
                    .and_then(|value| u64::try_from(value).ok())
                    .map(Owner::from_ffi)
                {
                    Some(owner) => {
                        self.get_island_context(parent_el, owner.0, ty)
                    }
                    None => None,
                }
            }
            // otherwise, check for a parent scope
            (None, None) => {
                self.node_owners.borrow().get(node).and_then(|parent| {
                    self.get_island_context(
                        self.shared_context
                            .borrow()
                            .islands
                            .get(&Owner(*parent))
                            .cloned(),
                        *parent,
                        ty,
                    )
                })
            }
        }
    }

    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(level = "trace", skip_all,)
    )]
    #[track_caller]
    pub(crate) fn push_scope_property(&self, prop: ScopeProperty) {
        #[cfg(debug_assertions)]
        let defined_at = std::panic::Location::caller();
        self.register_property(
            prop,
            #[cfg(debug_assertions)]
            defined_at,
        );
    }

    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(level = "trace", skip_all,)
    )]
    #[track_caller]
    pub(crate) fn remove_scope_property(
        &self,
        owner: NodeId,
        property: ScopeProperty,
    ) {
        let mut properties = self.node_properties.borrow_mut();
        if let Some(properties) = properties.get_mut(owner) {
            // remove this property from the list, if found
            if let Some(index) = properties.iter().position(|p| p == &property)
            {
                // order of properties doesn't matter so swap_remove
                // is the most efficient way to remove
                properties.swap_remove(index);
            }
        }

        if let Some(node) = property.to_node_id() {
            let mut owners = self.node_owners.borrow_mut();
            owners.remove(node);
        }
    }
}

impl Debug for Runtime {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Runtime").finish()
    }
}

/// Get the selected runtime from the thread-local set of runtimes. On the server,
/// this will return the correct runtime. In the browser, there should only be one runtime.
#[cfg_attr(
    any(debug_assertions, feature = "ssr"),
    instrument(level = "trace", skip_all,)
)]
#[inline(always)] // it monomorphizes anyway
pub(crate) fn with_runtime<T>(
    f: impl FnOnce(&Runtime) -> T,
) -> Result<T, ReactiveSystemError> {
    // in the browser, everything should exist under one runtime
    cfg_if! {
        if #[cfg(any(feature = "csr", feature = "hydrate"))] {
            Ok(RUNTIME.with(|runtime| f(runtime)))
        } else {
            RUNTIMES.with(|runtimes| {
                let runtimes = runtimes.borrow();
                let rt = Runtime::current();
                match runtimes.get(rt) {
                    None => Err(ReactiveSystemError::RuntimeDisposed(rt)),
                    Some(runtime) => Ok(f(runtime))
                }
            })
        }
    }
}

#[must_use = "Runtime will leak memory if Runtime::dispose() is never called."]
/// Creates a new reactive runtime and sets it as the current runtime.
///
/// This should almost always be handled by the framework, not called directly in user code.
pub fn create_runtime() -> RuntimeId {
    cfg_if! {
        if #[cfg(any(feature = "csr", feature = "hydrate"))] {
            Default::default()
        } else {
            let id = RUNTIMES.with(|runtimes| runtimes.borrow_mut().insert(Runtime::new()));
            Runtime::set_runtime(Some(id));

            id
        }
    }
}

#[cfg(not(any(feature = "csr", feature = "hydrate")))]
slotmap::new_key_type! {
    /// Unique ID assigned to a Runtime.
    pub struct RuntimeId;
}

/// Unique ID assigned to a Runtime.
#[cfg(any(feature = "csr", feature = "hydrate"))]
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RuntimeId;

/// Wraps the given function so that, whenever it is called, it creates
/// a child node owned by whichever reactive node was the owner
/// when it was created, runs the function, and returns a disposer that
/// can be used to dispose of the child later.
///
/// This can be used to hoist children created inside an effect up to
/// the level of a higher parent, to prevent each one from being disposed
/// every time the effect within which they're created is run.
///
/// For example, each row in a `<For/>` component could be created using this,
/// so that they are owned by the `<For/>` component itself, not an effect
/// running within it.
///
/// ## Panics
/// Panics if there is no current reactive runtime.
pub fn as_child_of_current_owner<T, U>(
    f: impl Fn(T) -> U,
) -> impl Fn(T) -> (U, Disposer)
where
    T: 'static,
{
    let owner = with_runtime(|runtime| runtime.owner.get())
        .expect("runtime should be alive when created");
    move |t| {
        with_runtime(|runtime| {
            let prev_observer = runtime.observer.take();
            let prev_owner = runtime.owner.take();

            runtime.owner.set(owner);
            runtime.observer.set(owner);

            let id = runtime.nodes.borrow_mut().insert(ReactiveNode {
                value: None,
                state: ReactiveNodeState::Clean,
                node_type: ReactiveNodeType::Trigger,
            });
            runtime.push_scope_property(ScopeProperty::Trigger(id));
            let disposer = Disposer(id);

            runtime.owner.set(Some(id));
            runtime.observer.set(Some(id));

            let v = f(t);

            runtime.observer.set(prev_observer);
            runtime.owner.set(prev_owner);

            (v, disposer)
        })
        .expect("runtime should be alive when run")
    }
}

/// Wraps the given function so that, whenever it is called, it is run
/// in the reactive scope of whatever the reactive owner was when it was
/// created.
///
/// ## Panics
/// Panics if there is no current reactive runtime.
pub fn with_current_owner<T, U>(f: impl Fn(T) -> U + 'static) -> impl Fn(T) -> U
where
    T: 'static,
{
    let owner = with_runtime(|runtime| runtime.owner.get())
        .expect("runtime should be alive when created");
    move |t| {
        with_runtime(|runtime| {
            let prev_observer = runtime.observer.take();
            let prev_owner = runtime.owner.take();

            runtime.owner.set(owner);
            runtime.observer.set(owner);

            let v = f(t);

            runtime.observer.set(prev_observer);
            runtime.owner.set(prev_owner);

            v
        })
        .expect("runtime should be alive when run")
    }
}

/// Runs the given code with the given reactive owner.
///
/// ## Panics
/// Panics if there is no current reactive runtime.
pub fn with_owner<T>(owner: Owner, f: impl FnOnce() -> T) -> T {
    try_with_owner(owner, f).unwrap()
}

#[derive(Error, Debug)]
pub enum ReactiveSystemError {
    #[error("Runtime {0:?} has been disposed.")]
    RuntimeDisposed(RuntimeId),
    #[error("Owner {0:?} has been disposed.")]
    OwnerDisposed(Owner),
    #[error("Error borrowing runtime.nodes {0:?}")]
    Borrow(std::cell::BorrowError),
}

/// Runs the given code with the given reactive owner.
pub fn try_with_owner<T>(
    owner: Owner,
    f: impl FnOnce() -> T,
) -> Result<T, ReactiveSystemError> {
    with_runtime(|runtime| {
        let scope_exists = {
            let nodes = runtime
                .nodes
                .try_borrow()
                .map_err(ReactiveSystemError::Borrow)?;
            nodes.contains_key(owner.0)
        };
        if scope_exists {
            let prev_observer = runtime.observer.take();
            let prev_owner = runtime.owner.take();

            runtime.owner.set(Some(owner.0));
            runtime.observer.set(Some(owner.0));

            let v = f();

            runtime.observer.set(prev_observer);
            runtime.owner.set(prev_owner);

            Ok(v)
        } else {
            Err(ReactiveSystemError::OwnerDisposed(owner))
        }
    })?
}

/// Runs the given function as a child of the current Owner, once.
pub fn run_as_child<T>(f: impl FnOnce() -> T + 'static) -> T {
    let owner = with_runtime(|runtime| runtime.owner.get())
        .expect("runtime should be alive when created");
    let (value, disposer) = with_runtime(|runtime| {
        let prev_observer = runtime.observer.take();
        let prev_owner = runtime.owner.take();

        runtime.owner.set(owner);
        runtime.observer.set(owner);

        let id = runtime.nodes.borrow_mut().insert(ReactiveNode {
            value: None,
            state: ReactiveNodeState::Clean,
            node_type: ReactiveNodeType::Trigger,
        });
        runtime.push_scope_property(ScopeProperty::Trigger(id));
        let disposer = Disposer(id);

        runtime.owner.set(Some(id));
        runtime.observer.set(Some(id));

        let v = f();

        runtime.observer.set(prev_observer);
        runtime.owner.set(prev_owner);

        (v, disposer)
    })
    .expect("runtime should be alive when run");
    on_cleanup(move || drop(disposer));
    value
}

impl RuntimeId {
    /// Removes the runtime, disposing of everything created in it.
    ///
    /// ## Panics
    /// Panics if the reactive runtime you’re trying to dispose is not found.
    /// This would suggest either that you’re trying to dispose of it twice, or
    /// that it was created in a different thread; panicking here indicates a
    /// memory leak.
    pub fn dispose(self) {
        cfg_if! {
            if #[cfg(not(any(feature = "csr", feature = "hydrate")))] {
                // remove this from the set of runtimes
                let runtime = RUNTIMES.with(move |runtimes| runtimes.borrow_mut().remove(self))
                    .expect("Attempted to dispose of a reactive runtime that was not found. This suggests \
                    a possible memory leak. Please open an issue with details at https://github.com/leptos-rs/leptos");

                // remove this from being the current runtime
                CURRENT_RUNTIME.with(|runtime| {
                    if runtime.get() == Some(self) {
                        runtime.take();
                    }
                });

                drop(runtime);
            }
        }
    }

    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(level = "trace", skip_all,)
    )]
    #[inline(always)]
    pub(crate) fn untrack<T>(
        self,
        f: impl FnOnce() -> T,
        #[allow(unused)] diagnostics: bool,
    ) -> T {
        with_runtime(|runtime| {
            let untracked_result;

            #[cfg(debug_assertions)]
            let prev = if !diagnostics {
                SpecialNonReactiveZone::enter()
            } else {
                false
            };

            let prev_observer = SetObserverOnDrop(runtime.observer.take());

            untracked_result = f();

            runtime.observer.set(prev_observer.0);
            std::mem::forget(prev_observer); // avoid Drop

            #[cfg(debug_assertions)]
            if !diagnostics {
                SpecialNonReactiveZone::exit(prev);
            }

            untracked_result
        })
        .expect(
            "tried to run untracked function in a runtime that has been \
             disposed",
        )
    }

    #[track_caller]
    #[inline(always)] // only because it's placed here to fit in with the other create methods
    pub(crate) fn create_trigger(self) -> Trigger {
        let id = with_runtime(|runtime| {
            let id = runtime.nodes.borrow_mut().insert(ReactiveNode {
                value: None,
                state: ReactiveNodeState::Clean,
                node_type: ReactiveNodeType::Trigger,
            });
            runtime.push_scope_property(ScopeProperty::Trigger(id));
            id
        })
        .expect(
            "tried to create a trigger in a runtime that has been disposed",
        );

        Trigger {
            id,
            #[cfg(debug_assertions)]
            defined_at: std::panic::Location::caller(),
        }
    }

    pub(crate) fn create_concrete_signal(
        self,
        value: Rc<RefCell<dyn Any>>,
    ) -> NodeId {
        with_runtime(|runtime| {
            let id = runtime.nodes.borrow_mut().insert(ReactiveNode {
                value: Some(value),
                state: ReactiveNodeState::Clean,
                node_type: ReactiveNodeType::Signal,
            });
            runtime.push_scope_property(ScopeProperty::Signal(id));
            id
        })
        .expect("tried to create a signal in a runtime that has been disposed")
    }

    #[track_caller]
    #[inline(always)]
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
                id,
                ty: PhantomData,
                #[cfg(any(debug_assertions, feature = "ssr"))]
                defined_at: std::panic::Location::caller(),
            },
            WriteSignal {
                id,
                ty: PhantomData,
                #[cfg(any(debug_assertions, feature = "ssr"))]
                defined_at: std::panic::Location::caller(),
            },
        )
    }

    #[track_caller]
    #[inline(always)]
    pub(crate) fn create_rw_signal<T>(self, value: T) -> RwSignal<T>
    where
        T: Any + 'static,
    {
        let id = self.create_concrete_signal(
            Rc::new(RefCell::new(value)) as Rc<RefCell<dyn Any>>
        );
        RwSignal {
            id,
            ty: PhantomData,
            #[cfg(any(debug_assertions, feature = "ssr"))]
            defined_at: std::panic::Location::caller(),
        }
    }

    pub(crate) fn create_concrete_effect(
        self,
        value: Rc<RefCell<dyn Any>>,
        effect: Rc<dyn AnyComputation>,
    ) -> NodeId {
        with_runtime(|runtime| {
            let id = runtime.nodes.borrow_mut().insert(ReactiveNode {
                value: Some(Rc::clone(&value)),
                state: ReactiveNodeState::Dirty,
                node_type: ReactiveNodeType::Effect {
                    f: Rc::clone(&effect),
                },
            });
            runtime.push_scope_property(ScopeProperty::Effect(id));
            id
        })
        .expect("tried to create an effect in a runtime that has been disposed")
    }

    pub(crate) fn create_concrete_memo(
        self,
        value: Rc<RefCell<dyn Any>>,
        computation: Rc<dyn AnyComputation>,
    ) -> NodeId {
        with_runtime(|runtime| {
            let id = runtime.nodes.borrow_mut().insert(ReactiveNode {
                value: Some(value),
                // memos are lazy, so are dirty when created
                // will be run the first time we ask for it
                state: ReactiveNodeState::Dirty,
                node_type: ReactiveNodeType::Memo { f: computation },
            });
            runtime.push_scope_property(ScopeProperty::Effect(id));
            id
        })
        .expect("tried to create a memo in a runtime that has been disposed")
    }

    #[track_caller]
    #[inline(always)]
    pub(crate) fn create_effect<T>(
        self,
        f: impl Fn(Option<T>) -> T + 'static,
    ) -> NodeId
    where
        T: Any + 'static,
    {
        self.create_concrete_effect(
            Rc::new(RefCell::new(None::<T>)),
            Rc::new(EffectState {
                f,
                ty: PhantomData,
                #[cfg(any(debug_assertions, feature = "ssr"))]
                defined_at: std::panic::Location::caller(),
            }),
        )
    }

    pub(crate) fn watch<W, T>(
        self,
        deps: impl Fn() -> W + 'static,
        callback: impl Fn(&W, Option<&W>, Option<T>) -> T + Clone + 'static,
        immediate: bool,
    ) -> (NodeId, impl Fn() + Clone)
    where
        W: Clone + 'static,
        T: 'static,
    {
        let cur_deps_value = Rc::new(RefCell::new(None::<W>));
        let prev_deps_value = Rc::new(RefCell::new(None::<W>));
        let prev_callback_value = Rc::new(RefCell::new(None::<T>));

        let wrapped_callback = {
            let cur_deps_value = Rc::clone(&cur_deps_value);
            let prev_deps_value = Rc::clone(&prev_deps_value);
            let prev_callback_value = Rc::clone(&prev_callback_value);

            move || {
                callback(
                    cur_deps_value.borrow().as_ref().expect(
                        "this will not be called before there is deps value",
                    ),
                    prev_deps_value.borrow().as_ref(),
                    prev_callback_value.take(),
                )
            }
        };

        let effect_fn = {
            let prev_callback_value = Rc::clone(&prev_callback_value);
            move |did_run_before: Option<()>| {
                let deps_value = deps();

                let did_run_before = did_run_before.is_some();

                if !immediate && !did_run_before {
                    prev_deps_value.replace(Some(deps_value));
                    return;
                }

                cur_deps_value.replace(Some(deps_value.clone()));

                let callback_value =
                    Some(self.untrack(wrapped_callback.clone(), false));

                prev_callback_value.replace(callback_value);

                prev_deps_value.replace(Some(deps_value));
            }
        };

        let id = self.create_concrete_effect(
            Rc::new(RefCell::new(None::<()>)),
            Rc::new(EffectState {
                f: effect_fn,
                ty: PhantomData,
                #[cfg(any(debug_assertions, feature = "ssr"))]
                defined_at: std::panic::Location::caller(),
            }),
        );

        (id, move || {
            with_runtime(|runtime| runtime.dispose_node(id)).expect(
                "tried to stop a watch in a runtime that has been disposed",
            );
        })
    }

    #[track_caller]
    #[inline(always)]
    pub(crate) fn create_owning_memo<T>(
        self,
        f: impl Fn(Option<T>) -> (T, bool) + 'static,
    ) -> Memo<T>
    where
        T: 'static,
    {
        Memo {
            id: self.create_concrete_memo(
                Rc::new(RefCell::new(None::<T>)),
                Rc::new(MemoState {
                    f,
                    t: PhantomData,
                    #[cfg(any(debug_assertions, feature = "ssr"))]
                    defined_at: std::panic::Location::caller(),
                }),
            ),
            ty: PhantomData,
            #[cfg(any(debug_assertions, feature = "ssr"))]
            defined_at: std::panic::Location::caller(),
        }
    }
}

impl Runtime {
    pub fn new() -> Self {
        let root = ReactiveNode {
            value: None,
            state: ReactiveNodeState::Clean,
            node_type: ReactiveNodeType::Trigger,
        };
        let mut nodes: SlotMap<NodeId, ReactiveNode> = SlotMap::default();
        let root_id = nodes.insert(root);

        Self {
            owner: Cell::new(Some(root_id)),
            nodes: RefCell::new(nodes),
            ..Self::default()
        }
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

    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(level = "trace", skip_all,)
    )]
    #[track_caller]
    pub(crate) fn resource<S, T, U>(
        &self,
        id: ResourceId,
        f: impl FnOnce(&ResourceState<S, T>) -> U,
    ) -> U
    where
        S: 'static,
        T: 'static,
    {
        self.try_resource(id, f).unwrap_or_else(|| {
            panic!(
                "couldn't locate {id:?} at {:?}",
                std::panic::Location::caller()
            );
        })
    }

    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(level = "trace", skip_all,)
    )]
    #[track_caller]
    pub(crate) fn try_resource<S, T, U>(
        &self,
        id: ResourceId,
        f: impl FnOnce(&ResourceState<S, T>) -> U,
    ) -> Option<U>
    where
        S: 'static,
        T: 'static,
    {
        let resources = { self.resources.borrow().clone() };
        let res = resources.get(id);
        res.map(|res| {
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
        })
    }

    /// Returns IDs for all [`Resource`](crate::Resource)s found on any scope.
    pub(crate) fn all_resources(&self) -> Vec<ResourceId> {
        self.resources
            .borrow()
            .iter()
            .map(|(resource_id, _)| resource_id)
            .collect()
    }

    /// Returns IDs for all [`Resource`](crate::Resource)s found on any
    /// scope, pending from the server.
    pub(crate) fn pending_resources(&self) -> Vec<ResourceId> {
        self.resources
            .borrow()
            .iter()
            .filter_map(|(resource_id, res)| {
                if let AnyResource::Serializable(res) = res {
                    res.should_send_to_client().then_some(resource_id)
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
        let resources = { self.resources.borrow().clone() };
        for (id, resource) in resources.iter() {
            if let AnyResource::Serializable(resource) = resource {
                if resource.should_send_to_client() {
                    f.push(resource.to_serialization_resolver(id));
                }
            }
        }
        f
    }

    /// Do not call on triggers
    pub(crate) fn get_value(
        &self,
        node_id: NodeId,
    ) -> Option<Rc<RefCell<dyn Any>>> {
        let signals = self.nodes.borrow();
        signals.get(node_id).map(|node| node.value())
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

struct SetObserverOnDrop(Option<NodeId>);

impl Drop for SetObserverOnDrop {
    fn drop(&mut self) {
        _ = with_runtime(|rt| {
            rt.observer.set(self.0);
        });
    }
}

/// Batches any reactive updates, preventing effects from running until the whole
/// function has run. This allows you to prevent rerunning effects if multiple
/// signal updates might cause the same effect to run.
///
/// # Panics
/// Panics if the runtime has already been disposed.
///
/// To avoid panicking under any circumstances, use [`try_batch`].
#[cfg_attr(
    any(debug_assertions, feature = "ssr"),
    instrument(level = "trace", skip_all,)
)]
#[inline(always)]
pub fn batch<T>(f: impl FnOnce() -> T) -> T {
    try_batch(f).expect(
        "tried to run a batched update in a runtime that has been disposed",
    )
}

/// Attempts to batch any reactive updates, preventing effects from running until the whole
/// function has run. This allows you to prevent rerunning effects if multiple
/// signal updates might cause the same effect to run.
///
/// Unlike [`batch`], this will not panic if the runtime has been disposed.
#[cfg_attr(
    any(debug_assertions, feature = "ssr"),
    instrument(level = "trace", skip_all,)
)]
#[inline(always)]
pub fn try_batch<T>(f: impl FnOnce() -> T) -> Result<T, ReactiveSystemError> {
    with_runtime(move |runtime| {
        let batching = SetBatchingOnDrop(runtime.batching.get());
        runtime.batching.set(true);

        let val = f();

        runtime.batching.set(batching.0);
        std::mem::forget(batching);

        runtime.run_effects();
        val
    })
}

struct SetBatchingOnDrop(bool);

impl Drop for SetBatchingOnDrop {
    fn drop(&mut self) {
        _ = with_runtime(|rt| {
            rt.batching.set(self.0);
        });
    }
}

/// Creates a cleanup function, which will be run when the current reactive owner is disposed.
///
/// It runs after child nodes have been disposed, but before signals, effects, and resources
/// are invalidated.
#[inline(always)]
pub fn on_cleanup(cleanup_fn: impl FnOnce() + 'static) {
    #[cfg(debug_assertions)]
    let cleanup_fn = move || {
        #[cfg(debug_assertions)]
        let prev = crate::SpecialNonReactiveZone::enter();
        cleanup_fn();
        #[cfg(debug_assertions)]
        {
            crate::SpecialNonReactiveZone::exit(prev);
        }
    };
    push_cleanup(Box::new(cleanup_fn))
}

#[cfg_attr(
    any(debug_assertions, feature = "ssr"),
    instrument(level = "trace", skip_all,)
)]
fn push_cleanup(cleanup_fn: Box<dyn FnOnce()>) {
    _ = with_runtime(|runtime| {
        if let Some(owner) = runtime.owner.get() {
            let mut cleanups = runtime.on_cleanups.borrow_mut();
            if let Some(entries) = cleanups.get_mut(owner) {
                entries.push(cleanup_fn);
            } else {
                cleanups.insert(owner, vec![cleanup_fn]);
            }
        }
    });
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScopeProperty {
    Trigger(NodeId),
    Signal(NodeId),
    Effect(NodeId),
    Resource(ResourceId),
    StoredValue(StoredValueId),
}

impl ScopeProperty {
    pub fn to_node_id(self) -> Option<NodeId> {
        match self {
            Self::Trigger(node) | Self::Signal(node) | Self::Effect(node) => {
                Some(node)
            }
            _ => None,
        }
    }
}

/// Suspends reactive tracking while running the given function.
///
/// This can be used to isolate parts of the reactive graph from one another.
///
/// ```rust
/// # use leptos_reactive::*;
/// # let runtime = create_runtime();
/// let (a, set_a) = create_signal(0);
/// let (b, set_b) = create_signal(0);
/// let c = create_memo(move |_| {
///     // this memo will *only* update when `a` changes
///     a.get() + untrack(move || b.get())
/// });
///
/// assert_eq!(c.get(), 0);
/// set_a.set(1);
/// assert_eq!(c.get(), 1);
/// set_b.set(1);
/// // hasn't updated, because we untracked before reading b
/// assert_eq!(c.get(), 1);
/// set_a.set(2);
/// assert_eq!(c.get(), 3);
///
/// # runtime.dispose();
/// ```
#[cfg_attr(
    any(debug_assertions, feature = "ssr"),
    instrument(level = "trace", skip_all,)
)]
#[inline(always)]
pub fn untrack<T>(f: impl FnOnce() -> T) -> T {
    Runtime::current().untrack(f, false)
}

#[doc(hidden)]
#[cfg_attr(
    any(debug_assertions, feature = "ssr"),
    instrument(level = "trace", skip_all,)
)]
#[inline(always)]
pub fn untrack_with_diagnostics<T>(f: impl FnOnce() -> T) -> T {
    Runtime::current().untrack(f, true)
}

/// Allows running a future that has access to a given scope.
#[pin_project]
pub struct ScopedFuture<Fut: Future> {
    owner: Owner,
    #[pin]
    future: Fut,
}

/// Errors that can occur when trying to spawn a [`ScopedFuture`].
#[derive(Error, Debug, Clone)]
pub enum ScopedFutureError {
    #[error(
        "Tried to spawn a scoped Future without a current reactive Owner."
    )]
    NoCurrentOwner,
}

impl<Fut: Future + 'static> Future for ScopedFuture<Fut> {
    type Output = Option<Fut::Output>;

    fn poll(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Self::Output> {
        let this = self.project();

        if let Ok(poll) = try_with_owner(*this.owner, || this.future.poll(cx)) {
            match poll {
                Poll::Ready(res) => Poll::Ready(Some(res)),
                Poll::Pending => Poll::Pending,
            }
        } else {
            Poll::Ready(None)
        }
    }
}

impl<Fut: Future> ScopedFuture<Fut> {
    /// Creates a new future that will have access to the `[Owner]`'s
    /// scope context.
    pub fn new(owner: Owner, fut: Fut) -> Self {
        Self { owner, future: fut }
    }

    /// Runs the future in the current [`Owner`]'s scope context.
    #[track_caller]
    pub fn new_current(fut: Fut) -> Result<Self, ScopedFutureError> {
        Owner::current()
            .map(|owner| Self { owner, future: fut })
            .ok_or(ScopedFutureError::NoCurrentOwner)
    }
}

/// Runs a future that has access to the provided [`Owner`]'s
/// scope context.
#[track_caller]
pub fn spawn_local_with_owner(
    owner: Owner,
    fut: impl Future<Output = ()> + 'static,
) {
    let scoped_future = ScopedFuture::new(owner, fut);
    #[cfg(debug_assertions)]
    let loc = std::panic::Location::caller();

    crate::spawn_local(async move {
        if scoped_future.await.is_none() {
            crate::macros::debug_warn!(
                "`spawn_local_with_owner` called at {loc} returned `None`, \
                 i.e., its Owner was disposed before the `Future` resolved."
            );
        }
    });
}

/// Runs a future that has access to the provided [`Owner`]'s
/// scope context.
///
/// # Panics
/// Panics if there is no [`Owner`] context available.
#[track_caller]
pub fn spawn_local_with_current_owner(
    fut: impl Future<Output = ()> + 'static,
) -> Result<(), ScopedFutureError> {
    let scoped_future = ScopedFuture::new_current(fut)?;
    #[cfg(debug_assertions)]
    let loc = std::panic::Location::caller();

    crate::spawn_local(async move {
        if scoped_future.await.is_none() {
            crate::macros::debug_warn!(
                "`spawn_local_with_owner` called at {loc} returned `None`, \
                 i.e., its Owner was disposed before the `Future` resolved."
            );
        }
    });

    Ok(())
}

/// Runs a future that has access to the provided [`Owner`]'s
/// scope context.
///
/// Since futures run in the background, it is possible that
/// the scope has been cleaned up since the future started running.
/// If this happens, the future will not be completed.
///
/// The `on_cancelled` callback can be used to notify you that the
/// future was cancelled.
pub fn try_spawn_local_with_owner(
    owner: Owner,
    fut: impl Future<Output = ()> + 'static,
    on_cancelled: impl FnOnce() + 'static,
) {
    let scoped_future = ScopedFuture::new(owner, fut);

    crate::spawn_local(async move {
        if scoped_future.await.is_none() {
            on_cancelled();
        }
    });
}

/// Runs a future that has access to the provided [`Owner`]'s
/// scope context.
///
/// Since futures run in the background, it is possible that
/// the scope has been cleaned up since the future started running.
/// If this happens, the future will not be completed.
///
/// The `on_cancelled` callback can be used to notify you that the
/// future was cancelled.
///
/// # Panics
/// Panics if there is no [`Owner`] context available.
#[track_caller]
pub fn try_spawn_local_with_current_owner(
    fut: impl Future<Output = ()> + 'static,
    on_cancelled: impl FnOnce() + 'static,
) -> Result<(), ScopedFutureError> {
    let scoped_future = ScopedFuture::new_current(fut)?;

    crate::spawn_local(async move {
        if scoped_future.await.is_none() {
            on_cancelled();
        }
    });

    Ok(())
}
