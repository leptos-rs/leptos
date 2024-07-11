use crate::{
    path::{StorePath, StorePathSegment},
    store_field::StoreField,
};
use reactive_graph::{
    signal::{
        guards::{MappedMutArc, WriteGuard},
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
    ops::{DerefMut, Index, IndexMut},
    panic::Location,
    sync::{Arc, RwLock},
};

#[derive(Debug)]
pub struct AtIndex<Inner, Prev>
where
    Inner: StoreField<Prev>,
{
    #[cfg(debug_assertions)]
    defined_at: &'static Location<'static>,
    inner: Inner,
    index: usize,
    ty: PhantomData<Prev>,
}

impl<Inner, Prev> Clone for AtIndex<Inner, Prev>
where
    Inner: StoreField<Prev> + Clone,
{
    fn clone(&self) -> Self {
        Self {
            #[cfg(debug_assertions)]
            defined_at: self.defined_at,
            inner: self.inner.clone(),
            index: self.index,
            ty: self.ty,
        }
    }
}

impl<Inner, Prev> Copy for AtIndex<Inner, Prev> where
    Inner: StoreField<Prev> + Copy
{
}

impl<Inner, Prev> AtIndex<Inner, Prev>
where
    Inner: StoreField<Prev>,
{
    #[track_caller]
    pub fn new(inner: Inner, index: usize) -> Self {
        Self {
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
            inner,
            index,
            ty: PhantomData,
        }
    }
}

impl<Inner, Prev> StoreField<Prev::Output> for AtIndex<Inner, Prev>
where
    Inner: StoreField<Prev>,
    Prev: IndexMut<usize>,
    Prev::Output: Sized,
{
    type Orig = Inner::Orig;
    type Reader = MappedMutArc<Inner::Reader, Prev::Output>;
    type Writer =
        MappedMutArc<WriteGuard<ArcTrigger, Inner::Writer>, Prev::Output>;

    fn path(&self) -> impl IntoIterator<Item = StorePathSegment> {
        self.inner
            .path()
            .into_iter()
            .chain(iter::once(self.index.into()))
    }

    fn data(&self) -> Arc<RwLock<Self::Orig>> {
        self.inner.data()
    }

    fn get_trigger(&self, path: StorePath) -> ArcTrigger {
        self.inner.get_trigger(path)
    }

    fn reader(&self) -> Option<Self::Reader> {
        let inner = self.inner.reader()?;
        let index = self.index;
        Some(MappedMutArc::new(
            inner,
            move |n| &n[index],
            move |n| &mut n[index],
        ))
    }

    fn writer(&self) -> Option<Self::Writer> {
        let trigger = self.get_trigger(self.path().into_iter().collect());
        let inner = WriteGuard::new(trigger, self.inner.writer()?);
        let index = self.index;
        Some(MappedMutArc::new(
            inner,
            move |n| &n[index],
            move |n| &mut n[index],
        ))
    }
}

impl<Inner, Prev> DefinedAt for AtIndex<Inner, Prev>
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

impl<Inner, Prev> IsDisposed for AtIndex<Inner, Prev>
where
    Inner: StoreField<Prev> + IsDisposed,
{
    fn is_disposed(&self) -> bool {
        self.inner.is_disposed()
    }
}

impl<Inner, Prev> Trigger for AtIndex<Inner, Prev>
where
    Inner: StoreField<Prev>,
    Prev: IndexMut<usize> + 'static,
    Prev::Output: Sized,
{
    fn trigger(&self) {
        let trigger = self.get_trigger(self.path().into_iter().collect());
        trigger.trigger();
    }
}

impl<Inner, Prev> Track for AtIndex<Inner, Prev>
where
    Inner: StoreField<Prev> + Send + Sync + Clone + 'static,
    Prev: IndexMut<usize> + 'static,
    Prev::Output: Sized + 'static,
{
    fn track(&self) {
        let trigger = self.get_trigger(self.path().into_iter().collect());
        trigger.track();
    }
}

impl<Inner, Prev> ReadUntracked for AtIndex<Inner, Prev>
where
    Inner: StoreField<Prev>,
    Prev: IndexMut<usize>,
    Prev::Output: Sized,
{
    type Value = <Self as StoreField<Prev::Output>>::Reader;

    fn try_read_untracked(&self) -> Option<Self::Value> {
        self.reader()
    }
}

impl<Inner, Prev> Writeable for AtIndex<Inner, Prev>
where
    Inner: StoreField<Prev>,
    Prev: IndexMut<usize> + 'static,
    Prev::Output: Sized + 'static,
{
    type Value = Prev::Output;

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

pub trait StoreFieldIterator<Prev>: Sized {
    fn iter(self) -> StoreFieldIter<Self, Prev>;
}

impl<Inner, Prev> StoreFieldIterator<Prev> for Inner
where
    Inner: StoreField<Prev>,
    Prev::Output: Sized,
    Prev: IndexMut<usize> + AsRef<[Prev::Output]>,
{
    fn iter(self) -> StoreFieldIter<Inner, Prev> {
        // reactively track changes to this field
        let trigger = self.get_trigger(self.path().into_iter().collect());
        trigger.track();

        // get the current length of the field by accessing slice
        let len = self.reader().map(|n| n.as_ref().len()).unwrap_or(0);

        // return the iterator
        StoreFieldIter {
            inner: self,
            idx: 0,
            len,
            prev: PhantomData,
        }
    }
}

pub struct StoreFieldIter<Inner, Prev> {
    inner: Inner,
    idx: usize,
    len: usize,
    prev: PhantomData<Prev>,
}

impl<Inner, Prev> Iterator for StoreFieldIter<Inner, Prev>
where
    Inner: StoreField<Prev> + Clone + 'static,
    Prev: IndexMut<usize> + 'static,
    Prev::Output: Sized + 'static,
{
    type Item = AtIndex<Inner, Prev>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx < self.len {
            let field = AtIndex {
                #[cfg(debug_assertions)]
                defined_at: Location::caller(),
                index: self.idx,
                inner: self.inner.clone(),
                ty: PhantomData,
            };
            self.idx += 1;
            Some(field)
        } else {
            None
        }
    }
}
