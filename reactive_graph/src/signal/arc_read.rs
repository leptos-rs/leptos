use super::{subscriber_traits::AsSubscriberSet, SignalReadGuard};
use crate::{
    graph::SubscriberSet,
    traits::{DefinedAt, IsDisposed, ReadUntracked},
};
use core::fmt::{Debug, Formatter, Result};
use std::{
    panic::Location,
    sync::{Arc, RwLock},
};

pub struct ArcReadSignal<T> {
    #[cfg(debug_assertions)]
    pub(crate) defined_at: &'static Location<'static>,
    pub(crate) value: Arc<RwLock<T>>,
    pub(crate) inner: Arc<RwLock<SubscriberSet>>,
}

impl<T> Clone for ArcReadSignal<T> {
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

impl<T> Debug for ArcReadSignal<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_struct("ArcReadSignal")
            .field("type", &std::any::type_name::<T>())
            .field("value", &Arc::as_ptr(&self.value))
            .finish()
    }
}

impl<T> ArcReadSignal<T> {
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

impl<T> DefinedAt for ArcReadSignal<T> {
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

impl<T> IsDisposed for ArcReadSignal<T> {
    #[inline(always)]
    fn is_disposed(&self) -> bool {
        false
    }
}

impl<T> AsSubscriberSet for ArcReadSignal<T> {
    type Output = Arc<RwLock<SubscriberSet>>;

    #[inline(always)]
    fn as_subscriber_set(&self) -> Option<Self::Output> {
        Some(Arc::clone(&self.inner))
    }
}

impl<T: 'static> ReadUntracked for ArcReadSignal<T> {
    type Value = SignalReadGuard<T>;

    fn try_read_untracked(&self) -> Option<Self::Value> {
        SignalReadGuard::try_new(Arc::clone(&self.value))
    }
}
