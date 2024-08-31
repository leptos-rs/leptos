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

impl<T, S> Default for StoredValue<T, S>
where
    T: Default + 'static,
    S: Storage<T>,
{
    #[track_caller] // Default trait is not annotated with #[track_caller]
    fn default() -> Self {
        Self::new_with_storage(Default::default())
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
    /// Returns an [`Option`] of applying a function to the value within the [`StoredValue`].
    ///
    /// If the owner of the reactive node has not been disposed [`Some`] is returned. Calling this
    /// function after the owner has been disposed will always return [`None`].
    ///
    /// See [`StoredValue::with_value`] for a version that panics in the case of the owner being
    /// disposed.
    ///
    /// # Examples
    /// ```rust
    /// # use reactive_graph::owner::StoredValue;
    /// # use reactive_graph::traits::Dispose;
    ///
    /// // Does not implement Clone
    /// struct Data {
    ///     rows: Vec<u8>,
    /// }
    ///
    /// let data = StoredValue::new(Data {
    ///     rows: vec![0, 1, 2, 3, 4],
    /// });
    ///
    /// // Easy to move into closures because StoredValue is Copy + 'static.
    /// // *NOTE* this is not the same thing as a derived signal!
    /// // *NOTE* this will not be automatically rerun as StoredValue is NOT reactive!
    /// let length_fn = move || data.try_with_value(|inner| inner.rows.len());
    ///
    /// let sum = data.try_with_value(|inner| inner.rows.iter().sum::<u8>());
    ///
    /// assert_eq!(sum, Some(10));
    /// assert_eq!(length_fn(), Some(5));
    ///
    /// // You should not call dispose yourself in normal user code.
    /// // This is shown here for the sake of the example.
    /// data.dispose();
    ///
    /// let last = data.try_with_value(|inner| inner.rows.last().cloned());
    ///
    /// assert_eq!(last, None);
    /// assert_eq!(length_fn(), None);
    /// ```
    pub fn try_with_value<U>(&self, fun: impl FnOnce(&T) -> U) -> Option<U> {
        S::try_with(self.node, fun)
    }

    /// Returns the output of applying a function to the value within the [`StoredValue`].
    ///
    /// # Panics
    ///
    /// This function panics when called after the owner of the reactive node has been disposed.
    /// See [`StoredValue::try_with_value`] for a version without panic.
    ///
    /// # Examples
    /// ```rust
    /// # use reactive_graph::owner::StoredValue;
    ///
    /// // Does not implement Clone
    /// struct Data {
    ///     rows: Vec<u8>,
    /// }
    ///
    /// let data = StoredValue::new(Data {
    ///     rows: vec![1, 2, 3],
    /// });
    ///
    /// // Easy to move into closures because StoredValue is Copy + 'static.
    /// // *NOTE* this is not the same thing as a derived signal!
    /// // *NOTE* this will not be automatically rerun as StoredValue is NOT reactive!
    /// let length_fn = move || data.with_value(|inner| inner.rows.len());
    ///
    /// let sum = data.with_value(|inner| inner.rows.iter().sum::<u8>());
    ///
    /// assert_eq!(sum, 6);
    /// assert_eq!(length_fn(), 3);
    /// ```
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

    /// Updates the value within [`StoredValue`] by applying a function to it.
    ///
    /// # Panics
    /// This function panics when called after the owner of the reactive node has been disposed.
    /// See [`StoredValue::try_update_value`] for a version without panic.
    ///
    /// # Examples
    /// ```rust
    /// # use reactive_graph::owner::StoredValue;
    ///
    /// #[derive(Default)] // Does not implement Clone
    /// struct Data {
    ///     rows: Vec<u8>,
    /// }
    ///
    /// let data = StoredValue::new(Data::default());
    ///
    /// // Easy to move into closures because StoredValue is Copy + 'static.
    /// // *NOTE* this is not the same thing as a derived signal!
    /// // *NOTE* this will not be automatically rerun as StoredValue is NOT reactive!
    /// let push_next = move || {
    ///     data.update_value(|inner| match inner.rows.last().as_deref() {
    ///         Some(n) => inner.rows.push(n + 1),
    ///         None => inner.rows.push(0),
    ///     })
    /// };
    ///
    /// data.update_value(|inner| inner.rows = vec![5, 6, 7]);
    /// data.with_value(|inner| assert_eq!(inner.rows.last(), Some(&7)));
    ///
    /// push_next();
    /// data.with_value(|inner| assert_eq!(inner.rows.last(), Some(&8)));
    ///
    /// data.update_value(|inner| {
    ///     std::mem::take(inner) // sets Data back to default
    /// });
    /// data.with_value(|inner| assert!(inner.rows.is_empty()));
    /// ```
    pub fn update_value<U>(&self, fun: impl FnOnce(&mut T) -> U) {
        self.try_update_value(fun);
    }

    /// Sets the value within [`StoredValue`].
    ///
    /// Returns [`Some`] containing the passed value if the owner of the reactive node has been
    /// disposed.
    ///
    /// For types that do not implement [`Clone`], or in cases where allocating the entire object
    /// would be too expensive, prefer [`StoredValue::try_update_value`].
    ///
    /// # Examples
    /// ```rust
    /// # use reactive_graph::owner::StoredValue;
    /// # use reactive_graph::traits::Dispose;
    ///
    /// let data = StoredValue::new(String::default());
    ///
    /// // Easy to move into closures because StoredValue is Copy + 'static.
    /// // *NOTE* this is not the same thing as a derived signal!
    /// // *NOTE* this will not be automatically rerun as StoredValue is NOT reactive!
    /// let say_hello = move || {
    ///     // Note that using the `update` methods would be more efficient here.
    ///     data.try_set_value("Hello, World!".into())
    /// };
    /// // *NOTE* this is not the same thing as a derived signal!
    /// // *NOTE* this will not be automatically rerun as StoredValue is NOT reactive!
    /// let reset = move || {
    ///     // Note that using the `update` methods would be more efficient here.
    ///     data.try_set_value(Default::default())
    /// };
    /// assert_eq!(data.get_value(), "");
    ///
    /// // None is returned because the value was able to be updated
    /// assert_eq!(say_hello(), None);
    ///
    /// assert_eq!(data.get_value(), "Hello, World!");
    ///
    /// reset();
    /// assert_eq!(data.get_value(), "");
    ///
    /// // You should not call dispose yourself in normal user code.
    /// // This is shown here for the sake of the example.
    /// data.dispose();
    ///
    /// // The reactive owner is disposed, so the value we intended to set is now
    /// // returned as some.
    /// assert_eq!(say_hello().as_deref(), Some("Hello, World!"));
    /// assert_eq!(reset().as_deref(), Some(""));
    /// ```
    pub fn try_set_value(&self, value: T) -> Option<T> {
        S::try_set(self.node, value)
    }

    /// Sets the value within [`StoredValue`].
    ///
    /// For types that do not implement [`Clone`], or in cases where allocating the entire object
    /// would be too expensive, prefer [`StoredValue::update_value`].
    ///
    /// # Panics
    /// This function panics when called after the owner of the reactive node has been disposed.
    /// See [`StoredValue::try_set_value`] for a version without panic.
    ///
    /// # Examples
    /// ```rust
    /// # use reactive_graph::owner::StoredValue;
    ///
    /// let data = StoredValue::new(10);
    ///
    /// // Easy to move into closures because StoredValue is Copy + 'static.
    /// // *NOTE* this is not the same thing as a derived signal!
    /// // *NOTE* this will not be automatically rerun as StoredValue is NOT reactive!
    /// let maxout = move || data.set_value(u8::MAX);
    /// let zero = move || data.set_value(u8::MIN);
    ///
    /// maxout();
    /// assert_eq!(data.get_value(), u8::MAX);
    ///
    /// zero();
    /// assert_eq!(data.get_value(), u8::MIN);
    /// ```
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
    /// Returns the value within [`StoredValue`] by cloning.
    ///
    /// Returns [`Some`] containing the value if the owner of the reactive node has not been
    /// disposed. When disposed, returns [`None`].
    ///
    /// See [`StoredValue::try_with_value`] for a version that avoids cloning. See
    /// [`StoredValue::get_value`] for a version that clones, but panics if the node is disposed.
    ///
    /// # Examples
    /// ```rust
    /// # use reactive_graph::owner::StoredValue;
    /// # use reactive_graph::traits::Dispose;
    ///
    /// // u8 is practically free to clone.
    /// let data: StoredValue<u8> = StoredValue::new(10);
    ///
    /// // Larger data structures can become very expensive to clone.
    /// // You may prefer to use StoredValue::try_with_value.
    /// let _expensive: StoredValue<Vec<String>> = StoredValue::new(vec![]);
    ///
    /// // Easy to move into closures because StoredValue is Copy + 'static
    /// let maxout = move || data.set_value(u8::MAX);
    /// let zero = move || data.set_value(u8::MIN);
    ///
    /// maxout();
    /// assert_eq!(data.try_get_value(), Some(u8::MAX));
    ///
    /// zero();
    /// assert_eq!(data.try_get_value(), Some(u8::MIN));
    ///
    /// // You should not call dispose yourself in normal user code.
    /// // This is shown here for the sake of the example.
    /// data.dispose();
    ///
    /// assert_eq!(data.try_get_value(), None);
    /// ```
    pub fn try_get_value(&self) -> Option<T> {
        self.try_with_value(T::clone)
    }

    /// Returns the value within [`StoredValue`] by cloning.
    ///
    /// See [`StoredValue::with_value`] for a version that avoids cloning.
    ///
    /// # Panics
    /// This function panics when called after the owner of the reactive node has been disposed.
    /// See [`StoredValue::try_get_value`] for a version without panic.
    ///
    /// # Examples
    /// ```rust
    /// # use reactive_graph::owner::StoredValue;
    ///
    /// // u8 is practically free to clone.
    /// let data: StoredValue<u8> = StoredValue::new(10);
    ///
    /// // Larger data structures can become very expensive to clone.
    /// // You may prefer to use StoredValue::try_with_value.
    /// let _expensive: StoredValue<Vec<String>> = StoredValue::new(vec![]);
    ///
    /// // Easy to move into closures because StoredValue is Copy + 'static
    /// let maxout = move || data.set_value(u8::MAX);
    /// let zero = move || data.set_value(u8::MIN);
    ///
    /// maxout();
    /// assert_eq!(data.get_value(), u8::MAX);
    ///
    /// zero();
    /// assert_eq!(data.get_value(), u8::MIN);
    /// ```
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
#[deprecated(
    since = "0.7.0-beta4",
    note = "This function is being removed to conform to Rust idioms. Please \
            use `StoredValue::new()` or `StoredValue::new_local()` instead."
)]
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
