use crate::{
    arc_field::{StoreFieldReader, StoreFieldWriter},
    path::{StorePath, StorePathSegment},
    ArcField, AtIndex, StoreField, Subfield,
};
use reactive_graph::{
    owner::StoredValue,
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

pub struct Field<T>
where
    T: 'static,
{
    #[cfg(debug_assertions)]
    defined_at: &'static Location<'static>,
    inner: StoredValue<ArcField<T>>,
}

impl<T> StoreField<T> for Field<T> {
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

impl<Inner, Prev, T> From<Subfield<Inner, Prev, T>> for Field<T>
where
    T: Send + Sync,
    Subfield<Inner, Prev, T>: Clone,
    Inner: StoreField<Prev> + Send + Sync + 'static,
    Prev: 'static,
{
    #[track_caller]
    fn from(value: Subfield<Inner, Prev, T>) -> Self {
        Field {
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
            inner: StoredValue::new(value.into()),
        }
    }
}

impl<Inner, Prev> From<AtIndex<Inner, Prev>> for Field<Prev::Output>
where
    AtIndex<Inner, Prev>: Clone,
    Inner: StoreField<Prev> + Send + Sync + 'static,
    Prev: IndexMut<usize> + Send + Sync + 'static,
    Prev::Output: Sized + Send + Sync,
{
    #[track_caller]
    fn from(value: AtIndex<Inner, Prev>) -> Self {
        Field {
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
            inner: StoredValue::new(value.into()),
        }
    }
}

impl<T> Clone for Field<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Field<T> {}

impl<T> DefinedAt for Field<T> {
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

impl<T> Trigger for Field<T> {
    fn trigger(&self) {
        if let Some(inner) = self.inner.try_get_value() {
            inner.trigger();
        }
    }
}

impl<T> Track for Field<T> {
    fn track(&self) {
        if let Some(inner) = self.inner.try_get_value() {
            inner.track();
        }
    }
}

impl<T> ReadUntracked for Field<T> {
    type Value = StoreFieldReader<T>;

    fn try_read_untracked(&self) -> Option<Self::Value> {
        self.inner
            .try_get_value()
            .and_then(|inner| inner.try_read_untracked())
    }
}

impl<T> IsDisposed for Field<T> {
    fn is_disposed(&self) -> bool {
        !self.inner.exists()
    }
}
