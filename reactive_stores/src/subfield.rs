use crate::{
    path::{StorePath, StorePathSegment},
    store_field::StoreField,
};
use reactive_graph::{
    signal::{
        guards::{
            Mapped, MappedMut, Plain, ReadGuard, UntrackedWriteGuard,
            WriteGuard,
        },
        ArcTrigger,
    },
    traits::{
        DefinedAt, IsDisposed, ReadUntracked, Track, Trigger, UntrackableGuard,
        Writeable,
    },
};
use std::{
    iter,
    marker::PhantomData,
    ops::DerefMut,
    panic::Location,
    sync::{Arc, RwLock},
};

#[derive(Debug)]
pub struct Subfield<Inner, Prev, T>
where
    Inner: StoreField<Prev>,
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
    Inner: StoreField<Prev> + Clone,
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
    Inner: StoreField<Prev> + Copy
{
}

impl<Inner, Prev, T> Subfield<Inner, Prev, T>
where
    Inner: StoreField<Prev>,
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

impl<Inner, Prev, T> StoreField<T> for Subfield<Inner, Prev, T>
where
    Inner: StoreField<Prev>,
{
    type Orig = Inner::Orig;
    type Reader = Mapped<Inner::Reader, T>;
    type Writer = MappedMut<WriteGuard<ArcTrigger, Inner::Writer>, T>;

    fn path(&self) -> impl IntoIterator<Item = StorePathSegment> {
        self.inner
            .path()
            .into_iter()
            .chain(iter::once(self.path_segment))
    }

    fn data(&self) -> Arc<RwLock<Self::Orig>> {
        self.inner.data()
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
        let inner = WriteGuard::new(trigger, self.inner.writer()?);
        Some(MappedMut::new(inner, self.read, self.write))
    }
}

impl<Inner, Prev, T> DefinedAt for Subfield<Inner, Prev, T>
where
    Inner: StoreField<Prev>,
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
    Inner: StoreField<Prev> + IsDisposed,
{
    fn is_disposed(&self) -> bool {
        self.inner.is_disposed()
    }
}

impl<Inner, Prev, T> Trigger for Subfield<Inner, Prev, T>
where
    Inner: StoreField<Prev>,
{
    fn trigger(&self) {
        let trigger = self.get_trigger(self.path().into_iter().collect());
        trigger.trigger();
    }
}

impl<Inner, Prev, T> Track for Subfield<Inner, Prev, T>
where
    Inner: StoreField<Prev> + Send + Sync + Clone + 'static,
    Prev: 'static,
    T: 'static,
{
    fn track(&self) {
        let trigger = self.get_trigger(self.path().into_iter().collect());
        trigger.track();
    }
}

impl<Inner, Prev, T> ReadUntracked for Subfield<Inner, Prev, T>
where
    Inner: StoreField<Prev>,
{
    type Value = <Self as StoreField<T>>::Reader;

    fn try_read_untracked(&self) -> Option<Self::Value> {
        self.reader()
    }
}

impl<Inner, Prev, T> Writeable for Subfield<Inner, Prev, T>
where
    T: 'static,
    Inner: StoreField<Prev>,
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
