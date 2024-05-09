use super::{
    guards::{Plain, ReadGuard},
    subscriber_traits::AsSubscriberSet,
    ArcRwSignal, ReadSignal, WriteSignal,
};
use crate::{
    graph::{ReactiveNode, SubscriberSet},
    owner::StoredValue,
    traits::{
        DefinedAt, Dispose, IsDisposed, ReadUntracked, Trigger, UpdateUntracked,
    },
    unwrap_signal,
};
use core::fmt::Debug;
use std::{
    hash::Hash,
    panic::Location,
    sync::{Arc, RwLock},
};

pub struct RwSignal<T: 'static> {
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
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "trace", skip_all,)
    )]
    pub fn new(value: T) -> Self {
        Self {
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
            inner: StoredValue::new(ArcRwSignal::new(value)),
        }
    }

    #[inline(always)]
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

impl<T: 'static> Copy for RwSignal<T> {}

impl<T: 'static> Clone for RwSignal<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: 'static> Debug for RwSignal<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RwSignal")
            .field("type", &std::any::type_name::<T>())
            .field("store", &self.inner)
            .finish()
    }
}

impl<T: 'static> PartialEq for RwSignal<T> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<T: 'static> Eq for RwSignal<T> {}

impl<T: 'static> Hash for RwSignal<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
    }
}

impl<T: 'static> DefinedAt for RwSignal<T> {
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

impl<T: Send + Sync + 'static> IsDisposed for RwSignal<T> {
    fn is_disposed(&self) -> bool {
        self.inner.exists()
    }
}

impl<T: Send + Sync + 'static> AsSubscriberSet for RwSignal<T> {
    type Output = Arc<RwLock<SubscriberSet>>;

    fn as_subscriber_set(&self) -> Option<Self::Output> {
        self.inner
            .try_with_value(|inner| inner.as_subscriber_set())
            .flatten()
    }
}

impl<T: Send + Sync + 'static> ReadUntracked for RwSignal<T> {
    type Value = ReadGuard<T, Plain<T>>;

    fn try_read_untracked(&self) -> Option<Self::Value> {
        self.inner.get().map(|inner| inner.read_untracked())
    }
}

impl<T: Send + Sync + 'static> Trigger for RwSignal<T> {
    fn trigger(&self) {
        self.mark_dirty();
    }
}

impl<T: Send + Sync + 'static> UpdateUntracked for RwSignal<T> {
    type Value = T;

    fn try_update_untracked<U>(
        &self,
        fun: impl FnOnce(&mut Self::Value) -> U,
    ) -> Option<U> {
        self.inner.get().and_then(|n| n.try_update_untracked(fun))
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
