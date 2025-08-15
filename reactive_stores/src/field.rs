use crate::{
    arc_field::{StoreFieldReader, StoreFieldWriter},
    path::{StorePath, StorePathSegment},
    ArcField, ArcStore, AtIndex, AtKeyed, DerefedField, KeyMap, KeyedSubfield,
    Store, StoreField, StoreFieldTrigger, Subfield,
};
use reactive_graph::{
    owner::{ArenaItem, Storage, SyncStorage},
    traits::{
        DefinedAt, IsDisposed, Notify, ReadUntracked, Track, UntrackableGuard,
        Write,
    },
};
use std::{
    fmt::Debug,
    hash::Hash,
    ops::{Deref, DerefMut, IndexMut},
    panic::Location,
};

/// Wraps access to a single field of type `T`.
///
/// This can be used to erase the chain of field-accessors, to make it easier to pass this into
/// another component or function without needing to specify the full type signature.
pub struct Field<T, S = SyncStorage>
where
    T: 'static,
{
    #[cfg(any(debug_assertions, leptos_debuginfo))]
    defined_at: &'static Location<'static>,
    inner: ArenaItem<ArcField<T>, S>,
}

impl<T, S> Debug for Field<T, S>
where
    T: 'static,
    S: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut f = f.debug_struct("Field");
        #[cfg(any(debug_assertions, leptos_debuginfo))]
        let f = f.field("defined_at", &self.defined_at);
        f.field("inner", &self.inner).finish()
    }
}

impl<T, S> StoreField for Field<T, S>
where
    S: Storage<ArcField<T>>,
{
    type Value = T;
    type Reader = StoreFieldReader<T>;
    type Writer = StoreFieldWriter<T>;

    fn get_trigger(&self, path: StorePath) -> StoreFieldTrigger {
        self.inner
            .try_get_value()
            .map(|inner| inner.get_trigger(path))
            .unwrap_or_default()
    }

    fn path(&self) -> impl IntoIterator<Item = StorePathSegment> {
        self.inner
            .try_get_value()
            .map(|inner| inner.path().into_iter().collect::<Vec<_>>())
            .unwrap_or_default()
    }

    fn reader(&self) -> Option<Self::Reader> {
        self.inner.try_get_value().and_then(|inner| inner.reader())
    }

    fn writer(&self) -> Option<Self::Writer> {
        self.inner.try_get_value().and_then(|inner| inner.writer())
    }

    fn keys(&self) -> Option<KeyMap> {
        self.inner.try_get_value().and_then(|n| n.keys())
    }
}

impl<T, S> From<Store<T, S>> for Field<T, S>
where
    T: 'static,
    S: Storage<ArcStore<T>> + Storage<ArcField<T>>,
{
    #[track_caller]
    fn from(value: Store<T, S>) -> Self {
        Field {
            #[cfg(any(debug_assertions, leptos_debuginfo))]
            defined_at: Location::caller(),
            inner: ArenaItem::new_with_storage(value.into()),
        }
    }
}

impl<T, S> From<ArcField<T>> for Field<T, S>
where
    T: 'static,
    S: Storage<ArcField<T>>,
{
    #[track_caller]
    fn from(value: ArcField<T>) -> Self {
        Field {
            #[cfg(any(debug_assertions, leptos_debuginfo))]
            defined_at: Location::caller(),
            inner: ArenaItem::new_with_storage(value),
        }
    }
}

impl<T, S> From<ArcStore<T>> for Field<T, S>
where
    T: Send + Sync + 'static,
    S: Storage<ArcStore<T>> + Storage<ArcField<T>>,
{
    #[track_caller]
    fn from(value: ArcStore<T>) -> Self {
        Field {
            #[cfg(any(debug_assertions, leptos_debuginfo))]
            defined_at: Location::caller(),
            inner: ArenaItem::new_with_storage(value.into()),
        }
    }
}

