use super::OWNER;
use or_poisoned::OrPoisoned;
use slotmap::{new_key_type, SlotMap};
#[cfg(not(feature = "sandboxed-arenas"))]
use std::sync::OnceLock;
use std::{any::Any, hash::Hash, marker::PhantomData, sync::RwLock};
#[cfg(feature = "sandboxed-arenas")]
use std::{cell::RefCell, sync::Arc};

new_key_type! { pub(crate) struct NodeId; }

pub(crate) struct Arena;

type ArenaMap = SlotMap<NodeId, Box<dyn Any + Send + Sync>>;

#[cfg(not(feature = "sandboxed-arenas"))]
static MAP: OnceLock<RwLock<ArenaMap>> = OnceLock::new();
#[cfg(feature = "sandboxed-arenas")]
thread_local! {
    pub(crate) static MAP: RefCell<Option<Arc<RwLock<ArenaMap>>>> = RefCell::new(None);
}

impl Arena {
    #[inline(always)]
    pub fn enter_new() {
        #[cfg(feature = "sandboxed-arenas")]
        MAP.with_borrow_mut(|arena| {
            *arena =
                Some(Arc::new(RwLock::new(SlotMap::with_capacity_and_key(32))))
        })
    }

    #[track_caller]
    pub fn with<U>(fun: impl FnOnce(&ArenaMap) -> U) -> U {
        #[cfg(not(feature = "sandboxed-arenas"))]
        {
            fun(&MAP.get_or_init(Default::default).read().or_poisoned())
        }
        #[cfg(feature = "sandboxed-arenas")]
        {
            MAP.with_borrow(|arena| {
                fun(&arena
                    .as_ref()
                    .unwrap_or_else(|| {
                        panic!(
                            "at {}, the `sandboxed-arenas` feature is active, \
                             but no Arena is active",
                            std::panic::Location::caller()
                        )
                    })
                    .read()
                    .or_poisoned())
            })
        }
    }

    #[track_caller]
    pub fn with_mut<U>(fun: impl FnOnce(&mut ArenaMap) -> U) -> U {
        #[cfg(not(feature = "sandboxed-arenas"))]
        {
            fun(&mut MAP.get_or_init(Default::default).write().or_poisoned())
        }
        #[cfg(feature = "sandboxed-arenas")]
        {
            let caller = std::panic::Location::caller();
            MAP.with_borrow(|arena| {
                fun(&mut arena
                    .as_ref()
                    .unwrap_or_else(|| {
                        panic!(
                            "at {}, the `sandboxed-arenas` feature is active, \
                             but no Arena is active",
                            caller
                        )
                    })
                    .write()
                    .or_poisoned())
            })
        }
    }
}

#[cfg(feature = "sandboxed-arenas")]
pub mod sandboxed {
    use super::{Arena, ArenaMap, MAP};
    use futures::Stream;
    use pin_project_lite::pin_project;
    use std::{
        future::Future,
        mem,
        pin::Pin,
        sync::{Arc, RwLock},
        task::{Context, Poll},
    };

    impl Arena {
        fn set(new_arena: Arc<RwLock<ArenaMap>>) -> UnsetArenaOnDrop {
            MAP.with_borrow_mut(|arena| {
                UnsetArenaOnDrop(mem::replace(arena, Some(new_arena)))
            })
        }
    }

    pin_project! {
        pub struct Sandboxed<T> {
            arena: Arc<RwLock<ArenaMap>>,
            #[pin]
            inner: T,
        }
    }

    impl<T> Sandboxed<T> {
        pub fn new(inner: T) -> Self {
            let arena = MAP.with_borrow(|current| {
                Arc::clone(current.as_ref().expect(
                    "the `sandboxed-arenas` feature is active, but no Arena \
                     is active",
                ))
            });
            Self { arena, inner }
        }
    }

    impl<Fut> Future for Sandboxed<Fut>
    where
        Fut: Future,
    {
        type Output = Fut::Output;

        fn poll(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
        ) -> Poll<Self::Output> {
            let unset = Arena::set(Arc::clone(&self.arena));
            let this = self.project();
            let res = this.inner.poll(cx);
            drop(unset);
            res
        }
    }

    impl<T> Stream for Sandboxed<T>
    where
        T: Stream,
    {
        type Item = T::Item;

        fn poll_next(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
        ) -> Poll<Option<Self::Item>> {
            let unset = Arena::set(Arc::clone(&self.arena));
            let this = self.project();
            let res = this.inner.poll_next(cx);
            drop(unset);
            res
        }
    }

    #[derive(Debug)]
    struct UnsetArenaOnDrop(Option<Arc<RwLock<ArenaMap>>>);

    impl Drop for UnsetArenaOnDrop {
        fn drop(&mut self) {
            if let Some(inner) = self.0.take() {
                MAP.with_borrow_mut(|current_map| *current_map = Some(inner));
            }
        }
    }
}

#[derive(Debug)]
pub struct StoredValue<T> {
    node: NodeId,
    ty: PhantomData<T>,
}

impl<T> Copy for StoredValue<T> {}

impl<T> Clone for StoredValue<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> PartialEq for StoredValue<T> {
    fn eq(&self, other: &Self) -> bool {
        self.node == other.node && self.ty == other.ty
    }
}

impl<T> Eq for StoredValue<T> {}

impl<T> Hash for StoredValue<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.node.hash(state);
        self.ty.hash(state);
    }
}

