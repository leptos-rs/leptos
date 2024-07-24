use crate::{
    arc_field::{StoreFieldReader, StoreFieldWriter},
    path::{StorePath, StorePathSegment},
    ArcField, AtIndex, StoreField, Subfield,
};
use reactive_graph::{
    owner::{Storage, StoredValue, SyncStorage},
    signal::ArcTrigger,
    traits::{
        DefinedAt, IsDisposed, ReadUntracked, Track, Trigger, UntrackableGuard,
    },
    unwrap_signal,
};
use std::{
    ops::{Deref, DerefMut, IndexMut},
    panic::Location,
    sync::Arc,
};

pub struct Field<T, S = SyncStorage>
where
    T: 'static,
{
    #[cfg(debug_assertions)]
    defined_at: &'static Location<'static>,
    inner: StoredValue<ArcField<T>, S>,
}

impl<T, S> StoreField<T> for Field<T, S>
where
    S: Storage<ArcField<T>>,
{
    type Reader = StoreFieldReader<T>;
    type Writer = StoreFieldWriter<T>;

    fn get_trigger(&self, path: StorePath) -> ArcTrigger {
        self.inner
            .try_get_value()
            .map(|inner| inner.get_trigger(path))
            .unwrap_or_else(unwrap_signal!(self))
    }

    fn path(&self) -> impl IntoIterator<Item = StorePathSegment> {
        self.inner
            .try_get_value()
            .map(|inner| inner.path().into_iter().collect::<Vec<_>>())
            .unwrap_or_else(unwrap_signal!(self))
    }

    fn reader(&self) -> Option<Self::Reader> {
        self.inner.try_get_value().and_then(|inner| inner.reader())
    }

    fn writer(&self) -> Option<Self::Writer> {
        self.inner.try_get_value().and_then(|inner| inner.writer())
    }
}

impl<Inner, Prev, T, S> From<Subfield<Inner, Prev, T>> for Field<T, S>
where
    T: Send + Sync,
    S: Storage<ArcField<T>>,
    Subfield<Inner, Prev, T>: Clone,
    Inner: StoreField<Prev> + Send + Sync + 'static,
    Prev: 'static,
{
    #[track_caller]
    fn from(value: Subfield<Inner, Prev, T>) -> Self {
        Field {
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
            inner: StoredValue::new_with_storage(value.into()),
        }
    }
}

impl<Inner, Prev, S> From<AtIndex<Inner, Prev>> for Field<Prev::Output, S>
where
    AtIndex<Inner, Prev>: Clone,
    S: Storage<ArcField<Prev::Output>>,
    Inner: StoreField<Prev> + Send + Sync + 'static,
    Prev: IndexMut<usize> + Send + Sync + 'static,
    Prev::Output: Sized + Send + Sync,
{
    #[track_caller]
    fn from(value: AtIndex<Inner, Prev>) -> Self {
        Field {
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
            inner: StoredValue::new_with_storage(value.into()),
        }
    }
}

impl<T, S> Clone for Field<T, S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T, S> Copy for Field<T, S> {}

impl<T, S> DefinedAt for Field<T, S> {
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

impl<T, S> Trigger for Field<T, S>
where
    S: Storage<ArcField<T>>,
{
    fn trigger(&self) {
        if let Some(inner) = self.inner.try_get_value() {
            inner.trigger();
        }
    }
}

impl<T, S> Track for Field<T, S>
where
    S: Storage<ArcField<T>>,
{
    fn track(&self) {
        if let Some(inner) = self.inner.try_get_value() {
            inner.track();
        }
    }
}

impl<T, S> ReadUntracked for Field<T, S>
where
    S: Storage<ArcField<T>>,
{
    type Value = StoreFieldReader<T>;

    fn try_read_untracked(&self) -> Option<Self::Value> {
        self.inner
            .try_get_value()
            .and_then(|inner| inner.try_read_untracked())
    }
}

impl<T, S> IsDisposed for Field<T, S> {
    fn is_disposed(&self) -> bool {
        self.inner.is_disposed()
    }
}
