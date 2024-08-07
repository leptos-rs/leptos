use super::{
    arena::{Arena, NodeId},
    OWNER,
};
use crate::{
    traits::{DefinedAt, Dispose, IsDisposed},
    unwrap_signal,
};
use send_wrapper::SendWrapper;
use std::{any::Any, hash::Hash, marker::PhantomData, panic::Location};

/// A trait for borrowing and taking data.
pub trait StorageAccess<T> {
    /// Borrows the value.
    fn as_borrowed(&self) -> &T;

    /// Takes the value.
    fn into_taken(self) -> T;
}

impl<T> StorageAccess<T> for T {
    fn as_borrowed(&self) -> &T {
        self
    }

    fn into_taken(self) -> T {
        self
    }
}

impl<T> StorageAccess<T> for SendWrapper<T> {
    fn as_borrowed(&self) -> &T {
        self
    }

    fn into_taken(self) -> T {
        self.take()
    }
}

/// A way of storing a [`StoredValue`], either as itself or with a wrapper to make it threadsafe.
///
/// This exists because all items stored in the arena must be `Send + Sync`, but in single-threaded
/// environments you might want or need to use thread-unsafe types.
pub trait Storage<T>: Send + Sync + 'static {
    /// The type being stored, once it has been wrapped.
    type Wrapped: StorageAccess<T> + Send + Sync + 'static;

    /// Adds any needed wrapper to the type.
    fn wrap(value: T) -> Self::Wrapped;

    /// Applies the given function to the stored value, if it exists and can be accessed from this
    /// thread.
    fn try_with<U>(node: NodeId, fun: impl FnOnce(&T) -> U) -> Option<U>;

    /// Applies the given function to a mutable reference to the stored value, if it exists and can be accessed from this
    /// thread.
    fn try_with_mut<U>(
        node: NodeId,
        fun: impl FnOnce(&mut T) -> U,
    ) -> Option<U>;

    /// Sets a new value for the stored value. If it has been disposed, returns `Some(T)`.
    fn try_set(node: NodeId, value: T) -> Option<T>;
}

/// A form of [`Storage`] that stores the type as itself, with no wrapper.
#[derive(Debug, Copy, Clone)]
pub struct SyncStorage;

impl<T> Storage<T> for SyncStorage
where
    T: Send + Sync + 'static,
{
    type Wrapped = T;

    #[inline(always)]
    fn wrap(value: T) -> Self::Wrapped {
        value
    }

    fn try_with<U>(node: NodeId, fun: impl FnOnce(&T) -> U) -> Option<U> {
        Arena::with(|arena| {
            let m = arena.get(node);
            m.and_then(|n| n.downcast_ref::<T>()).map(fun)
        })
    }

    fn try_with_mut<U>(
        node: NodeId,
        fun: impl FnOnce(&mut T) -> U,
    ) -> Option<U> {
        Arena::with_mut(|arena| {
            let m = arena.get_mut(node);
            m.and_then(|n| n.downcast_mut::<T>()).map(fun)
        })
    }

    fn try_set(node: NodeId, value: T) -> Option<T> {
        Arena::with_mut(|arena| {
            let m = arena.get_mut(node);
            match m.and_then(|n| n.downcast_mut::<T>()) {
                Some(inner) => {
                    *inner = value;
                    None
                }
                None => Some(value),
            }
        })
    }
}

/// A form of [`Storage`] that stores the type with a wrapper that makes it `Send + Sync`, but only
/// allows it to be accessed from the thread on which it was created.
#[derive(Debug, Copy, Clone)]
pub struct LocalStorage;

impl<T> Storage<T> for LocalStorage
where
    T: 'static,
{
    type Wrapped = SendWrapper<T>;

    fn wrap(value: T) -> Self::Wrapped {
        SendWrapper::new(value)
    }

    fn try_with<U>(node: NodeId, fun: impl FnOnce(&T) -> U) -> Option<U> {
        Arena::with(|arena| {
            let m = arena.get(node);
            m.and_then(|n| n.downcast_ref::<SendWrapper<T>>())
                .map(|inner| fun(inner))
        })
    }

    fn try_with_mut<U>(
        node: NodeId,
        fun: impl FnOnce(&mut T) -> U,
    ) -> Option<U> {
        Arena::with_mut(|arena| {
            let m = arena.get_mut(node);
            m.and_then(|n| n.downcast_mut::<SendWrapper<T>>())
                .map(|inner| fun(&mut *inner))
        })
    }

    fn try_set(node: NodeId, value: T) -> Option<T> {
        Arena::with_mut(|arena| {
            let m = arena.get_mut(node);
            match m.and_then(|n| n.downcast_mut::<SendWrapper<T>>()) {
                Some(inner) => {
                    *inner = SendWrapper::new(value);
                    None
                }
                None => Some(value),
            }
        })
    }
}

