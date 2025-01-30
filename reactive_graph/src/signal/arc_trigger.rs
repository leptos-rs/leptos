use super::subscriber_traits::AsSubscriberSet;
use crate::{
    graph::{ReactiveNode, SubscriberSet},
    traits::{DefinedAt, IsDisposed, Notify, Track},
};
use std::{
    fmt::{Debug, Formatter, Result},
    panic::Location,
    sync::{Arc, RwLock},
};

/// A trigger is a data-less signal with the sole purpose of notifying other reactive code of a change.
///
/// This can be useful for when using external data not stored in signals, for example.
pub struct ArcTrigger {
    #[cfg(any(debug_assertions, leptos_debuginfo))]
    pub(crate) defined_at: &'static Location<'static>,
    pub(crate) inner: Arc<RwLock<SubscriberSet>>,
}

impl ArcTrigger {
    /// Creates a new trigger.
    #[track_caller]
    pub fn new() -> Self {
        Self {
            #[cfg(any(debug_assertions, leptos_debuginfo))]
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
            #[cfg(any(debug_assertions, leptos_debuginfo))]
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

impl Notify for Vec<ArcTrigger> {
    fn notify(&self) {
        for trigger in self {
            trigger.notify();
        }
    }
}

impl Track for Vec<ArcTrigger> {
    fn track(&self) {
        for trigger in self {
            trigger.track();
        }
    }
}

impl DefinedAt for ArcTrigger {
    #[inline(always)]
    fn defined_at(&self) -> Option<&'static Location<'static>> {
        #[cfg(any(debug_assertions, leptos_debuginfo))]
        {
            Some(self.defined_at)
        }
        #[cfg(not(any(debug_assertions, leptos_debuginfo)))]
        {
            None
        }
    }
}

impl Notify for ArcTrigger {
    fn notify(&self) {
        self.inner.mark_dirty();
    }
}
