use super::{
    arena::{Arena, NodeId},
    OWNER,
};
use crate::{
    traits::{DefinedAt, Dispose},
    unwrap_signal,
};
use std::{any::Any, hash::Hash, marker::PhantomData, panic::Location};

#[derive(Debug)]
pub struct StoredValue<T> {
    node: NodeId,
    ty: PhantomData<T>,
    #[cfg(debug_assertions)]
    defined_at: &'static Location<'static>,
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

impl<T> DefinedAt for StoredValue<T> {
    fn defined_at(&self) -> Option<&'static Location<'static>> {
        #[cfg(debug_assertions)]
        {
            Some(self.defined_at)
        }
        #[cfg(not(debug_assertions))]
        {
            None
        }
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
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
        }
    }

    pub fn try_with_value<U>(&self, fun: impl FnOnce(&T) -> U) -> Option<U> {
        Arena::with(|arena| {
            let m = arena.get(self.node);
            m.and_then(|n| n.downcast_ref::<T>()).map(fun)
        })
    }

    pub fn with_value<U>(&self, fun: impl FnOnce(&T) -> U) -> U {
        self.try_with_value(fun)
            .unwrap_or_else(unwrap_signal!(self))
    }

    pub fn try_update_value<U>(
        &self,
        fun: impl FnOnce(&mut T) -> U,
    ) -> Option<U> {
        Arena::with_mut(|arena| {
            let m = arena.get_mut(self.node);
            m.and_then(|n| n.downcast_mut::<T>()).map(fun)
        })
    }

    pub fn update_value<U>(&self, fun: impl FnOnce(&mut T) -> U) {
        self.try_update_value(fun);
    }

    pub fn try_set_value(&self, value: T) -> Option<T> {
        Arena::with_mut(|arena| {
            let m = arena.get_mut(self.node);
            match m.and_then(|n| n.downcast_mut::<T>()) {
                Some(inner) => {
                    *inner = value;
                    None
                }
                None => Some(value),
            }
        })
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
}

impl<T> StoredValue<T>
where
    T: Send + Sync + Clone + 'static,
{
    pub fn try_get_value(&self) -> Option<T> {
        self.try_with_value(T::clone)
    }

    pub fn get_value(&self) -> T {
        self.with_value(T::clone)
    }

    pub(crate) fn get(&self) -> Option<T> {
        self.try_get_value()
    }
}

impl<T> Dispose for StoredValue<T> {
    fn dispose(self) {
        Arena::with_mut(|arena| arena.remove(self.node));
    }
}

#[inline(always)]
#[track_caller]
#[deprecated = "This function is being removed to conform to Rust idioms. \
                Please use `StoredValue::new()` instead."]
pub fn store_value<T>(value: T) -> StoredValue<T>
where
    T: Send + Sync + 'static,
{
    StoredValue::new(value)
}
