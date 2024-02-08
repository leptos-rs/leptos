use super::{
    subscriber_traits::AsSubscriberSet, ArcRwSignal, ReadSignal,
    SignalReadGuard, WriteSignal,
};
use crate::{
    graph::{ReactiveNode, SubscriberSet},
    owner::{Stored, StoredData},
    traits::{DefinedAt, IsDisposed, Readable, Trigger, UpdateUntracked},
    unwrap_signal,
};
use core::fmt::Debug;
use std::{
    panic::Location,
    sync::{Arc, RwLock},
};

pub struct RwSignal<T: Send + Sync + 'static> {
    #[cfg(debug_assertions)]
    defined_at: &'static Location<'static>,
    inner: Stored<ArcRwSignal<T>>,
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
            inner: Stored::new(ArcRwSignal::new(value)),
        }
    }

    #[inline(always)]
    pub fn read_only(&self) -> ReadSignal<T> {
        ReadSignal {
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
            inner: Stored::new(
                self.get_value()
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
            inner: Stored::new(
                self.get_value()
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
                        inner: Stored::new(ArcRwSignal {
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

impl<T: Send + Sync + 'static> Copy for RwSignal<T> {}

impl<T: Send + Sync + 'static> Clone for RwSignal<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Send + Sync + 'static> Debug for RwSignal<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RwSignal")
            .field("type", &std::any::type_name::<T>())
            .field("store", &self.inner)
            .finish()
    }
}

impl<T: Send + Sync + 'static> DefinedAt for RwSignal<T> {
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

impl<T: Send + Sync + 'static> StoredData for RwSignal<T> {
    type Data = ArcRwSignal<T>;

    fn get_value(&self) -> Option<Self::Data> {
        self.inner.get()
    }

    fn dispose(&self) {
        self.inner.dispose();
    }
}

impl<T: Send + Sync + 'static> AsSubscriberSet for RwSignal<T> {
    type Output = Arc<RwLock<SubscriberSet>>;

    fn as_subscriber_set(&self) -> Option<Self::Output> {
        self.inner
            .with_value(|inner| inner.as_subscriber_set())
            .flatten()
    }
}

impl<T: Send + Sync + 'static> Readable for RwSignal<T> {
    type Value = SignalReadGuard<T>;

    fn try_read(&self) -> Option<Self::Value> {
        self.get_value().map(|inner| inner.read())
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
        self.get_value().and_then(|n| n.try_update_untracked(fun))
    }
}
