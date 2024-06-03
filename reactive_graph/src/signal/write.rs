use super::{
    guards::{UntrackedWriteGuard, WriteGuard},
    ArcWriteSignal,
};
use crate::{
    owner::StoredValue,
    traits::{DefinedAt, Dispose, IsDisposed, Trigger, Writeable},
};
use core::fmt::Debug;
use guardian::ArcRwLockWriteGuardian;
use std::{hash::Hash, ops::DerefMut, panic::Location, sync::Arc};

pub struct WriteSignal<T> {
    #[cfg(debug_assertions)]
    pub(crate) defined_at: &'static Location<'static>,
    pub(crate) inner: StoredValue<ArcWriteSignal<T>>,
}

impl<T> Dispose for WriteSignal<T> {
    fn dispose(self) {
        self.inner.dispose()
    }
}

impl<T> Copy for WriteSignal<T> {}

impl<T> Clone for WriteSignal<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Debug for WriteSignal<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WriteSignal")
            .field("type", &std::any::type_name::<T>())
            .field("store", &self.inner)
            .finish()
    }
}

impl<T> PartialEq for WriteSignal<T> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<T> Eq for WriteSignal<T> {}

impl<T> Hash for WriteSignal<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
    }
}

impl<T> DefinedAt for WriteSignal<T> {
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

impl<T: 'static> IsDisposed for WriteSignal<T> {
    fn is_disposed(&self) -> bool {
        self.inner.exists()
    }
}

impl<T: 'static> Trigger for WriteSignal<T> {
    fn trigger(&self) {
        if let Some(inner) = self.inner.get() {
            inner.trigger();
        }
    }
}

impl<T: 'static> Writeable for WriteSignal<T> {
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
