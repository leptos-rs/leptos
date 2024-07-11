use crate::{
    path::{StorePath, StorePathSegment},
    ArcStore, Store,
};
use guardian::ArcRwLockWriteGuardian;
use or_poisoned::OrPoisoned;
use reactive_graph::{
    signal::{
        guards::{Plain, WriteGuard},
        ArcTrigger,
    },
    traits::{DefinedAt, UntrackableGuard},
    unwrap_signal,
};
use std::{
    iter,
    ops::Deref,
    sync::{Arc, RwLock},
};

pub trait StoreField<T>: Sized {
    type Orig;
    type Reader: Deref<Target = T>;
    type Writer: UntrackableGuard<Target = T>;

    fn data(&self) -> Arc<RwLock<Self::Orig>>;

    fn get_trigger(&self, path: StorePath) -> ArcTrigger;

    fn path(&self) -> impl IntoIterator<Item = StorePathSegment>;

    fn reader(&self) -> Option<Self::Reader>;

    fn writer(&self) -> Option<Self::Writer>;
}

impl<T> StoreField<T> for ArcStore<T>
where
    T: 'static,
{
    type Orig = T;
    type Reader = Plain<T>;
    type Writer = WriteGuard<ArcTrigger, ArcRwLockWriteGuardian<T>>;

    fn data(&self) -> Arc<RwLock<Self::Orig>> {
        Arc::clone(&self.value)
    }

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

impl<T> StoreField<T> for Store<T>
where
    T: 'static,
{
    type Orig = T;
    type Reader = Plain<T>;
    type Writer = WriteGuard<ArcTrigger, ArcRwLockWriteGuardian<T>>;

    fn data(&self) -> Arc<RwLock<Self::Orig>> {
        self.inner
            .try_get_value()
            .map(|n| n.data())
            .unwrap_or_else(unwrap_signal!(self))
    }

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
