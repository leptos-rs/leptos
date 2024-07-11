use reactive_graph::{
    signal::{
        guards::{Mapped, Plain, ReadGuard},
        ArcTrigger,
    },
    traits::{DefinedAt, ReadUntracked, Track},
};
use std::{
    panic::Location,
    sync::{Arc, RwLock},
};

pub struct ArcReadStoreField<Orig, T>
where
    T: 'static,
{
    #[cfg(debug_assertions)]
    defined_at: &'static Location<'static>,
    data: Arc<RwLock<Orig>>,
    trigger: ArcTrigger,
    read: fn(&Orig) -> &T,
}

impl<Orig, T> Clone for ArcReadStoreField<Orig, T> {
    fn clone(&self) -> Self {
        Self {
            #[cfg(debug_assertions)]
            defined_at: self.defined_at,
            data: Arc::clone(&self.data),
            trigger: self.trigger.clone(),
            read: self.read,
        }
    }
}

impl<Orig, T> DefinedAt for ArcReadStoreField<Orig, T> {
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

impl<Orig, T> Track for ArcReadStoreField<Orig, T> {
    fn track(&self) {
        self.trigger.track();
    }
}

impl<Orig, T> ReadUntracked for ArcReadStoreField<Orig, T>
where
    Orig: 'static,
{
    type Value = ReadGuard<T, Mapped<Plain<Orig>, T>>;

    fn try_read_untracked(&self) -> Option<Self::Value> {
        Mapped::try_new(Arc::clone(&self.data), self.read).map(ReadGuard::new)
    }
}
