use crate::{
    signal::guards::{Plain, ReadGuard, UntrackedWriteGuard},
    traits::{DefinedAt, IsDisposed, ReadValue, WriteValue},
};
use std::{
    fmt::{Debug, Formatter},
    hash::Hash,
    panic::Location,
    sync::{Arc, RwLock},
};

/// A reference-counted getter for any value non-reactively.
///
/// This is a reference-counted value, which is `Clone` but not `Copy`.
/// For arena-allocated `Copy` values, use [`StoredValue`](super::StoredValue).
///
/// This allows you to create a stable reference for any value by storing it within
/// the reactive system. Unlike e.g. [`ArcRwSignal`](crate::signal::ArcRwSignal), it is not reactive;
/// accessing it does not cause effects to subscribe, and
/// updating it does not notify anything else.
pub struct ArcStoredValue<T> {
    #[cfg(debug_assertions)]
    defined_at: &'static Location<'static>,
    value: Arc<RwLock<T>>,
}

impl<T> Clone for ArcStoredValue<T> {
    fn clone(&self) -> Self {
        Self {
            #[cfg(debug_assertions)]
            defined_at: self.defined_at,
            value: Arc::clone(&self.value),
        }
    }
}

impl<T> Debug for ArcStoredValue<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ArcStoredValue")
            .field("type", &std::any::type_name::<T>())
            .field("value", &Arc::as_ptr(&self.value))
            .finish()
    }
}

impl<T: Default> Default for ArcStoredValue<T> {
    #[track_caller]
    fn default() -> Self {
        Self {
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
            value: Arc::new(RwLock::new(T::default())),
        }
    }
}

impl<T> PartialEq for ArcStoredValue<T> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.value, &other.value)
    }
}

impl<T> Eq for ArcStoredValue<T> {}

impl<T> Hash for ArcStoredValue<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::ptr::hash(&Arc::as_ptr(&self.value), state);
    }
}

impl<T> DefinedAt for ArcStoredValue<T> {
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

impl<T> ArcStoredValue<T> {
    /// Creates a new stored value, taking the initial value as its argument.
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
        }
    }
}

impl<T> ReadValue for ArcStoredValue<T>
where
    T: 'static,
{
    type Value = ReadGuard<T, Plain<T>>;

    fn try_read_value(&self) -> Option<ReadGuard<T, Plain<T>>> {
        Plain::try_new(Arc::clone(&self.value)).map(ReadGuard::new)
    }
}

impl<T> WriteValue for ArcStoredValue<T>
where
    T: 'static,
{
    type Value = T;

    fn try_write_value(&self) -> Option<UntrackedWriteGuard<T>> {
        UntrackedWriteGuard::try_new(self.value.clone())
    }
}

impl<T> IsDisposed for ArcStoredValue<T> {
    fn is_disposed(&self) -> bool {
        false
    }
}
