use super::{
    arena::{Arena, NodeId},
    OWNER,
};
use crate::{
    traits::{DefinedAt, Dispose, IsDisposed},
    unwrap_signal,
};
use std::{any::Any, hash::Hash, marker::PhantomData, panic::Location};

/// A **non-reactive**, `Copy` handle for any value.
///
/// This allows you to create a stable reference for any value by storing it within
/// the reactive system. Like the signal types (e.g., [`ReadSignal`](crate::signal::ReadSignal)
/// and [`RwSignal`](crate::signal::RwSignal)), it is `Copy` and `'static`. Unlike the signal
/// types, it is not reactive; accessing it does not cause effects to subscribe, and
/// updating it does not notify anything else.
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
    /// Stores the given value in the arena allocator.
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
}

impl<T: 'static> StoredValue<T> {
    /// Same as [`StoredValue::with_value`] but returns `Some(O)` only if
    /// the stored value has not yet been disposed, `None` otherwise.
    pub fn try_with_value<U>(&self, fun: impl FnOnce(&T) -> U) -> Option<U> {
        Arena::with(|arena| {
            let m = arena.get(self.node);
            m.and_then(|n| n.downcast_ref::<T>()).map(fun)
        })
    }

    /// Applies a function to the current stored value and returns the result.
    ///
    /// # Panics
    /// Panics if you try to access a value owned by a reactive node that has been disposed.
    ///
    /// # Examples
    /// ```
    /// # use reactive_graph::owner::StoredValue;
    /// pub struct MyUncloneableData {
    ///     pub value: String,
    /// }
    /// let data = StoredValue::new(MyUncloneableData { value: "a".into() });
    ///
    /// // calling .with_value() to extract the value
    /// data.with_value(|data| assert_eq!(data.value, "a"));
    pub fn with_value<U>(&self, fun: impl FnOnce(&T) -> U) -> U {
        self.try_with_value(fun)
            .unwrap_or_else(unwrap_signal!(self))
    }

    /// Updates the current value by applying the given closure, returning the return value of the
    /// closure, or `None` if the value has already been disposed.
    pub fn try_update_value<U>(
        &self,
        fun: impl FnOnce(&mut T) -> U,
    ) -> Option<U> {
        Arena::with_mut(|arena| {
            let m = arena.get_mut(self.node);
            m.and_then(|n| n.downcast_mut::<T>()).map(fun)
        })
    }

    /// Updates the stored value by applying the given closure.
    ///
    /// ## Examples
    /// ```
    /// # use reactive_graph::owner::StoredValue;
    /// pub struct MyUncloneableData {
    ///     pub value: String,
    /// }
    /// let data = StoredValue::new(MyUncloneableData { value: "a".into() });
    /// data.update_value(|data| data.value = "b".into());
    /// assert_eq!(data.with_value(|data| data.value.clone()), "b");
    /// ```
    pub fn update_value<U>(&self, fun: impl FnOnce(&mut T) -> U) {
        self.try_update_value(fun);
    }

    /// Tries to set the value. If the value has been disposed, returns `Some(value)`.
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

    /// Sets the value to a new value.
    pub fn set_value(&self, value: T) {
        self.update_value(|n| *n = value);
    }

    /// Returns `true` if the value has not yet been disposed.
    pub fn exists(&self) -> bool
    where
        T: Clone,
    {
        Arena::with(|arena| arena.contains_key(self.node))
    }
}

impl<T> IsDisposed for StoredValue<T> {
    fn is_disposed(&self) -> bool {
        Arena::with(|arena| arena.contains_key(self.node))
    }
}

impl<T> StoredValue<T>
where
    T: Clone + 'static,
{
    /// Clones and returns the current value, or `None` if it has already been disposed.
    pub fn try_get_value(&self) -> Option<T> {
        self.try_with_value(T::clone)
    }

    /// Clones and returns the current value.
    ///
    /// # Panics
    /// Panics if you try to access a value owned by a reactive node that has been disposed.
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

/// Creates a new [`StoredValue`].
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
