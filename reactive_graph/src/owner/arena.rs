use super::OWNER;
use crate::OrPoisoned;
use slotmap::{new_key_type, SlotMap};
use std::{
    any::Any,
    marker::PhantomData,
    sync::{OnceLock, RwLock},
};

new_key_type! { pub(crate) struct NodeId; }

static MAP: OnceLock<RwLock<SlotMap<NodeId, Box<dyn Any + Send + Sync>>>> =
    OnceLock::new();

pub(crate) fn map(
) -> &'static RwLock<SlotMap<NodeId, Box<dyn Any + Send + Sync>>> {
    MAP.get_or_init(Default::default)
}

#[derive(Debug)]
pub struct Stored<T> {
    node: NodeId,
    ty: PhantomData<T>,
}

impl<T> Copy for Stored<T> {}

impl<T> Clone for Stored<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Stored<T>
where
    T: Send + Sync + 'static,
{
    #[track_caller]
    pub fn new(value: T) -> Self {
        let node = {
            map()
                .write()
                .or_poisoned()
                .insert(Box::new(value) as Box<dyn Any + Send + Sync>)
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
        let m = map().read().or_poisoned();
        let m = m.get(self.node);

        m.and_then(|n| n.downcast_ref::<T>()).map(fun)
    }

    pub fn get(&self) -> Option<T>
    where
        T: Clone,
    {
        self.with_value(T::clone)
    }

    pub fn exists(&self) -> bool
    where
        T: Clone,
    {
        map().read().or_poisoned().contains_key(self.node)
    }

    pub fn dispose(&self) {
        map().write().or_poisoned().remove(self.node);
    }
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