/// A **non-reactive**, `Copy` handle for any value.
///
/// This allows you to create a stable reference for any value by storing it within
/// the reactive system. Like the signal types (e.g., [`ReadSignal`](crate::signal::ReadSignal)
/// and [`RwSignal`](crate::signal::RwSignal)), it is `Copy` and `'static`. Unlike the signal
/// types, it is not reactive; accessing it does not cause effects to subscribe, and
/// updating it does not notify anything else.
#[derive(Debug)]
pub struct StoredValue<T, S = SyncStorage> {
    node: NodeId,
    ty: PhantomData<(SendWrapper<T>, S)>,
    #[cfg(debug_assertions)]
    defined_at: &'static Location<'static>,
}

impl<T, S> Copy for StoredValue<T, S> {}

impl<T, S> Clone for StoredValue<T, S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T, S> PartialEq for StoredValue<T, S> {
    fn eq(&self, other: &Self) -> bool {
        self.node == other.node
    }
}

impl<T, S> Eq for StoredValue<T, S> {}

impl<T, S> Hash for StoredValue<T, S> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.node.hash(state);
    }
}

impl<T, S> DefinedAt for StoredValue<T, S> {
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

impl<T, S> StoredValue<T, S>
where
    T: 'static,
    S: Storage<T>,
{
    /// Stores the given value in the arena allocator.
    #[track_caller]
    pub fn new_with_storage(value: T) -> Self {
        let node = {
            Arena::with_mut(|arena| {
                arena.insert(
                    Box::new(S::wrap(value)) as Box<dyn Any + Send + Sync>
                )
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

impl<T> StoredValue<T>
where
    T: Send + Sync + 'static,
{
    /// Stores the given value in the arena allocator.
    #[track_caller]
    pub fn new(value: T) -> Self {
        StoredValue::new_with_storage(value)
    }
}

impl<T> StoredValue<T, LocalStorage>
where
    T: 'static,
{
    /// Stores the given value in the arena allocator.
    #[track_caller]
    pub fn new_local(value: T) -> Self {
        StoredValue::new_with_storage(value)
    }
}

impl<T, S: Storage<T>> StoredValue<T, S> {
    /// Same as [`StoredValue::with_value`] but returns `Some(O)` only if
    /// the stored value has not yet been disposed, `None` otherwise.
    pub fn try_with_value<U>(&self, fun: impl FnOnce(&T) -> U) -> Option<U> {
        S::try_with(self.node, fun)
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
        S::try_with_mut(self.node, fun)
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
        S::try_set(self.node, value)
    }

    /// Sets the value to a new value.
    pub fn set_value(&self, value: T) {
        self.update_value(|n| *n = value);
    }
}

impl<T, S> IsDisposed for StoredValue<T, S> {
    fn is_disposed(&self) -> bool {
        Arena::with(|arena| !arena.contains_key(self.node))
    }
}

impl<T, S: Storage<T>> StoredValue<T, S>
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
}

impl<T, S> Dispose for StoredValue<T, S> {
    fn dispose(self) {
        Arena::with_mut(|arena| arena.remove(self.node));
    }
}

/// Creates a new [`StoredValue`].
#[inline(always)]
#[track_caller]
#[deprecated = "This function is being removed to conform to Rust idioms. \
                Please use `StoredValue::new()` or `StoredValue::new_local()` \
                instead."]
pub fn store_value<T>(value: T) -> StoredValue<T>
where
    T: Send + Sync + 'static,
{
    StoredValue::new(value)
}

/// Converts some value into a locally-stored type, using [`LocalStorage`].
///
/// This is modeled on [`From`] but special-cased for this thread-local storage method, which
/// allows for better type inference for the default case.
pub trait FromLocal<T> {
    /// Converts between the types.
    fn from_local(value: T) -> Self;
}