impl<Inner, Prev, T, S> From<Subfield<Inner, Prev, T>> for Field<T, S>
where
    T: Send + Sync,
    S: Storage<ArcField<T>>,
    Subfield<Inner, Prev, T>: Clone,
    Inner: StoreField<Value = Prev> + Send + Sync + 'static,
    Prev: 'static,
{
    #[track_caller]
    fn from(value: Subfield<Inner, Prev, T>) -> Self {
        Field {
            #[cfg(any(debug_assertions, leptos_debuginfo))]
            defined_at: Location::caller(),
            inner: ArenaItem::new_with_storage(value.into()),
        }
    }
}

impl<Inner, T> From<DerefedField<Inner>> for Field<T>
where
    Inner: Clone + StoreField + Send + Sync + 'static,
    Inner::Value: Deref<Target = T> + DerefMut,
    T: Sized + 'static,
{
    #[track_caller]
    fn from(value: DerefedField<Inner>) -> Self {
        Field {
            #[cfg(any(debug_assertions, leptos_debuginfo))]
            defined_at: Location::caller(),
            inner: ArenaItem::new_with_storage(value.into()),
        }
    }
}

impl<Inner, Prev, S> From<AtIndex<Inner, Prev>> for Field<Prev::Output, S>
where
    AtIndex<Inner, Prev>: Clone,
    S: Storage<ArcField<Prev::Output>>,
    Inner: StoreField<Value = Prev> + Send + Sync + 'static,
    Prev: IndexMut<usize> + Send + Sync + 'static,
    Prev::Output: Sized + Send + Sync,
{
    #[track_caller]
    fn from(value: AtIndex<Inner, Prev>) -> Self {
        Field {
            #[cfg(any(debug_assertions, leptos_debuginfo))]
            defined_at: Location::caller(),
            inner: ArenaItem::new_with_storage(value.into()),
        }
    }
}

impl<Inner, Prev, K, T, S> From<AtKeyed<Inner, Prev, K, T>>
    for Field<T::Output, S>
where
    S: Storage<ArcField<T::Output>>,
    AtKeyed<Inner, Prev, K, T>: Clone,
    K: Debug + Send + Sync + PartialEq + Eq + Hash + 'static,
    KeyedSubfield<Inner, Prev, K, T>: Clone,
    for<'a> &'a T: IntoIterator,
    Inner: StoreField<Value = Prev> + Send + Sync + 'static,
    Prev: 'static,
    T: IndexMut<usize> + 'static,
    T::Output: Sized,
{
    #[track_caller]
    fn from(value: AtKeyed<Inner, Prev, K, T>) -> Self {
        Field {
            #[cfg(any(debug_assertions, leptos_debuginfo))]
            defined_at: Location::caller(),
            inner: ArenaItem::new_with_storage(value.into()),
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
        #[cfg(any(debug_assertions, leptos_debuginfo))]
        {
            Some(self.defined_at)
        }
        #[cfg(not(any(debug_assertions, leptos_debuginfo)))]
        {
            None
        }
    }
}

impl<T, S> Notify for Field<T, S>
where
    S: Storage<ArcField<T>>,
{
    fn notify(&self) {
        if let Some(inner) = self.inner.try_get_value() {
            inner.notify();
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

impl<T> Write for Field<T> {
    type Value = T;

    fn try_write(&self) -> Option<impl UntrackableGuard<Target = Self::Value>> {
        self.inner.try_get_value().and_then(|inner| (inner.write)())
    }

    fn try_write_untracked(
        &self,
    ) -> Option<impl DerefMut<Target = Self::Value>> {
        self.inner.try_get_value().and_then(|inner| {
            let mut guard = (inner.write)()?;
            guard.untrack();
            Some(guard)
        })
    }
}

impl<T, S> IsDisposed for Field<T, S> {
    fn is_disposed(&self) -> bool {
        self.inner.is_disposed()
    }
}
