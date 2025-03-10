use super::arena::{Arena, NodeId};
use send_wrapper::SendWrapper;

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

/// A way of storing an [`ArenaItem`](super::arena_item::ArenaItem), either as itself or with a wrapper to make it threadsafe.
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

    /// Takes an item from the arena if it exists and can be accessed from this thread.
    /// If it cannot be casted, it will still be removed from the arena.
    fn take(node: NodeId) -> Option<T>;
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
        Arena::try_with(|arena| {
            let m = arena.get(node);
            m.and_then(|n| n.downcast_ref::<T>()).map(fun)
        })
        .flatten()
    }

    fn try_with_mut<U>(
        node: NodeId,
        fun: impl FnOnce(&mut T) -> U,
    ) -> Option<U> {
        Arena::try_with_mut(|arena| {
            let m = arena.get_mut(node);
            m.and_then(|n| n.downcast_mut::<T>()).map(fun)
        })
        .flatten()
    }

    fn try_set(node: NodeId, value: T) -> Option<T> {
        Arena::try_with_mut(|arena| {
            let m = arena.get_mut(node);
            match m.and_then(|n| n.downcast_mut::<T>()) {
                Some(inner) => {
                    *inner = value;
                    None
                }
                None => Some(value),
            }
        })
        .flatten()
    }

    fn take(node: NodeId) -> Option<T> {
        Arena::with_mut(|arena| {
            let m = arena.remove(node)?;
            match m.downcast::<T>() {
                Ok(inner) => Some(*inner),
                Err(_) => None,
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

    fn take(node: NodeId) -> Option<T> {
        Arena::with_mut(|arena| {
            let m = arena.remove(node)?;
            match m.downcast::<SendWrapper<T>>() {
                Ok(inner) => Some(inner.take()),
                Err(_) => None,
            }
        })
    }
}
