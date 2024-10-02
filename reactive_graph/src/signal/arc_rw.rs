use super::{
    guards::{Plain, ReadGuard, UntrackedWriteGuard, WriteGuard},
    subscriber_traits::AsSubscriberSet,
    ArcReadSignal, ArcWriteSignal,
};
use crate::{
    graph::{ReactiveNode, SubscriberSet},
    prelude::{IsDisposed, Notify},
    traits::{DefinedAt, ReadUntracked, UntrackableGuard, Write},
};
use core::fmt::{Debug, Formatter, Result};
use std::{
    hash::Hash,
    panic::Location,
    sync::{Arc, RwLock},
};

/// A reference-counted signal that can be read from or written to.
///
/// A signal is a piece of data that may change over time, and notifies other
/// code when it has changed. This is the atomic unit of reactivity, which begins all other
/// processes of reactive updates.
///
/// This is a reference-counted signal, which is `Clone` but not `Copy`.
/// For arena-allocated `Copy` signals, use [`RwSignal`](super::RwSignal).
///
/// ## Core Trait Implementations
///
/// ### Reading the Value
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
///
/// ### Updating the Value
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
///
/// ```
/// # use reactive_graph::prelude::*;
/// # use reactive_graph::signal::*; let owner = reactive_graph::owner::Owner::new(); owner.set();
/// let count = ArcRwSignal::new(0);
///
/// // ✅ calling the getter clones and returns the value
/// //    this can be `count()` on nightly
/// assert_eq!(count.get(), 0);
///
/// // ✅ calling the setter sets the value
/// //    this can be `set_count(1)` on nightly
/// count.set(1);
/// assert_eq!(count.get(), 1);
///
/// // ❌ you could call the getter within the setter
/// // set_count.set(count.get() + 1);
///
/// // ✅ however it's more efficient to use .update() and mutate the value in place
/// count.update(|count: &mut i32| *count += 1);
/// assert_eq!(count.get(), 2);
///
/// // ✅ you can create "derived signals" with a Fn() -> T interface
/// let double_count = {
///   // clone before moving into the closure because we use it below
///   let count = count.clone();
///   move || count.get() * 2
/// };
/// count.set(0);
/// assert_eq!(double_count(), 0);
/// count.set(1);
/// assert_eq!(double_count(), 2);
/// ```
pub struct ArcRwSignal<T> {
    #[cfg(debug_assertions)]
    pub(crate) defined_at: &'static Location<'static>,
    pub(crate) value: Arc<RwLock<T>>,
    pub(crate) inner: Arc<RwLock<SubscriberSet>>,
}

impl<T> Clone for ArcRwSignal<T> {
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

impl<T> Debug for ArcRwSignal<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_struct("ArcRwSignal")
            .field("type", &std::any::type_name::<T>())
            .field("value", &Arc::as_ptr(&self.value))
            .finish()
    }
}

impl<T> PartialEq for ArcRwSignal<T> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.value, &other.value)
    }
}

impl<T> Eq for ArcRwSignal<T> {}

impl<T> Hash for ArcRwSignal<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::ptr::hash(&Arc::as_ptr(&self.value), state);
    }
}

impl<T> Default for ArcRwSignal<T>
where
    T: Default,
{
    #[track_caller]
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T> ArcRwSignal<T> {
    /// Creates a new signal, taking the initial value as its argument.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "trace", skip_all)
    )]
    #[track_caller]
    pub fn new(value: T) -> Self {
        Self {
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
            value: Arc::new(RwLock::new(value)),
            inner: Arc::new(RwLock::new(SubscriberSet::new())),
        }
    }

    /// Returns a read-only handle to the signal.
    #[track_caller]
    pub fn read_only(&self) -> ArcReadSignal<T> {
        ArcReadSignal {
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
            value: Arc::clone(&self.value),
            inner: Arc::clone(&self.inner),
        }
    }

    /// Returns a write-only handle to the signal.
    #[track_caller]
    pub fn write_only(&self) -> ArcWriteSignal<T> {
        ArcWriteSignal {
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
            value: Arc::clone(&self.value),
            inner: Arc::clone(&self.inner),
        }
    }

    /// Splits the signal into its readable and writable halves.
    #[track_caller]
    pub fn split(&self) -> (ArcReadSignal<T>, ArcWriteSignal<T>) {
        (self.read_only(), self.write_only())
    }

    /// Reunites the two halves of a signal. Returns `None` if the two signals
    /// provided were not created from the same signal.
    #[track_caller]
    pub fn unite(
        read: ArcReadSignal<T>,
        write: ArcWriteSignal<T>,
    ) -> Option<Self> {
        if Arc::ptr_eq(&read.inner, &write.inner) {
            Some(Self {
                #[cfg(debug_assertions)]
                defined_at: Location::caller(),
                value: read.value,
                inner: read.inner,
            })
        } else {
            None
        }
    }
}

impl<T> DefinedAt for ArcRwSignal<T> {
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

impl<T> IsDisposed for ArcRwSignal<T> {
    #[inline(always)]
    fn is_disposed(&self) -> bool {
        false
    }
}

impl<T> AsSubscriberSet for ArcRwSignal<T> {
    type Output = Arc<RwLock<SubscriberSet>>;

    #[inline(always)]
    fn as_subscriber_set(&self) -> Option<Self::Output> {
        Some(Arc::clone(&self.inner))
    }
}

impl<T: 'static> ReadUntracked for ArcRwSignal<T> {
    type Value = ReadGuard<T, Plain<T>>;

    fn try_read_untracked(&self) -> Option<Self::Value> {
        Plain::try_new(Arc::clone(&self.value)).map(ReadGuard::new)
    }
}

impl<T> Notify for ArcRwSignal<T> {
    fn notify(&self) {
        self.mark_dirty();
    }
}

impl<T: 'static> Write for ArcRwSignal<T> {
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
