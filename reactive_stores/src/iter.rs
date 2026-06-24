use crate::{
    KeyMap, StoreFieldTrigger,
    len::Len,
    path::{StorePath, StorePathSegment},
    store_field::StoreField,
};
use reactive_graph::{
    signal::{
        ArcTrigger,
        guards::{MappedMutArc, WriteGuard},
    },
    traits::{
        DefinedAt, IsDisposed, Notify, ReadUntracked, Track, UntrackableGuard,
        Write,
    },
};
use std::{
    iter,
    marker::PhantomData,
    ops::{DerefMut, IndexMut},
    panic::Location,
};

/// Provides access to the data at some index in another collection.
#[derive(Debug)]
pub struct AtIndex<Inner, Prev> {
    #[cfg(any(debug_assertions, leptos_debuginfo))]
    defined_at: &'static Location<'static>,
    inner: Inner,
    index: usize,
    ty: PhantomData<Prev>,
}

impl<Inner, Prev> Clone for AtIndex<Inner, Prev>
where
    Inner: Clone,
{
    fn clone(&self) -> Self {
        Self {
            #[cfg(any(debug_assertions, leptos_debuginfo))]
            defined_at: self.defined_at,
            inner: self.inner.clone(),
            index: self.index,
            ty: self.ty,
        }
    }
}

impl<Inner, Prev> Copy for AtIndex<Inner, Prev> where Inner: Copy {}

impl<Inner, Prev> AtIndex<Inner, Prev> {
    /// Creates a new accessor for the inner collection at the given index.
    #[track_caller]
    pub fn new(inner: Inner, index: usize) -> Self {
        Self {
            #[cfg(any(debug_assertions, leptos_debuginfo))]
            defined_at: Location::caller(),
            inner,
            index,
            ty: PhantomData,
        }
    }
}

impl<Inner, Prev> StoreField for AtIndex<Inner, Prev>
where
    Inner: StoreField<Value = Prev>,
    Prev: IndexMut<usize> + Len + 'static,
    Prev::Output: Sized,
{
    type Value = Prev::Output;
    type Reader = MappedMutArc<Inner::Reader, Prev::Output>;
    type Writer =
        MappedMutArc<WriteGuard<ArcTrigger, Inner::Writer>, Prev::Output>;

    fn path(&self) -> impl IntoIterator<Item = StorePathSegment> {
        self.inner
            .path()
            .into_iter()
            .chain(iter::once(self.index.into()))
    }

    fn path_unkeyed(&self) -> impl IntoIterator<Item = StorePathSegment> {
        self.inner
            .path_unkeyed()
            .into_iter()
            .chain(iter::once(self.index.into()))
    }

    fn get_trigger(&self, path: StorePath) -> StoreFieldTrigger {
        self.inner.get_trigger(path)
    }

    fn get_trigger_unkeyed(&self, path: StorePath) -> StoreFieldTrigger {
        self.inner.get_trigger_unkeyed(path)
    }

    fn reader(&self) -> Option<Self::Reader> {
        let inner = self.inner.reader()?;
        let index = self.index;
        // The reader holds the inner lock for its whole lifetime, so the
        // length cannot change before the (lazy) projection runs on deref.
        // Bail out with `None` if the index is out of bounds, instead of
        // handing back a guard that panics when first dereferenced.
        if index >= inner.len() {
            return None;
        }
        Some(MappedMutArc::new(
            inner,
            move |n| &n[index],
            move |n| &mut n[index],
        ))
    }

    fn writer(&self) -> Option<Self::Writer> {
        let trigger = self.get_trigger(self.path().into_iter().collect());
        let inner = WriteGuard::new(trigger.children, self.inner.writer()?);
        let index = self.index;
        // See `reader`: the write guard holds the inner lock, so a single
        // bounds check here is sufficient to keep the projection panic-free.
        if index >= inner.len() {
            return None;
        }
        Some(MappedMutArc::new(
            inner,
            move |n| &n[index],
            move |n| &mut n[index],
        ))
    }

    #[inline(always)]
    fn keys(&self) -> Option<KeyMap> {
        self.inner.keys()
    }

    fn track_field(&self) {
        let mut full_path = self.path().into_iter().collect::<StorePath>();
        let trigger = self.get_trigger(self.path().into_iter().collect());
        trigger.this.track();
        trigger.children.track();

        // tracks `this` for all ancestors: i.e., it will track any change that is made
        // directly to one of its ancestors, but not a change made to a *child* of an ancestor
        // (which would end up with every subfield tracking its own siblings, because they are
        // children of its parent)
        while !full_path.is_empty() {
            full_path.pop();
            let inner = self.get_trigger(full_path.clone());
            inner.this.track();
        }
    }
}

