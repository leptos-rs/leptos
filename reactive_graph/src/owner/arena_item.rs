use super::{
    arena::{Arena, NodeId},
    LocalStorage, Storage, SyncStorage, OWNER,
};
use crate::traits::{Dispose, IsDisposed};
use send_wrapper::SendWrapper;
use std::{any::Any, hash::Hash, marker::PhantomData};

/// A copyable, stable reference for any value, stored on the arena whose ownership is managed by the
/// reactive ownership tree.
#[derive(Debug)]
pub struct ArenaItem<T, S = SyncStorage> {
    node: NodeId,
    #[allow(clippy::type_complexity)]
    ty: PhantomData<fn() -> (SendWrapper<T>, S)>,
}

impl<T, S> Copy for ArenaItem<T, S> {}

impl<T, S> Clone for ArenaItem<T, S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T, S> PartialEq for ArenaItem<T, S> {
    fn eq(&self, other: &Self) -> bool {
        self.node == other.node
    }
}

impl<T, S> Eq for ArenaItem<T, S> {}

impl<T, S> Hash for ArenaItem<T, S> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.node.hash(state);
    }
}

impl<T, S> ArenaItem<T, S>
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
        }
    }
}

impl<T, S> Default for ArenaItem<T, S>
where
    T: Default + 'static,
    S: Storage<T>,
{
    #[track_caller] // Default trait is not annotated with #[track_caller]
    fn default() -> Self {
        Self::new_with_storage(Default::default())
    }
}

impl<T> ArenaItem<T>
where
    T: Send + Sync + 'static,
{
    /// Stores the given value in the arena allocator.
    #[track_caller]
    pub fn new(value: T) -> Self {
        ArenaItem::new_with_storage(value)
    }
}

impl<T> ArenaItem<T, LocalStorage>
where
    T: 'static,
{
    /// Stores the given value in the arena allocator.
    #[track_caller]
    pub fn new_local(value: T) -> Self {
        ArenaItem::new_with_storage(value)
    }
}

impl<T, S: Storage<T>> ArenaItem<T, S> {
    /// Applies a function to a reference to the stored value and returns the result, or `None` if it has already been disposed.
    #[track_caller]
    pub fn try_with_value<U>(&self, fun: impl FnOnce(&T) -> U) -> Option<U> {
        S::try_with(self.node, fun)
    }

    /// Applies a function to a mutable reference to the stored value and returns the result, or `None` if it has already been disposed.
    #[track_caller]
    pub fn try_update_value<U>(
        &self,
        fun: impl FnOnce(&mut T) -> U,
    ) -> Option<U> {
        S::try_with_mut(self.node, fun)
    }
}

impl<T: Clone, S: Storage<T>> ArenaItem<T, S> {
    /// Returns a clone of the stored value, or `None` if it has already been disposed.
    #[track_caller]
    pub fn try_get_value(&self) -> Option<T> {
        S::try_with(self.node, Clone::clone)
    }
}

impl<T, S> IsDisposed for ArenaItem<T, S> {
    fn is_disposed(&self) -> bool {
        Arena::with(|arena| !arena.contains_key(self.node))
    }
}

impl<T, S> Dispose for ArenaItem<T, S> {
    fn dispose(self) {
        Arena::with_mut(|arena| arena.remove(self.node));
    }
}
