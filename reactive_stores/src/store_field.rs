use crate::{
    path::{StorePath, StorePathSegment},
    ArcStore, KeyMap, Store, StoreFieldTrigger,
};
use or_poisoned::OrPoisoned;
use reactive_graph::{
    owner::Storage,
    signal::{
        guards::{Plain, UntrackedWriteGuard, WriteGuard},
        ArcTrigger,
    },
    traits::{Track, UntrackableGuard},
};
use std::{iter, ops::Deref, sync::Arc};

/// Describes a type that can be accessed as a reactive store field.
pub trait StoreField: Sized {
    /// The value this field contains.
    type Value;
    /// A read guard to access this field.
    type Reader: Deref<Target = Self::Value>;
    /// A write guard to update this field.
    type Writer: UntrackableGuard<Target = Self::Value>;

    /// Returns the trigger that tracks access and updates for this field.
    #[track_caller]
    fn get_trigger(&self, path: StorePath) -> StoreFieldTrigger;

    /// The path of this field (see [`StorePath`]).
    #[track_caller]
    fn path(&self) -> impl IntoIterator<Item = StorePathSegment>;

    /// Reactively tracks this field.
    #[track_caller]
    fn track_field(&self) {
        let path = self.path().into_iter().collect();
        let trigger = self.get_trigger(path);
        trigger.this.track();
        trigger.children.track();
    }

    /// Returns a read guard to access this field.
    #[track_caller]
    fn reader(&self) -> Option<Self::Reader>;

    /// Returns a write guard to update this field.
    #[track_caller]
    fn writer(&self) -> Option<Self::Writer>;

    /// The keys for this field, if it is a keyed field.
    #[track_caller]
    fn keys(&self) -> Option<KeyMap>;
}

impl<T> StoreField for ArcStore<T>
where
    T: 'static,
{
    type Value = T;
    type Reader = Plain<T>;
    type Writer = WriteGuard<ArcTrigger, UntrackedWriteGuard<T>>;

    #[track_caller]
    fn get_trigger(&self, path: StorePath) -> StoreFieldTrigger {
        let triggers = &self.signals;
        let trigger = triggers.write().or_poisoned().get_or_insert(path);
        trigger
    }

    #[track_caller]
    fn path(&self) -> impl IntoIterator<Item = StorePathSegment> {
        iter::empty()
    }

    #[track_caller]
    fn reader(&self) -> Option<Self::Reader> {
        Plain::try_new(Arc::clone(&self.value))
    }

    #[track_caller]
    fn writer(&self) -> Option<Self::Writer> {
        let trigger = self.get_trigger(Default::default());
        let guard = UntrackedWriteGuard::try_new(Arc::clone(&self.value))?;
        Some(WriteGuard::new(trigger.children, guard))
    }

    #[track_caller]
    fn keys(&self) -> Option<KeyMap> {
        Some(self.keys.clone())
    }
}

impl<T, S> StoreField for Store<T, S>
where
    T: 'static,
    S: Storage<ArcStore<T>>,
{
    type Value = T;
    type Reader = Plain<T>;
    type Writer = WriteGuard<ArcTrigger, UntrackedWriteGuard<T>>;

    #[track_caller]
    fn get_trigger(&self, path: StorePath) -> StoreFieldTrigger {
        self.inner
            .try_get_value()
            .map(|n| n.get_trigger(path))
            .unwrap_or_default()
    }

    #[track_caller]
    fn path(&self) -> impl IntoIterator<Item = StorePathSegment> {
        self.inner
            .try_get_value()
            .map(|n| n.path().into_iter().collect::<Vec<_>>())
            .unwrap_or_default()
    }

    #[track_caller]
    fn reader(&self) -> Option<Self::Reader> {
        self.inner.try_get_value().and_then(|n| n.reader())
    }

    #[track_caller]
    fn writer(&self) -> Option<Self::Writer> {
        self.inner.try_get_value().and_then(|n| n.writer())
    }

    #[track_caller]
    fn keys(&self) -> Option<KeyMap> {
        self.inner.try_get_value().and_then(|inner| inner.keys())
    }
}
