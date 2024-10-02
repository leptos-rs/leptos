use super::{subscriber_traits::AsSubscriberSet, ArcTrigger};
use crate::{
    graph::{ReactiveNode, SubscriberSet},
    owner::ArenaItem,
    traits::{DefinedAt, Dispose, IsDisposed, Notify},
};
use std::{
    fmt::{Debug, Formatter, Result},
    panic::Location,
    sync::{Arc, RwLock},
};

/// A trigger is a data-less signal with the sole purpose of notifying other reactive code of a change.
///
/// This can be useful for when using external data not stored in signals, for example.
///
/// This is an arena-allocated Trigger, which is `Copy` and is disposed when its reactive
/// [`Owner`](crate::owner::Owner) cleans up. For a reference-counted trigger that lives
/// as long as a reference to it is alive, see [`ArcTrigger`].
pub struct Trigger {
    #[cfg(debug_assertions)]
    pub(crate) defined_at: &'static Location<'static>,
    pub(crate) inner: ArenaItem<ArcTrigger>,
}

impl Trigger {
    /// Creates a new trigger.
    #[track_caller]
    pub fn new() -> Self {
        Self {
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
            inner: ArenaItem::new(ArcTrigger::new()),
        }
    }
}

impl Default for Trigger {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for Trigger {
    #[track_caller]
    fn clone(&self) -> Self {
        *self
    }
}

impl Copy for Trigger {}

impl Debug for Trigger {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_struct("Trigger").finish()
    }
}

impl Dispose for Trigger {
    fn dispose(self) {
        self.inner.dispose()
    }
}

impl IsDisposed for Trigger {
    #[inline(always)]
    fn is_disposed(&self) -> bool {
        self.inner.is_disposed()
    }
}

impl AsSubscriberSet for Trigger {
    type Output = Arc<RwLock<SubscriberSet>>;

    #[inline(always)]
    fn as_subscriber_set(&self) -> Option<Self::Output> {
        self.inner
            .try_get_value()
            .and_then(|arc_trigger| arc_trigger.as_subscriber_set())
    }
}

impl DefinedAt for Trigger {
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

impl Notify for Trigger {
    fn notify(&self) {
        if let Some(inner) = self.inner.try_get_value() {
            inner.mark_dirty();
        }
    }
}
