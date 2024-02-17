use super::guards::{UntrackedWriteGuard, WriteGuard};
use crate::{
    graph::{ReactiveNode, SubscriberSet},
    prelude::{IsDisposed, Trigger},
    traits::{DefinedAt, Writeable},
};
use core::fmt::{Debug, Formatter, Result};
use std::{
    panic::Location,
    sync::{Arc, RwLock},
};

pub struct ArcWriteSignal<T> {
    #[cfg(debug_assertions)]
    pub(crate) defined_at: &'static Location<'static>,
    pub(crate) value: Arc<RwLock<T>>,
    pub(crate) inner: Arc<RwLock<SubscriberSet>>,
}

impl<T> Clone for ArcWriteSignal<T> {
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

impl<T> Debug for ArcWriteSignal<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_struct("ArcWriteSignal")
            .field("type", &std::any::type_name::<T>())
            .field("value", &Arc::as_ptr(&self.value))
            .finish()
    }
}

impl<T> ArcWriteSignal<T> {
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "trace", skip_all,)
    )]
    pub fn new(value: T) -> Self {
        Self {
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
            value: Arc::new(RwLock::new(value)),
            inner: Arc::new(RwLock::new(SubscriberSet::new())),
        }
    }
}

impl<T> DefinedAt for ArcWriteSignal<T> {
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

impl<T> IsDisposed for ArcWriteSignal<T> {
    #[inline(always)]
    fn is_disposed(&self) -> bool {
        false
    }
}

impl<T> Trigger for ArcWriteSignal<T> {
    fn trigger(&self) {
        self.inner.mark_dirty();
    }
}

impl<T> Writeable for ArcWriteSignal<T> {
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