impl<Inner, Prev> DefinedAt for AtIndex<Inner, Prev>
where
    Inner: StoreField<Value = Prev>,
{
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

impl<Inner, Prev> IsDisposed for AtIndex<Inner, Prev>
where
    Inner: StoreField<Value = Prev> + IsDisposed,
{
    fn is_disposed(&self) -> bool {
        self.inner.is_disposed()
    }
}

impl<Inner, Prev> Notify for AtIndex<Inner, Prev>
where
    Inner: StoreField<Value = Prev>,
    Prev: IndexMut<usize> + Len + 'static,
    Prev::Output: Sized,
{
    fn notify(&self) {
        let trigger = self.get_trigger(self.path().into_iter().collect());
        trigger.this.notify();
    }
}

impl<Inner, Prev> Track for AtIndex<Inner, Prev>
where
    Inner: StoreField<Value = Prev> + Send + Sync + Clone + 'static,
    Prev: IndexMut<usize> + Len + 'static,
    Prev::Output: Sized + 'static,
{
    fn track(&self) {
        self.track_field();
    }
}

impl<Inner, Prev> ReadUntracked for AtIndex<Inner, Prev>
where
    Inner: StoreField<Value = Prev>,
    Prev: IndexMut<usize> + Len + 'static,
    Prev::Output: Sized,
{
    type Value = <Self as StoreField>::Reader;

    fn try_read_untracked(&self) -> Option<Self::Value> {
        self.reader()
    }
}

impl<Inner, Prev> Write for AtIndex<Inner, Prev>
where
    Inner: StoreField<Value = Prev>,
    Prev: IndexMut<usize> + Len + 'static,
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

/// Provides unkeyed reactive access to the fields of some collection.
pub trait StoreFieldIterator<Prev>
where
    Self: StoreField<Value = Prev>,
{
    /// Reactive access to the value at some index.
    fn at_unkeyed(self, index: usize) -> AtIndex<Self, Prev>;

    /// An iterator over the values in the collection.
    fn iter_unkeyed(self) -> StoreFieldIter<Self, Prev>;
}

impl<Inner, Prev> StoreFieldIterator<Prev> for Inner
where
    Inner: StoreField<Value = Prev> + Clone,
    Prev::Output: Sized,
    Prev: IndexMut<usize> + Len,
{
    #[track_caller]
    fn at_unkeyed(self, index: usize) -> AtIndex<Inner, Prev> {
        AtIndex::new(self.clone(), index)
    }

    #[track_caller]
    fn iter_unkeyed(self) -> StoreFieldIter<Inner, Prev> {
        // reactively track changes to this field
        let trigger = self.get_trigger(self.path().into_iter().collect());
        trigger.this.track();
        trigger.children.track();

        // get the current length of the field by accessing slice
        let len = self.reader().map(|n| n.len()).unwrap_or(0);

        // return the iterator
        StoreFieldIter {
            inner: self,
            idx: 0,
            len,
            prev: PhantomData,
        }
    }
}

/// An iterator over the values in a collection, as reactive fields.
pub struct StoreFieldIter<Inner, Prev> {
    inner: Inner,
    idx: usize,
    len: usize,
    prev: PhantomData<Prev>,
}

impl<Inner, Prev> Iterator for StoreFieldIter<Inner, Prev>
where
    Inner: StoreField<Value = Prev> + Clone + 'static,
    Prev: IndexMut<usize> + 'static,
    Prev::Output: Sized + 'static,
{
    type Item = AtIndex<Inner, Prev>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx < self.len {
            let field = AtIndex::new(self.inner.clone(), self.idx);
            self.idx += 1;
            Some(field)
        } else {
            None
        }
    }
}

impl<Inner, Prev> DoubleEndedIterator for StoreFieldIter<Inner, Prev>
where
    Inner: StoreField<Value = Prev> + Clone + 'static,
    Prev: IndexMut<usize> + 'static,
    Prev::Output: Sized + 'static,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.len > self.idx {
            self.len -= 1;
            let field = AtIndex::new(self.inner.clone(), self.len);
            Some(field)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{self as reactive_stores, AtIndex, Store};
    use reactive_graph::{
        owner::Owner,
        traits::{ReadUntracked, Write},
    };

    #[derive(Default, reactive_stores_macro::Store)]
    struct State {
        items: Vec<i32>,
    }

    #[test]
    fn out_of_bounds_index_reads_as_none_without_panicking() {
        let owner = Owner::new();
        owner.set();

        let store = Store::new(State {
            items: vec![1, 2, 3],
        });

        // an index that has never been valid
        let at = AtIndex::new(store.items(), 5);
        let guard = at.try_read_untracked();
        assert!(guard.is_none());
        // forcing the (lazy) projection must not panic
        assert!(guard.map(|g| *g).is_none());

        // an index that was valid at construction but is shrunk away before read
        let at = AtIndex::new(store.items(), 2);
        store.items().write().clear();
        assert!(at.try_read_untracked().is_none());
    }

    #[test]
    fn in_bounds_index_still_reads() {
        let owner = Owner::new();
        owner.set();

        let store = Store::new(State {
            items: vec![10, 20, 30],
        });
        let at = AtIndex::new(store.items(), 1);
        let guard = at.try_read_untracked().expect("index 1 is in bounds");
        assert_eq!(*guard, 20);
    }
}
