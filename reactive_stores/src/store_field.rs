use crate::{
    path::{StorePath, StorePathSegment},
    ArcStore, Store,
};
use guardian::ArcRwLockWriteGuardian;
use or_poisoned::OrPoisoned;
use reactive_graph::{
    owner::Storage,
    signal::{
        guards::{Plain, WriteGuard},
        ArcTrigger,
    },
    traits::{DefinedAt, UntrackableGuard},
    unwrap_signal,
};
use std::{iter, ops::Deref, sync::Arc};

pub trait StoreField: Sized {
    type Value;
    type Reader: Deref<Target = Self::Value>;
    type Writer: UntrackableGuard<Target = Self::Value>;

    fn get_trigger(&self, path: StorePath) -> ArcTrigger;

    fn path(&self) -> impl IntoIterator<Item = StorePathSegment>;

    fn reader(&self) -> Option<Self::Reader>;

    fn writer(&self) -> Option<Self::Writer>;
}

impl<T> StoreField for ArcStore<T>
where
    T: 'static,
{
    type Value = T;
    type Reader = Plain<T>;
    type Writer = WriteGuard<ArcTrigger, ArcRwLockWriteGuardian<T>>;

    fn get_trigger(&self, path: StorePath) -> ArcTrigger {
        let triggers = &self.signals;
        let trigger = triggers.write().or_poisoned().get_or_insert(path);
        trigger
    }

    fn path(&self) -> impl IntoIterator<Item = StorePathSegment> {
        iter::empty()
    }

    fn reader(&self) -> Option<Self::Reader> {
        Plain::try_new(Arc::clone(&self.value))
    }

    fn writer(&self) -> Option<Self::Writer> {
        let trigger = self.get_trigger(Default::default());
        let guard =
            ArcRwLockWriteGuardian::take(Arc::clone(&self.value)).ok()?;
        Some(WriteGuard::new(trigger, guard))
    }
}

impl<T, S> StoreField for Store<T, S>
where
    T: 'static,
    S: Storage<ArcStore<T>>,
{
    type Value = T;
    type Reader = Plain<T>;
    type Writer = WriteGuard<ArcTrigger, ArcRwLockWriteGuardian<T>>;

    fn get_trigger(&self, path: StorePath) -> ArcTrigger {
        self.inner
            .try_get_value()
            .map(|n| n.get_trigger(path))
            .unwrap_or_else(unwrap_signal!(self))
    }

    fn path(&self) -> impl IntoIterator<Item = StorePathSegment> {
        self.inner
            .try_get_value()
            .map(|n| n.path().into_iter().collect::<Vec<_>>())
            .unwrap_or_else(unwrap_signal!(self))
    }

    fn reader(&self) -> Option<Self::Reader> {
        self.inner.try_get_value().and_then(|n| n.reader())
    }

    fn writer(&self) -> Option<Self::Writer> {
        self.inner.try_get_value().and_then(|n| n.writer())
    }
}
