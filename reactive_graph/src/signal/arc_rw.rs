use super::{
    guards::{Plain, ReadGuard, UntrackedWriteGuard, WriteGuard},
    subscriber_traits::AsSubscriberSet,
    ArcReadSignal, ArcWriteSignal,
};
use crate::{
    graph::{ReactiveNode, SubscriberSet},
    prelude::{IsDisposed, Trigger},
    traits::{DefinedAt, ReadUntracked, Writeable},
};
use core::fmt::{Debug, Formatter, Result};
use std::{
    hash::Hash,
    panic::Location,
    sync::{Arc, RwLock},
};

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
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "trace", skip_all,)
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

    #[track_caller]
    pub fn read_only(&self) -> ArcReadSignal<T> {
        ArcReadSignal {
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
            value: Arc::clone(&self.value),
            inner: Arc::clone(&self.inner),
        }
    }

    #[track_caller]
    pub fn write_only(&self) -> ArcWriteSignal<T> {
        ArcWriteSignal {
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
            value: Arc::clone(&self.value),
            inner: Arc::clone(&self.inner),
        }
    }

    #[track_caller]
    pub fn split(&self) -> (ArcReadSignal<T>, ArcWriteSignal<T>) {
        (self.read_only(), self.write_only())
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

impl<T> Trigger for ArcRwSignal<T> {
    fn trigger(&self) {
        self.mark_dirty();
    }
}

impl<T> Writeable for ArcRwSignal<T> {
    type Value = T;

    fn try_write(&self) -> Option<WriteGuard<'_, Self, Self::Value>> {
        self.value
            .write()
            .ok()
            .map(|guard| WriteGuard::new(self, guard))
    }

    fn try_write_untracked(
        &self,
    ) -> Option<UntrackedWriteGuard<'_, Self::Value>> {
        self.value.write().ok().map(UntrackedWriteGuard::from)
    }
}
