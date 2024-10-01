use super::{
    guards::{Plain, ReadGuard},
    subscriber_traits::AsSubscriberSet,
    ArcReadSignal,
};
use crate::{
    graph::SubscriberSet,
    owner::{ArenaItem, FromLocal, LocalStorage, Storage, SyncStorage},
    traits::{DefinedAt, Dispose, IsDisposed, ReadUntracked},
    unwrap_signal,
};
use core::fmt::Debug;
use std::{
    hash::Hash,
    panic::Location,
    sync::{Arc, RwLock},
};

/// An arena-allocated getter for a reactive signal.
///
/// A signal is a piece of data that may change over time,
/// and notifies other code when it has changed.
///
/// This is an arena-allocated signal, which is `Copy` and is disposed when its reactive
/// [`Owner`](crate::owner::Owner) cleans up. For a reference-counted signal that lives
/// as long as a reference to it is alive, see [`ArcReadSignal`].
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
/// # use reactive_graph::prelude::*; use reactive_graph::signal::*;  let owner = reactive_graph::owner::Owner::new(); owner.set();
/// let (count, set_count) = signal(0);
///
/// // calling .get() clones and returns the value
/// assert_eq!(count.get(), 0);
/// // calling .read() accesses the value by reference
/// assert_eq!(count.read(), 0);
/// ```
pub struct ReadSignal<T, S = SyncStorage> {
    #[cfg(debug_assertions)]
    pub(crate) defined_at: &'static Location<'static>,
    pub(crate) inner: ArenaItem<ArcReadSignal<T>, S>,
}

impl<T, S> Dispose for ReadSignal<T, S> {
    fn dispose(self) {
        self.inner.dispose()
    }
}

impl<T, S> Copy for ReadSignal<T, S> {}

impl<T, S> Clone for ReadSignal<T, S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T, S> Debug for ReadSignal<T, S>
where
    S: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReadSignal")
            .field("type", &std::any::type_name::<T>())
            .field("store", &self.inner)
            .finish()
    }
}

impl<T, S> PartialEq for ReadSignal<T, S> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<T, S> Eq for ReadSignal<T, S> {}

impl<T, S> Hash for ReadSignal<T, S> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
    }
}

impl<T, S> DefinedAt for ReadSignal<T, S> {
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

impl<T, S> IsDisposed for ReadSignal<T, S> {
    fn is_disposed(&self) -> bool {
        self.inner.is_disposed()
    }
}

impl<T, S> AsSubscriberSet for ReadSignal<T, S>
where
    S: Storage<ArcReadSignal<T>>,
{
    type Output = Arc<RwLock<SubscriberSet>>;

    fn as_subscriber_set(&self) -> Option<Self::Output> {
        self.inner
            .try_with_value(|inner| inner.as_subscriber_set())
            .flatten()
    }
}

impl<T, S> ReadUntracked for ReadSignal<T, S>
where
    T: 'static,
    S: Storage<ArcReadSignal<T>>,
{
    type Value = ReadGuard<T, Plain<T>>;

    fn try_read_untracked(&self) -> Option<Self::Value> {
        self.inner
            .try_get_value()
            .map(|inner| inner.read_untracked())
    }
}

impl<T> From<ArcReadSignal<T>> for ReadSignal<T>
where
    T: Send + Sync + 'static,
{
    #[track_caller]
    fn from(value: ArcReadSignal<T>) -> Self {
        ReadSignal {
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
            inner: ArenaItem::new_with_storage(value),
        }
    }
}

impl<T> FromLocal<ArcReadSignal<T>> for ReadSignal<T, LocalStorage>
where
    T: 'static,
{
    #[track_caller]
    fn from_local(value: ArcReadSignal<T>) -> Self {
        ReadSignal {
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
            inner: ArenaItem::new_with_storage(value),
        }
    }
}

impl<T, S> From<ReadSignal<T, S>> for ArcReadSignal<T>
where
    T: 'static,
    S: Storage<ArcReadSignal<T>>,
{
    #[track_caller]
    fn from(value: ReadSignal<T, S>) -> Self {
        value
            .inner
            .try_get_value()
            .unwrap_or_else(unwrap_signal!(value))
    }
}
