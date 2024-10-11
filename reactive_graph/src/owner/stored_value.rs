use super::{
    arc_stored_value::ArcStoredValue, ArenaItem, LocalStorage, Storage,
    SyncStorage,
};
use crate::{
    signal::guards::{Plain, ReadGuard, UntrackedWriteGuard},
    traits::{DefinedAt, Dispose, IsDisposed, ReadValue, WriteValue},
    unwrap_signal,
};
use std::{
    fmt::{Debug, Formatter},
    hash::Hash,
    panic::Location,
};

/// A **non-reactive**, `Copy` handle for any value.
///
/// This allows you to create a stable reference for any value by storing it within
/// the reactive system. Like the signal types (e.g., [`ReadSignal`](crate::signal::ReadSignal)
/// and [`RwSignal`](crate::signal::RwSignal)), it is `Copy` and `'static`. Unlike the signal
/// types, it is not reactive; accessing it does not cause effects to subscribe, and
/// updating it does not notify anything else.
pub struct StoredValue<T, S = SyncStorage> {
    value: ArenaItem<ArcStoredValue<T>, S>,
    #[cfg(debug_assertions)]
    defined_at: &'static Location<'static>,
}

impl<T, S> Copy for StoredValue<T, S> {}

impl<T, S> Clone for StoredValue<T, S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T, S> Debug for StoredValue<T, S>
where
    S: Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("StoredValue")
            .field("type", &std::any::type_name::<T>())
            .field("value", &self.value)
            .finish()
    }
}

impl<T, S> PartialEq for StoredValue<T, S> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<T, S> Eq for StoredValue<T, S> {}

impl<T, S> Hash for StoredValue<T, S> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.value.hash(state);
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
    S: Storage<ArcStoredValue<T>>,
{
    /// Stores the given value in the arena allocator.
    #[track_caller]
    pub fn new_with_storage(value: T) -> Self {
        Self {
            value: ArenaItem::new_with_storage(ArcStoredValue::new(value)),
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
        }
    }
}

impl<T, S> Default for StoredValue<T, S>
where
    T: Default + 'static,
    S: Storage<ArcStoredValue<T>>,
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

impl<T, S> ReadValue for StoredValue<T, S>
where
    T: 'static,
    S: Storage<ArcStoredValue<T>>,
{
    type Value = ReadGuard<T, Plain<T>>;

    fn try_read_value(&self) -> Option<ReadGuard<T, Plain<T>>> {
        self.value
            .try_get_value()
            .and_then(|inner| inner.try_read_value())
    }
}

impl<T, S> WriteValue for StoredValue<T, S>
where
    T: 'static,
    S: Storage<ArcStoredValue<T>>,
{
    type Value = T;

    fn try_write_value(&self) -> Option<UntrackedWriteGuard<T>> {
        self.value
            .try_get_value()
            .and_then(|inner| inner.try_write_value())
    }
}

impl<T, S> IsDisposed for StoredValue<T, S> {
    fn is_disposed(&self) -> bool {
        self.value.is_disposed()
    }
}

impl<T, S> Dispose for StoredValue<T, S> {
    fn dispose(self) {
        self.value.dispose();
    }
}

impl<T> From<ArcStoredValue<T>> for StoredValue<T>
where
    T: Send + Sync + 'static,
{
    #[track_caller]
    fn from(value: ArcStoredValue<T>) -> Self {
        StoredValue {
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
            value: ArenaItem::new(value),
        }
    }
}

impl<T, S> From<StoredValue<T, S>> for ArcStoredValue<T>
where
    S: Storage<ArcStoredValue<T>>,
{
    #[track_caller]
    fn from(value: StoredValue<T, S>) -> Self {
        value
            .value
            .try_get_value()
            .unwrap_or_else(unwrap_signal!(value))
    }
}

/// Creates a new [`StoredValue`].
#[inline(always)]
#[track_caller]
#[deprecated(
    since = "0.7.0-beta5",
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
