use crate::{
    path::{StorePath, StorePathSegment},
    ArcStore, KeyMap, Store, StoreFieldTrigger,
};
use or_poisoned::OrPoisoned;
use reactive_graph::{
    owner::Storage,
    signal::{
        guards::{Mapped, MappedMut, Plain, UntrackedWriteGuard, WriteGuard},
        ArcTrigger,
    },
    traits::{
        DefinedAt, IsDisposed, Notify, ReadUntracked, Track, UntrackableGuard,
        Write,
    },
    unwrap_signal,
};
use std::{
    iter,
    ops::{Deref, DerefMut},
    panic::Location,
    sync::Arc,
};

pub trait StoreField: Sized {
    type Value;
    type Reader: Deref<Target = Self::Value>;
    type Writer: UntrackableGuard<Target = Self::Value>;

    fn get_trigger(&self, path: StorePath) -> StoreFieldTrigger;

    fn path(&self) -> impl IntoIterator<Item = StorePathSegment>;

    fn track_field(&self) {
        let path = self.path().into_iter().collect();
        let trigger = self.get_trigger(path);
        trigger.this.track();
        trigger.children.track();
    }

    fn reader(&self) -> Option<Self::Reader>;

    fn writer(&self) -> Option<Self::Writer>;

    fn keys(&self) -> Option<KeyMap>;

    #[track_caller]
    fn then<T>(
        self,
        map_fn: fn(&Self::Value) -> &T,
        map_fn_mut: fn(&mut Self::Value) -> &mut T,
    ) -> Then<T, Self> {
        Then {
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
            inner: self,
            map_fn,
            map_fn_mut,
        }
    }
}

impl<T> StoreField for ArcStore<T>
where
    T: 'static,
{
    type Value = T;
    type Reader = Plain<T>;
    type Writer = WriteGuard<ArcTrigger, UntrackedWriteGuard<T>>;

    fn get_trigger(&self, path: StorePath) -> StoreFieldTrigger {
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
        let guard = UntrackedWriteGuard::try_new(Arc::clone(&self.value))?;
        Some(WriteGuard::new(trigger.children, guard))
    }

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

    fn get_trigger(&self, path: StorePath) -> StoreFieldTrigger {
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

    fn keys(&self) -> Option<KeyMap> {
        self.inner.try_get_value().and_then(|inner| inner.keys())
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Then<T, S>
where
    S: StoreField,
{
    inner: S,
    map_fn: fn(&S::Value) -> &T,
    map_fn_mut: fn(&mut S::Value) -> &mut T,
    #[cfg(debug_assertions)]
    defined_at: &'static Location<'static>,
}

impl<T, S> Then<T, S>
where
    S: StoreField,
{
    #[track_caller]
    pub fn new(
        inner: S,
        map_fn: fn(&S::Value) -> &T,
        map_fn_mut: fn(&mut S::Value) -> &mut T,
    ) -> Self {
        Self {
            inner,
            map_fn,
            map_fn_mut,
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
        }
    }
}

impl<T, S> StoreField for Then<T, S>
where
    S: StoreField,
{
    type Value = T;
    type Reader = Mapped<S::Reader, T>;
    type Writer = MappedMut<S::Writer, T>;

    fn get_trigger(&self, path: StorePath) -> StoreFieldTrigger {
        self.inner.get_trigger(path)
    }

    fn path(&self) -> impl IntoIterator<Item = StorePathSegment> {
        self.inner.path()
    }

    fn reader(&self) -> Option<Self::Reader> {
        let inner = self.inner.reader()?;
        Some(Mapped::new_with_guard(inner, self.map_fn))
    }

    fn writer(&self) -> Option<Self::Writer> {
        let inner = self.inner.writer()?;
        Some(MappedMut::new(inner, self.map_fn, self.map_fn_mut))
    }

    #[inline(always)]
    fn keys(&self) -> Option<KeyMap> {
        self.inner.keys()
    }
}

impl<T, S> DefinedAt for Then<T, S>
where
    S: StoreField,
{
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

impl<T, S> IsDisposed for Then<T, S>
where
    S: StoreField + IsDisposed,
{
    fn is_disposed(&self) -> bool {
        self.inner.is_disposed()
    }
}

impl<T, S> Notify for Then<T, S>
where
    S: StoreField,
{
    fn notify(&self) {
        let trigger = self.get_trigger(self.path().into_iter().collect());
        trigger.this.notify();
        trigger.children.notify();
    }
}

impl<T, S> Track for Then<T, S>
where
    S: StoreField,
{
    fn track(&self) {
        let trigger = self.get_trigger(self.path().into_iter().collect());
        trigger.this.track();
        trigger.children.track();
    }
}

impl<T, S> ReadUntracked for Then<T, S>
where
    S: StoreField,
{
    type Value = <Self as StoreField>::Reader;

    fn try_read_untracked(&self) -> Option<Self::Value> {
        self.reader()
    }
}

impl<T, S> Write for Then<T, S>
where
    T: 'static,
    S: StoreField,
{
    type Value = T;

    fn try_write(&self) -> Option<impl UntrackableGuard<Target = Self::Value>> {
        self.writer()
    }

    fn try_write_untracked(
        &self,
    ) -> Option<impl DerefMut<Target = Self::Value>> {
        self.writer().map(|mut writer| {
            writer.untrack();
            writer
        })
    }
}
