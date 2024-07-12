use super::{guards::WriteGuard, ArcWriteSignal};
use crate::{
    owner::StoredValue,
    traits::{
        DefinedAt, Dispose, IsDisposed, Trigger, UntrackableGuard, Writeable,
    },
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
/// - [`.write()`](crate::traits::Writeable) returns a guard through which the signal
///   can be mutated, and which notifies subscribers when it is dropped.
///
/// > Each of these has a related `_untracked()` method, which updates the signal
/// > without notifying subscribers. Untracked updates are not desirable in most
/// > cases, as they cause “tearing” between the signal’s value and its observed
/// > value. If you want a non-reactive container, used [`StoredValue`] instead.
///
/// ## Examples
/// ```
/// # use reactive_graph::prelude::*; use reactive_graph::signal::*;
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
pub struct WriteSignal<T> {
    #[cfg(debug_assertions)]
    pub(crate) defined_at: &'static Location<'static>,
    pub(crate) inner: StoredValue<ArcWriteSignal<T>>,
}

impl<T> Dispose for WriteSignal<T> {
    fn dispose(self) {
        self.inner.dispose()
    }
}

impl<T> Copy for WriteSignal<T> {}

impl<T> Clone for WriteSignal<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Debug for WriteSignal<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WriteSignal")
            .field("type", &std::any::type_name::<T>())
            .field("store", &self.inner)
            .finish()
    }
}

impl<T> PartialEq for WriteSignal<T> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<T> Eq for WriteSignal<T> {}

impl<T> Hash for WriteSignal<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
    }
}

impl<T> DefinedAt for WriteSignal<T> {
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

impl<T: 'static> IsDisposed for WriteSignal<T> {
    fn is_disposed(&self) -> bool {
        !self.inner.exists()
    }
}

impl<T: 'static> Trigger for WriteSignal<T> {
    fn trigger(&self) {
        if let Some(inner) = self.inner.get() {
            inner.trigger();
        }
    }
}

impl<T: 'static> Writeable for WriteSignal<T> {
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
        self.inner.with_value(|n| n.try_write_untracked())
    }
}
