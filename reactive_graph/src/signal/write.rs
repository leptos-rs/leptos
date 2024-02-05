use super::ArcWriteSignal;
use crate::{
    owner::{Stored, StoredData},
    traits::{DefinedAt, IsDisposed, Trigger, UpdateUntracked},
};
use core::fmt::Debug;
use std::panic::Location;

pub struct WriteSignal<T: Send + Sync + 'static> {
    #[cfg(debug_assertions)]
    pub(crate) defined_at: &'static Location<'static>,
    pub(crate) inner: Stored<ArcWriteSignal<T>>,
}

impl<T: Send + Sync + 'static> Copy for WriteSignal<T> {}

impl<T: Send + Sync + 'static> Clone for WriteSignal<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Send + Sync + 'static> Debug for WriteSignal<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WriteSignal")
            .field("type", &std::any::type_name::<T>())
            .field("store", &self.inner)
            .finish()
    }
}

impl<T: Send + Sync + 'static> DefinedAt for WriteSignal<T> {
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

impl<T: Send + Sync + 'static> IsDisposed for WriteSignal<T> {
    fn is_disposed(&self) -> bool {
        self.inner.exists()
    }
}

impl<T: Send + Sync + 'static> StoredData for WriteSignal<T> {
    type Data = ArcWriteSignal<T>;

    fn get_value(&self) -> Option<Self::Data> {
        self.inner.get()
    }

    fn dispose(&self) {
        self.inner.dispose();
    }
}

impl<T: Send + Sync + 'static> Trigger for WriteSignal<T> {
    fn trigger(&self) {
        if let Some(inner) = self.get_value() {
            inner.trigger();
        }
    }
}

impl<T: Send + Sync + 'static> UpdateUntracked for WriteSignal<T> {
    type Value = T;

    fn try_update_untracked<U>(
        &self,
        fun: impl FnOnce(&mut Self::Value) -> U,
    ) -> Option<U> {
        self.get_value().and_then(|n| n.try_update_untracked(fun))
    }
}