impl<T> StoredValue<T>
where
    T: Send + Sync + 'static,
{
    #[track_caller]
    pub fn new(value: T) -> Self {
        let node = {
            Arena::with_mut(|arena| {
                arena.insert(Box::new(value) as Box<dyn Any + Send + Sync>)
            })
        };
        OWNER.with(|o| {
            if let Some(owner) = &*o.borrow() {
                owner.register(node);
            }
        });

        Self {
            node,
            ty: PhantomData,
        }
    }

    pub fn with_value<U>(&self, fun: impl FnOnce(&T) -> U) -> Option<U> {
        Arena::with(|arena| {
            let m = arena.get(self.node);
            m.and_then(|n| n.downcast_ref::<T>()).map(fun)
        })
    }

    pub fn update_value<U>(&self, fun: impl FnOnce(&mut T) -> U) -> Option<U> {
        Arena::with_mut(|arena| {
            let m = arena.get_mut(self.node);
            m.and_then(|n| n.downcast_mut::<T>()).map(fun)
        })
    }

    pub fn get(&self) -> Option<T>
    where
        T: Clone,
    {
        self.with_value(T::clone)
    }

    pub fn set_value(&self, value: T) {
        self.update_value(|n| *n = value);
    }

    pub fn exists(&self) -> bool
    where
        T: Clone,
    {
        Arena::with(|arena| arena.contains_key(self.node))
    }

    pub fn dispose(&self) {
        Arena::with_mut(|arena| arena.remove(self.node));
    }
}

#[inline(always)]
#[track_caller]
#[deprecated = "This function is being removed to conform to Rust \
                idioms.Please use `StoredValue::new()` instead."]
pub fn store_value<T>(value: T) -> StoredValue<T>
where
    T: Send + Sync + 'static,
{
    StoredValue::new(value)
}

#[doc(hidden)]
pub trait StoredData {
    type Data;

    fn get_value(&self) -> Option<Self::Data>;

    fn dispose(&self);
}

/*impl<T> ReactiveNode for T
where
    T: StoredData,
    T::Data: ReactiveNode,
{
    fn mark_dirty(&self) {
        if let Some(inner) = self.get_value() {
            inner.mark_dirty();
        }
    }

    fn mark_check(&self) {
        if let Some(inner) = self.get_value() {
            inner.mark_check();
        }
    }

    fn mark_subscribers_check(&self) {
        if let Some(inner) = self.get_value() {
            inner.mark_subscribers_check();
        }
    }

    fn update_if_necessary(&self) -> bool {
        if let Some(inner) = self.get_value() {
            inner.update_if_necessary()
        } else {
            false
        }
    }
}

impl<T> Source for T
where
    T: StoredData,
    T::Data: Source,
{
    fn add_subscriber(&self, subscriber: AnySubscriber) {
        if let Some(inner) = self.get_value() {
            inner.add_subscriber(subscriber);
        }
    }

    fn remove_subscriber(&self, subscriber: &AnySubscriber) {
        if let Some(inner) = self.get_value() {
            inner.remove_subscriber(subscriber);
        }
    }

    fn clear_subscribers(&self) {
        if let Some(inner) = self.get_value() {
            inner.clear_subscribers();
        }
    }
}

impl<T> Subscriber for T
where
    T: StoredData,
    T::Data: Subscriber,
{
    fn add_source(&self, source: AnySource) {
        if let Some(inner) = self.get_value() {
            inner.add_source(source);
        }
    }

    fn clear_sources(&self, subscriber: &AnySubscriber) {
        if let Some(inner) = self.get_value() {
            inner.clear_sources(subscriber);
        }
    }
}

impl<T> DefinedAt for T
where
    T: StoredData,
    T::Data: DefinedAt,
{
    fn defined_at(&self) -> Option<&'static Location<'static>> {
        self.get_value().and_then(|n| n.defined_at())
    }
}

impl<T> ToAnySource for T
where
    T: StoredData,
    T::Data: ToAnySource + DefinedAt,
{
    #[track_caller]
    fn to_any_source(&self) -> AnySource {
        self.get_value()
            .map(|inner| inner.to_any_source())
            .unwrap_or_else(unwrap_signal!(self))
    }
}

impl<T> ToAnySubscriber for T
where
    T: StoredData,
    T::Data: ToAnySubscriber + DefinedAt,
{
    #[track_caller]
    fn to_any_subscriber(&self) -> AnySubscriber {
        self.get_value()
            .map(|inner| inner.to_any_subscriber())
            .unwrap_or_else(unwrap_signal!(self))
    }
}

impl<T> WithUntracked for T
where
    T: StoredData + DefinedAt,
    T::Data: WithUntracked,
{
    type Value = <<T as StoredData>::Data as WithUntracked>::Value;

    #[track_caller]
    fn try_with_untracked<U>(
        &self,
        fun: impl FnOnce(&Self::Value) -> U,
    ) -> Option<U> {
        self.get_value().and_then(|n| n.try_with_untracked(fun))
    }
}

impl<T> Trigger for T
where
    T: StoredData,
    T::Data: Trigger,
{
    fn trigger(&self) {
        if let Some(inner) = self.get_value() {
            inner.trigger();
        }
    }
}

impl<T> UpdateUntracked for T
where
    T: StoredData,
    T::Data: UpdateUntracked,
{
    type Value = <<T as StoredData>::Data as UpdateUntracked>::Value;

    fn try_update_untracked<U>(
        &self,
        fun: impl FnOnce(&mut Self::Value) -> U,
    ) -> Option<U> {
        self.get_value()
            .and_then(|inner| inner.try_update_untracked(fun))
    }
}*/
