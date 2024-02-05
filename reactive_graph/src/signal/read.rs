use super::{subscriber_traits::AsSubscriberSet, ArcReadSignal};
use crate::{
    graph::SubscriberSet,
    owner::{Stored, StoredData},
    traits::{DefinedAt, IsDisposed, WithUntracked},
};
use core::fmt::Debug;
use std::{
    panic::Location,
    sync::{Arc, RwLock},
};

pub struct ReadSignal<T: Send + Sync + 'static> {
    #[cfg(debug_assertions)]
    pub(crate) defined_at: &'static Location<'static>,
    pub(crate) inner: Stored<ArcReadSignal<T>>,
}

impl<T: Send + Sync + 'static> Copy for ReadSignal<T> {}

impl<T: Send + Sync + 'static> Clone for ReadSignal<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Send + Sync + 'static> Debug for ReadSignal<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReadSignal")
            .field("type", &std::any::type_name::<T>())
            .field("store", &self.inner)
            .finish()
    }
}

impl<T: Send + Sync + 'static> DefinedAt for ReadSignal<T> {
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

impl<T: Send + Sync + 'static> IsDisposed for ReadSignal<T> {
    fn is_disposed(&self) -> bool {
        self.inner.exists()
    }
}

impl<T: Send + Sync + 'static> StoredData for ReadSignal<T> {
    type Data = ArcReadSignal<T>;

    fn get_value(&self) -> Option<Self::Data> {
        self.inner.get()
    }

    fn dispose(&self) {
        self.inner.dispose();
    }
}

impl<T: Send + Sync + 'static> AsSubscriberSet for ReadSignal<T> {
    type Output = Arc<RwLock<SubscriberSet>>;

    fn as_subscriber_set(&self) -> Option<Self::Output> {
        self.inner
            .with_value(|inner| inner.as_subscriber_set())
            .flatten()
    }
}

impl<T: Send + Sync + 'static> WithUntracked for ReadSignal<T> {
    type Value = T;

    fn try_with_untracked<U>(
        &self,
        fun: impl FnOnce(&Self::Value) -> U,
    ) -> Option<U> {
        self.get_value().and_then(|n| n.try_with_untracked(fun))
    }
}
