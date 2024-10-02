use super::{
    guards::{Plain, ReadGuard},
    subscriber_traits::AsSubscriberSet,
    ArcReadSignal, ArcRwSignal, ArcWriteSignal, ReadSignal, WriteSignal,
};
use crate::{
    graph::{ReactiveNode, SubscriberSet},
    owner::{ArenaItem, FromLocal, LocalStorage, Storage, SyncStorage},
    signal::guards::{UntrackedWriteGuard, WriteGuard},
    traits::{
        DefinedAt, Dispose, IsDisposed, Notify, ReadUntracked,
        UntrackableGuard, Write,
    },
    unwrap_signal,
};
use core::fmt::Debug;
use guardian::ArcRwLockWriteGuardian;
use std::{
    hash::Hash,
    panic::Location,
    sync::{Arc, RwLock},
};

/// An arena-allocated signal that can be read from or written to.
///
/// A signal is a piece of data that may change over time, and notifies other
/// code when it has changed. This is the atomic unit of reactivity, which begins all other
/// processes of reactive updates.
///
/// This is an arena-allocated signal, which is `Copy` and is disposed when its reactive
/// [`Owner`](crate::owner::Owner) cleans up. For a reference-counted signal that lives
/// as long as a reference to it is alive, see [`ArcRwSignal`].
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
/// > value. If you want a non-reactive container, used [`ArenaItem`] instead.
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
pub struct RwSignal<T, S = SyncStorage> {
    #[cfg(debug_assertions)]
    defined_at: &'static Location<'static>,
    inner: ArenaItem<ArcRwSignal<T>, S>,
}

impl<T, S> Dispose for RwSignal<T, S> {
    fn dispose(self) {
        self.inner.dispose()
    }
}

impl<T> RwSignal<T>
where
    T: Send + Sync + 'static,
{
    /// Creates a new signal, taking the initial value as its argument.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "trace", skip_all)
    )]
    #[track_caller]
    pub fn new(value: T) -> Self {
        Self::new_with_storage(value)
    }
}

impl<T, S> RwSignal<T, S>
where
    T: 'static,
    S: Storage<ArcRwSignal<T>>,
{
    /// Creates a new signal with the given arena storage method.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "trace", skip_all)
    )]
    #[track_caller]
    pub fn new_with_storage(value: T) -> Self {
        Self {
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
            inner: ArenaItem::new_with_storage(ArcRwSignal::new(value)),
        }
    }
}

impl<T> RwSignal<T, LocalStorage>
where
    T: 'static,
{
    /// Creates a new signal, taking the initial value as its argument. Unlike [`RwSignal::new`],
    /// this pins the value to the current thread. Accessing it from any other thread will panic.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "trace", skip_all)
    )]
    #[track_caller]
    pub fn new_local(value: T) -> Self {
        Self::new_with_storage(value)
    }
}

impl<T, S> RwSignal<T, S>
where
    T: 'static,
    S: Storage<ArcRwSignal<T>> + Storage<ArcReadSignal<T>>,
{
    /// Returns a read-only handle to the signal.
    #[inline(always)]
    #[track_caller]
    pub fn read_only(&self) -> ReadSignal<T, S> {
        ReadSignal {
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
            inner: ArenaItem::new_with_storage(
                self.inner
                    .try_get_value()
                    .map(|inner| inner.read_only())
                    .unwrap_or_else(unwrap_signal!(self)),
            ),
        }
    }
}

impl<T, S> RwSignal<T, S>
where
    T: 'static,
    S: Storage<ArcRwSignal<T>> + Storage<ArcWriteSignal<T>>,
{
    /// Returns a write-only handle to the signal.
    #[inline(always)]
    #[track_caller]
    pub fn write_only(&self) -> WriteSignal<T, S> {
        WriteSignal {
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
            inner: ArenaItem::new_with_storage(
                self.inner
                    .try_get_value()
                    .map(|inner| inner.write_only())
                    .unwrap_or_else(unwrap_signal!(self)),
            ),
        }
    }
}

