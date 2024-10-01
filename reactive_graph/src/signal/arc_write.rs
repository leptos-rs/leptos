use super::guards::{UntrackedWriteGuard, WriteGuard};
use crate::{
    graph::{ReactiveNode, SubscriberSet},
    prelude::{IsDisposed, Notify},
    traits::{DefinedAt, UntrackableGuard, Write},
};
use core::fmt::{Debug, Formatter, Result};
use std::{
    hash::Hash,
    panic::Location,
    sync::{Arc, RwLock},
};

/// A reference-counted setter for a reactive signal.
///
/// A signal is a piece of data that may change over time,
/// and notifies other code when it has changed.
///
/// This is a reference-counted signal, which is `Clone` but not `Copy`.
/// For arena-allocated `Copy` signals, use [`WriteSignal`](super::WriteSignal).
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
/// > value. If you want a non-reactive container, used [`ArenaItem`](crate::owner::ArenaItem)
/// > instead.
///
/// ## Examples
/// ```
/// # use reactive_graph::prelude::*; use reactive_graph::signal::*;
/// let (count, set_count) = arc_signal(0);
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
pub struct ArcWriteSignal<T> {
    #[cfg(debug_assertions)]
    pub(crate) defined_at: &'static Location<'static>,
    pub(crate) value: Arc<RwLock<T>>,
    pub(crate) inner: Arc<RwLock<SubscriberSet>>,
}

impl<T> Clone for ArcWriteSignal<T> {
    #[track_caller]
    fn clone(&self) -> Self {
        Self {
            #[cfg(debug_assertions)]
            defined_at: self.defined_at,
            value: Arc::clone(&self.value),
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<T> Debug for ArcWriteSignal<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_struct("ArcWriteSignal")
            .field("type", &std::any::type_name::<T>())
            .field("value", &Arc::as_ptr(&self.value))
            .finish()
    }
}

impl<T> PartialEq for ArcWriteSignal<T> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.value, &other.value)
    }
}

impl<T> Eq for ArcWriteSignal<T> {}

impl<T> Hash for ArcWriteSignal<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::ptr::hash(&Arc::as_ptr(&self.value), state);
    }
}

impl<T> DefinedAt for ArcWriteSignal<T> {
    #[inline(always)]
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

impl<T> IsDisposed for ArcWriteSignal<T> {
    #[inline(always)]
    fn is_disposed(&self) -> bool {
        false
    }
}

impl<T> Notify for ArcWriteSignal<T> {
    fn notify(&self) {
        self.inner.mark_dirty();
    }
}

impl<T: 'static> Write for ArcWriteSignal<T> {
    type Value = T;

    fn try_write(&self) -> Option<impl UntrackableGuard<Target = Self::Value>> {
        self.value
            .write()
            .ok()
            .map(|guard| WriteGuard::new(self.clone(), guard))
    }

    #[allow(refining_impl_trait)]
    fn try_write_untracked(&self) -> Option<UntrackedWriteGuard<Self::Value>> {
        UntrackedWriteGuard::try_new(Arc::clone(&self.value))
    }
}
