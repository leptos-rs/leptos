use super::subscriber_traits::AsSubscriberSet;
use crate::{
    graph::{ReactiveNode, SubscriberSet},
    traits::{DefinedAt, IsDisposed, Trigger},
};
use std::{
    fmt::{Debug, Formatter, Result},
    panic::Location,
    sync::{Arc, RwLock},
};

pub struct ArcTrigger {
    #[cfg(debug_assertions)]
    pub(crate) defined_at: &'static Location<'static>,
    pub(crate) inner: Arc<RwLock<SubscriberSet>>,
}

impl ArcTrigger {
    #[track_caller]
    pub fn new() -> Self {
        Self {
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
            inner: Default::default(),
        }
    }
}

impl Default for ArcTrigger {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for ArcTrigger {
    #[track_caller]
    fn clone(&self) -> Self {
        Self {
            #[cfg(debug_assertions)]
            defined_at: self.defined_at,
            inner: Arc::clone(&self.inner),
        }
    }
}

impl Debug for ArcTrigger {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_struct("ArcTrigger").finish()
    }
}

impl IsDisposed for ArcTrigger {
    #[inline(always)]
    fn is_disposed(&self) -> bool {
        false
    }
}

impl AsSubscriberSet for ArcTrigger {
    type Output = Arc<RwLock<SubscriberSet>>;

    #[inline(always)]
    fn as_subscriber_set(&self) -> Option<Self::Output> {
        Some(Arc::clone(&self.inner))
    }
}

impl DefinedAt for ArcTrigger {
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

impl Trigger for ArcTrigger {
    fn trigger(&self) {
        self.inner.mark_dirty();
    }
}
