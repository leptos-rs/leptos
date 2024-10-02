use super::{guards::WriteGuard, ArcWriteSignal};
use crate::{
    owner::{ArenaItem, Storage, SyncStorage},
    traits::{DefinedAt, Dispose, IsDisposed, Notify, UntrackableGuard, Write},
};
use core::fmt::Debug;
use guardian::ArcRwLockWriteGuardian;
use std::{hash::Hash, ops::DerefMut, panic::Location, sync::Arc};

/// An arena-allocated setter for a reactive signal.
///
/// A signal is a piece of data that may change over time,
/// and notifies other code when it has changed.
///
/// This is an arena-allocated signal, which is `Copy` and is disposed when its reactive
/// [`Owner`](crate::owner::Owner) cleans up. For a reference-counted signal that lives
/// as long as a reference to it is alive, see [`ArcWriteSignal`].
///
/// ## Core Trait Implementations
/// - [`.set()`](crate::traits::Set) sets the signal to a new value.
/// - [`.update()`](crate::traits::Update) updates the value of the signal by
///   applying a closure that takes a mutable reference.
/// - [`.write()`](crate::traits::Write) returns a guard through which the signal
///   can be mutated, and which notifies subscribers when it is dropped.
///
/// > Each of these has a related `_untracked()` method, which updates the signal
/// > without notifying subscribers. Untracked updates are not desirable in most
/// > cases, as they cause “tearing” between the signal’s value and its observed
/// > value. If you want a non-reactive container, used [`ArenaItem`] instead.
///
/// ## Examples
/// ```
/// # use reactive_graph::prelude::*; use reactive_graph::signal::*;  let owner = reactive_graph::owner::Owner::new(); owner.set();
/// let (count, set_count) = signal(0);
///
/// // ✅ calling the setter sets the value
/// //    `set_count(1)` on nightly
/// set_count.set(1);
/// assert_eq!(count.get(), 1);
///
/// // ❌ you could call the getter within the setter
/// // set_count.set(count.get() + 1);
///
/// // ✅ however it's more efficient to use .update() and mutate the value in place
/// set_count.update(|count: &mut i32| *count += 1);
/// assert_eq!(count.get(), 2);
///
/// // ✅ `.write()` returns a guard that implements `DerefMut` and will notify when dropped
/// *set_count.write() += 1;
/// assert_eq!(count.get(), 3);
/// ```
pub struct WriteSignal<T, S = SyncStorage> {
    #[cfg(debug_assertions)]
    pub(crate) defined_at: &'static Location<'static>,
    pub(crate) inner: ArenaItem<ArcWriteSignal<T>, S>,
}

impl<T, S> Dispose for WriteSignal<T, S> {
    fn dispose(self) {
        self.inner.dispose()
    }
}

impl<T, S> Copy for WriteSignal<T, S> {}

impl<T, S> Clone for WriteSignal<T, S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T, S> Debug for WriteSignal<T, S>
where
    S: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WriteSignal")
            .field("type", &std::any::type_name::<T>())
            .field("store", &self.inner)
            .finish()
    }
}

impl<T, S> PartialEq for WriteSignal<T, S> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<T, S> Eq for WriteSignal<T, S> {}

impl<T, S> Hash for WriteSignal<T, S> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
    }
}

impl<T, S> DefinedAt for WriteSignal<T, S> {
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

impl<T, S> IsDisposed for WriteSignal<T, S> {
    fn is_disposed(&self) -> bool {
        self.inner.is_disposed()
    }
}

impl<T, S> Notify for WriteSignal<T, S>
where
    T: 'static,
    S: Storage<ArcWriteSignal<T>>,
{
    fn notify(&self) {
        if let Some(inner) = self.inner.try_get_value() {
            inner.notify();
        }
    }
}

impl<T, S> Write for WriteSignal<T, S>
where
    T: 'static,
    S: Storage<ArcWriteSignal<T>>,
{
    type Value = T;

    fn try_write(&self) -> Option<impl UntrackableGuard<Target = Self::Value>> {
        let guard = self.inner.try_with_value(|n| {
            ArcRwLockWriteGuardian::take(Arc::clone(&n.value)).ok()
        })??;
        Some(WriteGuard::new(*self, guard))
    }

    fn try_write_untracked(
        &self,
    ) -> Option<impl DerefMut<Target = Self::Value>> {
        self.inner
            .try_with_value(|n| n.try_write_untracked())
            .flatten()
    }
}
