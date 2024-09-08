use crate::{
    path::{StorePath, StorePathSegment},
    store_field::StoreField,
};
use reactive_graph::{
    owner::{LocalStorage, SyncStorage},
    signal::{
        guards::{Mapped, MappedMut, WriteGuard},
        ArcTrigger,
    },
    traits::{
        DefinedAt, Get, IsDisposed, ReadUntracked, Track, Trigger,
        UntrackableGuard, Writeable,
    },
    wrappers::read::{MaybeSignal, Signal},
};
use std::{iter, marker::PhantomData, ops::DerefMut, panic::Location};

#[derive(Debug)]
pub struct Subfield<Inner, Prev, T>
where
    Inner: StoreField<Value = Prev>,
{
    #[cfg(debug_assertions)]
    defined_at: &'static Location<'static>,
    path_segment: StorePathSegment,
    inner: Inner,
    read: fn(&Prev) -> &T,
    write: fn(&mut Prev) -> &mut T,
    ty: PhantomData<T>,
}

impl<Inner, Prev, T> Clone for Subfield<Inner, Prev, T>
where
    Inner: StoreField<Value = Prev> + Clone,
{
    fn clone(&self) -> Self {
        Self {
            #[cfg(debug_assertions)]
            defined_at: self.defined_at,
            path_segment: self.path_segment,
            inner: self.inner.clone(),
            read: self.read,
            write: self.write,
            ty: self.ty,
        }
    }
}

impl<Inner, Prev, T> Copy for Subfield<Inner, Prev, T> where
    Inner: StoreField<Value = Prev> + Copy
{
}

impl<Inner, Prev, T> Subfield<Inner, Prev, T>
where
    Inner: StoreField<Value = Prev>,
{
    #[track_caller]
    pub fn new(
        inner: Inner,
        path_segment: StorePathSegment,
        read: fn(&Prev) -> &T,
        write: fn(&mut Prev) -> &mut T,
    ) -> Self {
        Self {
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
            inner,
            path_segment,
            read,
            write,
            ty: PhantomData,
        }
    }
}

impl<Inner, Prev, T> StoreField for Subfield<Inner, Prev, T>
where
    Inner: StoreField<Value = Prev>,
    Prev: 'static,
{
    type Value = T;
    type Reader = Mapped<Inner::Reader, T>;
    type Writer = MappedMut<WriteGuard<ArcTrigger, Inner::UntrackedWriter>, T>;
    type UntrackedWriter =
        MappedMut<WriteGuard<ArcTrigger, Inner::UntrackedWriter>, T>;

    fn path(&self) -> impl IntoIterator<Item = StorePathSegment> {
        self.inner
            .path()
            .into_iter()
            .chain(iter::once(self.path_segment))
    }

    fn get_trigger(&self, path: StorePath) -> ArcTrigger {
        self.inner.get_trigger(path)
    }

    fn reader(&self) -> Option<Self::Reader> {
        let inner = self.inner.reader()?;
        Some(Mapped::new_with_guard(inner, self.read))
    }

    fn writer(&self) -> Option<Self::Writer> {
        let trigger = self.get_trigger(self.path().into_iter().collect());
        let inner = WriteGuard::new(trigger, self.inner.untracked_writer()?);
        Some(MappedMut::new(inner, self.read, self.write))
    }

    fn untracked_writer(&self) -> Option<Self::UntrackedWriter> {
        let mut guard = self.writer()?;
        guard.untrack();
        Some(guard)
    }
}

impl<Inner, Prev, T> DefinedAt for Subfield<Inner, Prev, T>
where
    Inner: StoreField<Value = Prev>,
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

impl<Inner, Prev, T> IsDisposed for Subfield<Inner, Prev, T>
where
    Inner: StoreField<Value = Prev> + IsDisposed,
{
    fn is_disposed(&self) -> bool {
        self.inner.is_disposed()
    }
}

impl<Inner, Prev, T> Trigger for Subfield<Inner, Prev, T>
where
    Inner: StoreField<Value = Prev>,
    Prev: 'static,
{
    fn trigger(&self) {
        let trigger = self.get_trigger(self.path().into_iter().collect());
        trigger.trigger();
    }
}

impl<Inner, Prev, T> Track for Subfield<Inner, Prev, T>
where
    Inner: StoreField<Value = Prev> + Track + 'static,
    Prev: 'static,
    T: 'static,
{
    fn track(&self) {
        self.inner.track();
        let trigger = self.get_trigger(self.path().into_iter().collect());
        trigger.track();
    }
}

impl<Inner, Prev, T> ReadUntracked for Subfield<Inner, Prev, T>
where
    Inner: StoreField<Value = Prev>,
    Prev: 'static,
{
    type Value = <Self as StoreField>::Reader;

    fn try_read_untracked(&self) -> Option<Self::Value> {
        self.reader()
    }
}

impl<Inner, Prev, T> Writeable for Subfield<Inner, Prev, T>
where
    T: 'static,
    Inner: StoreField<Value = Prev>,
    Prev: 'static,
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

impl<Inner, Prev, T> From<Subfield<Inner, Prev, T>> for Signal<T, SyncStorage>
where
    T: Clone + Send + Sync + 'static,
    Inner: StoreField<Value = Prev> + Send + Sync + 'static,
    Prev: 'static,
    Subfield<Inner, Prev, T>: Track,
{
    fn from(value: Subfield<Inner, Prev, T>) -> Self {
        Self::derive(move || value.get())
    }
}

impl<Inner, Prev, T> From<Subfield<Inner, Prev, T>> for Signal<T, LocalStorage>
where
    T: Clone + 'static,
    Inner: StoreField<Value = Prev> + 'static,
    Prev: 'static,
    Subfield<Inner, Prev, T>: Track,
{
    fn from(value: Subfield<Inner, Prev, T>) -> Self {
        Self::derive_local(move || value.get())
    }
}

impl<Inner, Prev, T> From<Subfield<Inner, Prev, T>>
    for MaybeSignal<T, SyncStorage>
where
    T: Clone + Send + Sync + 'static,
    Inner: StoreField<Value = Prev> + Send + Sync + 'static,
    Prev: 'static,
    Subfield<Inner, Prev, T>: Track,
{
    fn from(value: Subfield<Inner, Prev, T>) -> Self {
        Self::Dynamic(value.into())
    }
}

impl<Inner, Prev, T> From<Subfield<Inner, Prev, T>>
    for MaybeSignal<T, LocalStorage>
where
    T: Clone + 'static,
    Inner: StoreField<Value = Prev> + 'static,
    Prev: 'static,
    Subfield<Inner, Prev, T>: Track,
{
    fn from(value: Subfield<Inner, Prev, T>) -> Self {
        Self::Dynamic(value.into())
    }
}
