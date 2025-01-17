use super::{
    guards::{Plain, ReadGuard},
    subscriber_traits::AsSubscriberSet,
};
use crate::{
    graph::SubscriberSet,
    traits::{DefinedAt, IntoInner, IsDisposed, ReadUntracked},
};
use core::fmt::{Debug, Formatter, Result};
use std::{
    hash::Hash,
    panic::Location,
    sync::{Arc, RwLock},
};

/// A reference-counted getter for a reactive signal.
///
/// A signal is a piece of data that may change over time,
/// and notifies other code when it has changed.
///
/// This is a reference-counted signal, which is `Clone` but not `Copy`.
/// For arena-allocated `Copy` signals, use [`ReadSignal`](super::ReadSignal).
///
/// ## Core Trait Implementations
/// - [`.get()`](crate::traits::Get) clones the current value of the signal.
///   If you call it within an effect, it will cause that effect to subscribe
///   to the signal, and to re-run whenever the value of the signal changes.
///   - [`.get_untracked()`](crate::traits::GetUntracked) clones the value of
///     the signal without reactively tracking it.
/// - [`.read()`](crate::traits::Read) returns a guard that allows accessing the
///   value of the signal by reference. If you call it within an effect, it will
///   cause that effect to subscribe to the signal, and to re-run whenever the
///   value of the signal changes.
///   - [`.read_untracked()`](crate::traits::ReadUntracked) gives access to the
///     current value of the signal without reactively tracking it.
/// - [`.with()`](crate::traits::With) allows you to reactively access the signal’s
///   value without cloning by applying a callback function.
///   - [`.with_untracked()`](crate::traits::WithUntracked) allows you to access
///     the signal’s value by applying a callback function without reactively
///     tracking it.
/// - [`.to_stream()`](crate::traits::ToStream) converts the signal to an `async`
///   stream of values.
/// - [`::from_stream()`](crate::traits::FromStream) converts an `async` stream
///   of values into a signal containing the latest value.
///
/// ## Examples
/// ```
/// # use reactive_graph::prelude::*; use reactive_graph::signal::*;
/// let (count, set_count) = arc_signal(0);
///
/// // calling .get() clones and returns the value
/// assert_eq!(count.get(), 0);
/// // calling .read() accesses the value by reference
/// assert_eq!(count.read(), 0);
/// ```
pub struct ArcReadSignal<T> {
    #[cfg(any(debug_assertions, leptos_debuginfo))]
    pub(crate) defined_at: &'static Location<'static>,
    pub(crate) value: Arc<RwLock<T>>,
    pub(crate) inner: Arc<RwLock<SubscriberSet>>,
}

impl<T> Clone for ArcReadSignal<T> {
    #[track_caller]
    fn clone(&self) -> Self {
        Self {
            #[cfg(any(debug_assertions, leptos_debuginfo))]
            defined_at: self.defined_at,
            value: Arc::clone(&self.value),
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<T> Debug for ArcReadSignal<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_struct("ArcReadSignal")
            .field("type", &std::any::type_name::<T>())
            .field("value", &Arc::as_ptr(&self.value))
            .finish()
    }
}

impl<T: Default> Default for ArcReadSignal<T> {
    #[track_caller]
    fn default() -> Self {
        Self {
            #[cfg(any(debug_assertions, leptos_debuginfo))]
            defined_at: Location::caller(),
            value: Arc::new(RwLock::new(T::default())),
            inner: Arc::new(RwLock::new(SubscriberSet::new())),
        }
    }
}

impl<T> PartialEq for ArcReadSignal<T> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.value, &other.value)
    }
}

impl<T> Eq for ArcReadSignal<T> {}

impl<T> Hash for ArcReadSignal<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::ptr::hash(&Arc::as_ptr(&self.value), state);
    }
}

impl<T> DefinedAt for ArcReadSignal<T> {
    #[inline(always)]
    fn defined_at(&self) -> Option<&'static Location<'static>> {
        #[cfg(any(debug_assertions, leptos_debuginfo))]
        {
            Some(self.defined_at)
        }
        #[cfg(not(any(debug_assertions, leptos_debuginfo)))]
        {
            None
        }
    }
}

impl<T> IsDisposed for ArcReadSignal<T> {
    #[inline(always)]
    fn is_disposed(&self) -> bool {
        false
    }
}

impl<T> IntoInner for ArcReadSignal<T> {
    type Value = T;

    #[inline(always)]
    fn into_inner(self) -> Option<Self::Value> {
        Some(Arc::into_inner(self.value)?.into_inner().unwrap())
    }
}

impl<T> AsSubscriberSet for ArcReadSignal<T> {
    type Output = Arc<RwLock<SubscriberSet>>;

    #[inline(always)]
    fn as_subscriber_set(&self) -> Option<Self::Output> {
        Some(Arc::clone(&self.inner))
    }
}

impl<T: 'static> ReadUntracked for ArcReadSignal<T> {
    type Value = ReadGuard<T, Plain<T>>;

    #[track_caller]
    fn try_read_untracked(&self) -> Option<Self::Value> {
        Plain::try_new(Arc::clone(&self.value)).map(ReadGuard::new)
    }
}
