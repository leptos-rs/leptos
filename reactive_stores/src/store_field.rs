use crate::{
    path::{StorePath, StorePathSegment},
    ArcStore,
};
use guardian::ArcRwLockWriteGuardian;
use or_poisoned::OrPoisoned;
use reactive_graph::{
    signal::{
        guards::{Plain, WriteGuard},
        ArcTrigger,
    },
    traits::UntrackableGuard,
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

    fn path(&self) -> impl Iterator<Item = StorePathSegment>;

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

    fn path(&self) -> impl Iterator<Item = StorePathSegment> {
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