impl<T, S> RwSignal<T, S>
where
    T: 'static,
    S: Storage<ArcRwSignal<T>>
        + Storage<ArcWriteSignal<T>>
        + Storage<ArcReadSignal<T>>,
{
    /// Splits the signal into its readable and writable halves.
    #[track_caller]
    #[inline(always)]
    pub fn split(&self) -> (ReadSignal<T, S>, WriteSignal<T, S>) {
        (self.read_only(), self.write_only())
    }

    /// Reunites the two halves of a signal. Returns `None` if the two signals
    /// provided were not created from the same signal.
    #[track_caller]
    pub fn unite(
        read: ReadSignal<T, S>,
        write: WriteSignal<T, S>,
    ) -> Option<Self> {
        match (read.inner.try_get_value(), write.inner.try_get_value()) {
            (Some(read), Some(write)) => {
                if Arc::ptr_eq(&read.inner, &write.inner) {
                    Some(Self {
                        #[cfg(debug_assertions)]
                        defined_at: Location::caller(),
                        inner: ArenaItem::new_with_storage(ArcRwSignal {
                            #[cfg(debug_assertions)]
                            defined_at: Location::caller(),
                            value: Arc::clone(&read.value),
                            inner: Arc::clone(&read.inner),
                        }),
                    })
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

impl<T, S> Copy for RwSignal<T, S> {}

impl<T, S> Clone for RwSignal<T, S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T, S> Debug for RwSignal<T, S>
where
    S: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RwSignal")
            .field("type", &std::any::type_name::<T>())
            .field("store", &self.inner)
            .finish()
    }
}

impl<T, S> Default for RwSignal<T, S>
where
    T: Default + 'static,
    S: Storage<ArcRwSignal<T>>,
{
    #[track_caller]
    fn default() -> Self {
        Self::new_with_storage(T::default())
    }
}

impl<T, S> PartialEq for RwSignal<T, S> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<T, S> Eq for RwSignal<T, S> {}

impl<T, S> Hash for RwSignal<T, S> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
    }
}

impl<T, S> DefinedAt for RwSignal<T, S> {
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

impl<T: 'static, S> IsDisposed for RwSignal<T, S> {
    fn is_disposed(&self) -> bool {
        self.inner.is_disposed()
    }
}

impl<T, S> AsSubscriberSet for RwSignal<T, S>
where
    S: Storage<ArcRwSignal<T>>,
{
    type Output = Arc<RwLock<SubscriberSet>>;

    fn as_subscriber_set(&self) -> Option<Self::Output> {
        self.inner
            .try_with_value(|inner| inner.as_subscriber_set())
            .flatten()
    }
}

impl<T, S> ReadUntracked for RwSignal<T, S>
where
    T: 'static,
    S: Storage<ArcRwSignal<T>>,
{
    type Value = ReadGuard<T, Plain<T>>;

    fn try_read_untracked(&self) -> Option<Self::Value> {
        self.inner
            .try_get_value()
            .map(|inner| inner.read_untracked())
    }
}

impl<T, S> Notify for RwSignal<T, S>
where
    S: Storage<ArcRwSignal<T>>,
{
    fn notify(&self) {
        self.mark_dirty();
    }
}

impl<T, S> Write for RwSignal<T, S>
where
    T: 'static,
    S: Storage<ArcRwSignal<T>>,
{
    type Value = T;

    fn try_write(&self) -> Option<impl UntrackableGuard<Target = Self::Value>> {
        let guard = self.inner.try_with_value(|n| {
            ArcRwLockWriteGuardian::take(Arc::clone(&n.value)).ok()
        })??;
        Some(WriteGuard::new(*self, guard))
    }

    #[allow(refining_impl_trait)]
    fn try_write_untracked(&self) -> Option<UntrackedWriteGuard<Self::Value>> {
        self.inner
            .try_with_value(|n| n.try_write_untracked())
            .flatten()
    }
}

impl<T> From<ArcRwSignal<T>> for RwSignal<T>
where
    T: Send + Sync + 'static,
{
    #[track_caller]
    fn from(value: ArcRwSignal<T>) -> Self {
        RwSignal {
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
            inner: ArenaItem::new_with_storage(value),
        }
    }
}

impl<'a, T> From<&'a ArcRwSignal<T>> for RwSignal<T>
where
    T: Send + Sync + 'static,
{
    #[track_caller]
    fn from(value: &'a ArcRwSignal<T>) -> Self {
        value.clone().into()
    }
}

impl<T> FromLocal<ArcRwSignal<T>> for RwSignal<T, LocalStorage>
where
    T: 'static,
{
    #[track_caller]
    fn from_local(value: ArcRwSignal<T>) -> Self {
        RwSignal {
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
            inner: ArenaItem::new_with_storage(value),
        }
    }
}

impl<T, S> From<RwSignal<T, S>> for ArcRwSignal<T>
where
    T: 'static,
    S: Storage<ArcRwSignal<T>>,
{
    #[track_caller]
    fn from(value: RwSignal<T, S>) -> Self {
        value
            .inner
            .try_get_value()
            .unwrap_or_else(unwrap_signal!(value))
    }
}
