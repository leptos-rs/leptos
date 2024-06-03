use super::{
    guards::{Plain, ReadGuard},
    subscriber_traits::AsSubscriberSet,
    ArcRwSignal, ReadSignal, WriteSignal,
};
use crate::{
    graph::{ReactiveNode, SubscriberSet},
    owner::StoredValue,
    signal::guards::{UntrackedWriteGuard, WriteGuard},
    traits::{
        DefinedAt, Dispose, IsDisposed, ReadUntracked, Trigger, Writeable,
    },
    unwrap_signal,
};
use core::fmt::Debug;
use guardian::ArcRwLockWriteGuardian;
use std::{
    hash::Hash,
    ops::DerefMut,
    panic::Location,
    sync::{Arc, RwLock},
};

pub struct RwSignal<T> {
    #[cfg(debug_assertions)]
    defined_at: &'static Location<'static>,
    inner: StoredValue<ArcRwSignal<T>>,
}

impl<T: Send + Sync + 'static> Dispose for RwSignal<T> {
    fn dispose(self) {
        self.inner.dispose()
    }
}

impl<T: Send + Sync + 'static> RwSignal<T> {
    #[track_caller]
    pub fn new(value: T) -> Self {
        Self {
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
            inner: StoredValue::new(ArcRwSignal::new(value)),
        }
    }

    #[inline(always)]
    #[track_caller]
    pub fn read_only(&self) -> ReadSignal<T> {
        ReadSignal {
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
            inner: StoredValue::new(
                self.inner
                    .get()
                    .map(|inner| inner.read_only())
                    .unwrap_or_else(unwrap_signal!(self)),
            ),
        }
    }

    #[inline(always)]
    #[track_caller]
    pub fn write_only(&self) -> WriteSignal<T> {
        WriteSignal {
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
            inner: StoredValue::new(
                self.inner
                    .get()
                    .map(|inner| inner.write_only())
                    .unwrap_or_else(unwrap_signal!(self)),
            ),
        }
    }

    #[track_caller]
    #[inline(always)]
    pub fn split(&self) -> (ReadSignal<T>, WriteSignal<T>) {
        (self.read_only(), self.write_only())
    }

    #[track_caller]
    pub fn unite(read: ReadSignal<T>, write: WriteSignal<T>) -> Option<Self> {
        match (read.inner.get(), write.inner.get()) {
            (Some(read), Some(write)) => {
                if Arc::ptr_eq(&read.inner, &write.inner) {
                    Some(Self {
                        #[cfg(debug_assertions)]
                        defined_at: Location::caller(),
                        inner: StoredValue::new(ArcRwSignal {
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

impl<T> Copy for RwSignal<T> {}

impl<T> Clone for RwSignal<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Debug for RwSignal<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RwSignal")
            .field("type", &std::any::type_name::<T>())
            .field("store", &self.inner)
            .finish()
    }
}

impl<T> PartialEq for RwSignal<T> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<T> Eq for RwSignal<T> {}

impl<T> Hash for RwSignal<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
    }
}

impl<T> DefinedAt for RwSignal<T> {
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

impl<T: 'static> IsDisposed for RwSignal<T> {
    fn is_disposed(&self) -> bool {
        self.inner.exists()
    }
}

impl<T: 'static> AsSubscriberSet for RwSignal<T> {
    type Output = Arc<RwLock<SubscriberSet>>;

    fn as_subscriber_set(&self) -> Option<Self::Output> {
        self.inner
            .try_with_value(|inner| inner.as_subscriber_set())
            .flatten()
    }
}

impl<T: 'static> ReadUntracked for RwSignal<T> {
    type Value = ReadGuard<T, Plain<T>>;

    fn try_read_untracked(&self) -> Option<Self::Value> {
        self.inner.get().map(|inner| inner.read_untracked())
    }
}

impl<T: 'static> Trigger for RwSignal<T> {
    fn trigger(&self) {
        self.mark_dirty();
    }
}

impl<T: 'static> Writeable for RwSignal<T> {
    type Value = T;

    fn try_write(
        &self,
    ) -> Option<WriteGuard<'_, Self, impl DerefMut<Target = Self::Value>>> {
        let guard = self.inner.try_with_value(|n| {
            ArcRwLockWriteGuardian::take(Arc::clone(&n.value)).ok()
        })??;
        Some(WriteGuard::new(self, guard))
    }

    fn try_write_untracked(&self) -> Option<UntrackedWriteGuard<Self::Value>> {
        self.inner.with_value(|n| n.try_write_untracked())
    }
}

impl<T: Send + Sync + 'static> From<ArcRwSignal<T>> for RwSignal<T> {
    #[track_caller]
    fn from(value: ArcRwSignal<T>) -> Self {
        RwSignal {
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
            inner: StoredValue::new(value),
        }
    }
}

impl<'a, T: Send + Sync + 'static> From<&'a ArcRwSignal<T>> for RwSignal<T> {
    #[track_caller]
    fn from(value: &'a ArcRwSignal<T>) -> Self {
        value.clone().into()
    }
}

impl<T: Send + Sync + 'static> From<RwSignal<T>> for ArcRwSignal<T> {
    #[track_caller]
    fn from(value: RwSignal<T>) -> Self {
        value.inner.get().unwrap_or_else(unwrap_signal!(value))
    }
}
