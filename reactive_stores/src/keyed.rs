use crate::{
    path::{StorePath, StorePathSegment},
    store_field::StoreField,
    KeyMap, StoreFieldTrigger,
};
use reactive_graph::{
    signal::{
        guards::{Mapped, MappedMut, MappedMutArc, WriteGuard},
        ArcTrigger,
    },
    traits::{
        DefinedAt, IsDisposed, Notify, ReadUntracked, Track, UntrackableGuard,
        Write,
    },
};
use std::{
    collections::VecDeque,
    fmt::Debug,
    hash::Hash,
    iter,
    ops::{Deref, DerefMut, IndexMut},
    panic::Location,
};

/// Provides access to a subfield that contains some kind of keyed collection.
#[derive(Debug)]
pub struct KeyedSubfield<Inner, Prev, K, T>
where
    for<'a> &'a T: IntoIterator,
{
    #[cfg(any(debug_assertions, leptos_debuginfo))]
    defined_at: &'static Location<'static>,
    path_segment: StorePathSegment,
    inner: Inner,
    read: fn(&Prev) -> &T,
    write: fn(&mut Prev) -> &mut T,
    key_fn: fn(<&T as IntoIterator>::Item) -> K,
}

impl<Inner, Prev, K, T> Clone for KeyedSubfield<Inner, Prev, K, T>
where
    for<'a> &'a T: IntoIterator,
    Inner: Clone,
{
    fn clone(&self) -> Self {
        Self {
            #[cfg(any(debug_assertions, leptos_debuginfo))]
            defined_at: self.defined_at,
            path_segment: self.path_segment,
            inner: self.inner.clone(),
            read: self.read,
            write: self.write,
            key_fn: self.key_fn,
        }
    }
}

impl<Inner, Prev, K, T> Copy for KeyedSubfield<Inner, Prev, K, T>
where
    for<'a> &'a T: IntoIterator,
    Inner: Copy,
{
}

impl<Inner, Prev, K, T> KeyedSubfield<Inner, Prev, K, T>
where
    for<'a> &'a T: IntoIterator,
{
    /// Creates a keyed subfield of the inner data type with the given key function.
    #[track_caller]
    pub fn new(
        inner: Inner,
        path_segment: StorePathSegment,
        key_fn: fn(<&T as IntoIterator>::Item) -> K,
        read: fn(&Prev) -> &T,
        write: fn(&mut Prev) -> &mut T,
    ) -> Self {
        Self {
            #[cfg(any(debug_assertions, leptos_debuginfo))]
            defined_at: Location::caller(),
            inner,
            path_segment,
            read,
            write,
            key_fn,
        }
    }
}

impl<Inner, Prev, K, T> StoreField for KeyedSubfield<Inner, Prev, K, T>
where
    Self: Clone,
    for<'a> &'a T: IntoIterator,
    Inner: StoreField<Value = Prev>,
    Prev: 'static,
    K: Debug + Send + Sync + PartialEq + Eq + Hash + 'static,
{
    type Value = T;
    type Reader = Mapped<Inner::Reader, T>;
    type Writer = MappedMut<WriteGuard<ArcTrigger, Inner::Writer>, T>;

    fn path(&self) -> impl IntoIterator<Item = StorePathSegment> {
        self.inner
            .path()
            .into_iter()
            .chain(iter::once(self.path_segment))
    }

    fn get_trigger(&self, path: StorePath) -> StoreFieldTrigger {
        self.inner.get_trigger(path)
    }

    fn reader(&self) -> Option<Self::Reader> {
        let inner = self.inner.reader()?;
        Some(Mapped::new_with_guard(inner, self.read))
    }

    fn writer(&self) -> Option<Self::Writer> {
        let path = self.path().into_iter().collect::<StorePath>();
        let trigger = self.get_trigger(path.clone());
        let guard = WriteGuard::new(trigger.children, self.inner.writer()?);
        Some(MappedMut::new(guard, self.read, self.write))
    }

    #[inline(always)]
    fn keys(&self) -> Option<KeyMap> {
        self.inner.keys()
    }

    fn track_field(&self) {
        let inner = self
            .inner
            .get_trigger(self.inner.path().into_iter().collect());
        inner.this.track();
        let trigger = self.get_trigger(self.path().into_iter().collect());
        trigger.this.track();
        trigger.children.track();
    }
}

impl<Inner, Prev, K, T> KeyedSubfield<Inner, Prev, K, T>
where
    Self: Clone,
    for<'a> &'a T: IntoIterator,
    Inner: StoreField<Value = Prev>,
    Prev: 'static,
    K: Debug + Send + Sync + PartialEq + Eq + Hash + 'static,
{
    fn latest_keys(&self) -> Vec<K> {
        self.reader()
            .map(|r| r.deref().into_iter().map(|n| (self.key_fn)(n)).collect())
            .unwrap_or_default()
    }
}

/// Gives keyed write access to a value in some collection.
pub struct KeyedSubfieldWriteGuard<Inner, Prev, K, T, Guard>
where
    KeyedSubfield<Inner, Prev, K, T>: Clone,
    for<'a> &'a T: IntoIterator,
    Inner: StoreField<Value = Prev>,
    Prev: 'static,
    K: Debug + Send + Sync + PartialEq + Eq + Hash + 'static,
{
    inner: KeyedSubfield<Inner, Prev, K, T>,
    guard: Option<Guard>,
}

impl<Inner, Prev, K, T, Guard> Deref
    for KeyedSubfieldWriteGuard<Inner, Prev, K, T, Guard>
where
    Guard: Deref,
    KeyedSubfield<Inner, Prev, K, T>: Clone,
    for<'a> &'a T: IntoIterator,
    Inner: StoreField<Value = Prev>,
    Prev: 'static,
    K: Debug + Send + Sync + PartialEq + Eq + Hash + 'static,
{
    type Target = Guard::Target;

    fn deref(&self) -> &Self::Target {
        self.guard
            .as_ref()
            .expect("should be Some(_) until dropped")
            .deref()
    }
}

impl<Inner, Prev, K, T, Guard> DerefMut
    for KeyedSubfieldWriteGuard<Inner, Prev, K, T, Guard>
where
    Guard: DerefMut,
    KeyedSubfield<Inner, Prev, K, T>: Clone,
    for<'a> &'a T: IntoIterator,
    Inner: StoreField<Value = Prev>,
    Prev: 'static,
    K: Debug + Send + Sync + PartialEq + Eq + Hash + 'static,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.guard
            .as_mut()
            .expect("should be Some(_) until dropped")
            .deref_mut()
    }
}

impl<Inner, Prev, K, T, Guard> UntrackableGuard
    for KeyedSubfieldWriteGuard<Inner, Prev, K, T, Guard>
where
    Guard: UntrackableGuard,
    KeyedSubfield<Inner, Prev, K, T>: Clone,
    for<'a> &'a T: IntoIterator,
    Inner: StoreField<Value = Prev>,
    Prev: 'static,
    K: Debug + Send + Sync + PartialEq + Eq + Hash + 'static,
{
    fn untrack(&mut self) {
        if let Some(inner) = self.guard.as_mut() {
            inner.untrack();
        }
    }
}

impl<Inner, Prev, K, T, Guard> Drop
    for KeyedSubfieldWriteGuard<Inner, Prev, K, T, Guard>
where
    KeyedSubfield<Inner, Prev, K, T>: Clone,
    for<'a> &'a T: IntoIterator,
    Inner: StoreField<Value = Prev>,
    Prev: 'static,
    K: Debug + Send + Sync + PartialEq + Eq + Hash + 'static,
{
    fn drop(&mut self) {
        // dropping the inner guard will
        // 1) synchronously release its write lock on the store's value
        // 2) trigger an (asynchronous) reactive update
        drop(self.guard.take());

        // now that the write lock is release, we can get a read lock to refresh this keyed field
        // based on the new value
        self.inner.update_keys();
        self.inner.notify();

        // reactive updates happen on the next tick
    }
}

impl<Inner, Prev, K, T> DefinedAt for KeyedSubfield<Inner, Prev, K, T>
where
    for<'a> &'a T: IntoIterator,
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

impl<Inner, Prev, K, T> IsDisposed for KeyedSubfield<Inner, Prev, K, T>
where
    for<'a> &'a T: IntoIterator,
    Inner: IsDisposed,
{
    fn is_disposed(&self) -> bool {
        self.inner.is_disposed()
    }
}

impl<Inner, Prev, K, T> Notify for KeyedSubfield<Inner, Prev, K, T>
where
    Self: Clone,
    for<'a> &'a T: IntoIterator,
    Inner: StoreField<Value = Prev>,
    Prev: 'static,
    K: Debug + Send + Sync + PartialEq + Eq + Hash + 'static,
{
    fn notify(&self) {
        let trigger = self.get_trigger(self.path().into_iter().collect());
        trigger.this.notify();
        trigger.children.notify();
    }
}

impl<Inner, Prev, K, T> Track for KeyedSubfield<Inner, Prev, K, T>
where
    Self: Clone,
    for<'a> &'a T: IntoIterator,
    Inner: StoreField<Value = Prev> + Track + 'static,
    Prev: 'static,
    T: 'static,
    K: Debug + Send + Sync + PartialEq + Eq + Hash + 'static,
{
    fn track(&self) {
        self.track_field();
    }
}

impl<Inner, Prev, K, T> ReadUntracked for KeyedSubfield<Inner, Prev, K, T>
where
    Self: Clone,
    for<'a> &'a T: IntoIterator,
    Inner: StoreField<Value = Prev>,
    Prev: 'static,
    K: Debug + Send + Sync + PartialEq + Eq + Hash + 'static,
{
    type Value = <Self as StoreField>::Reader;

    fn try_read_untracked(&self) -> Option<Self::Value> {
        self.reader()
    }
}

impl<Inner, Prev, K, T> Write for KeyedSubfield<Inner, Prev, K, T>
where
    Self: Clone,
    for<'a> &'a T: IntoIterator,
    T: 'static,
    Inner: StoreField<Value = Prev>,
    Prev: 'static,
    K: Debug + Send + Sync + PartialEq + Eq + Hash + 'static,
{
    type Value = T;

    fn try_write(&self) -> Option<impl UntrackableGuard<Target = Self::Value>> {
        let guard = self.writer()?;
        Some(KeyedSubfieldWriteGuard {
            inner: self.clone(),
            guard: Some(guard),
        })
    }

    fn try_write_untracked(
        &self,
    ) -> Option<impl DerefMut<Target = Self::Value>> {
        let mut guard = self.writer()?;
        guard.untrack();
        Some(KeyedSubfieldWriteGuard {
            inner: self.clone(),
            guard: Some(guard),
        })
    }
}

/// Gives access to the value in a collection based on some key.
#[derive(Debug)]
pub struct AtKeyed<Inner, Prev, K, T>
where
    for<'a> &'a T: IntoIterator,
{
    #[cfg(any(debug_assertions, leptos_debuginfo))]
    defined_at: &'static Location<'static>,
    inner: KeyedSubfield<Inner, Prev, K, T>,
    key: K,
}

impl<Inner, Prev, K, T> Clone for AtKeyed<Inner, Prev, K, T>
where
    for<'a> &'a T: IntoIterator,
    KeyedSubfield<Inner, Prev, K, T>: Clone,
    K: Debug + Clone,
{
    fn clone(&self) -> Self {
        Self {
            #[cfg(any(debug_assertions, leptos_debuginfo))]
            defined_at: self.defined_at,
            inner: self.inner.clone(),
            key: self.key.clone(),
        }
    }
}

impl<Inner, Prev, K, T> Copy for AtKeyed<Inner, Prev, K, T>
where
    for<'a> &'a T: IntoIterator,
    KeyedSubfield<Inner, Prev, K, T>: Copy,
    K: Debug + Copy,
{
}

impl<Inner, Prev, K, T> AtKeyed<Inner, Prev, K, T>
where
    for<'a> &'a T: IntoIterator,
{
    /// Provides access to the item in the inner collection at this key.
    #[track_caller]
    pub fn new(inner: KeyedSubfield<Inner, Prev, K, T>, key: K) -> Self {
        Self {
            #[cfg(any(debug_assertions, leptos_debuginfo))]
            defined_at: Location::caller(),
            inner,
            key,
        }
    }
}

impl<Inner, Prev, K, T> StoreField for AtKeyed<Inner, Prev, K, T>
where
    K: Debug + Send + Sync + PartialEq + Eq + Hash + 'static,
    KeyedSubfield<Inner, Prev, K, T>: Clone,
    for<'a> &'a T: IntoIterator,
    Inner: StoreField<Value = Prev>,
    Prev: 'static,
    T: IndexMut<usize>,
    T::Output: Sized,
{
    type Value = T::Output;
    type Reader = MappedMutArc<
        <KeyedSubfield<Inner, Prev, K, T> as StoreField>::Reader,
        T::Output,
    >;
    type Writer = WriteGuard<
        ArcTrigger,
        MappedMutArc<
            <KeyedSubfield<Inner, Prev, K, T> as StoreField>::Writer,
            T::Output,
        >,
    >;

    fn path(&self) -> impl IntoIterator<Item = StorePathSegment> {
        let inner = self.inner.path().into_iter().collect::<StorePath>();
        let keys = self
            .inner
            .keys()
            .expect("using keys on a store with no keys");
        let this = keys
            .with_field_keys(
                inner.clone(),
                |keys| keys.get(&self.key),
                || self.inner.latest_keys(),
            )
            .flatten()
            .map(|(path, _)| path);
        inner.into_iter().chain(this)
    }

    fn get_trigger(&self, path: StorePath) -> StoreFieldTrigger {
        self.inner.get_trigger(path)
    }

    fn reader(&self) -> Option<Self::Reader> {
        let inner = self.inner.reader()?;

        let inner_path = self.inner.path().into_iter().collect();
        let keys = self.inner.keys()?;
        let index = keys
            .with_field_keys(
                inner_path,
                |keys| keys.get(&self.key),
                || self.inner.latest_keys(),
            )
            .flatten()
            .map(|(_, idx)| idx)?;

        Some(MappedMutArc::new(
            inner,
            move |n| &n[index],
            move |n| &mut n[index],
        ))
    }

    fn writer(&self) -> Option<Self::Writer> {
        let inner = self.inner.writer()?;
        let trigger = self.get_trigger(self.path().into_iter().collect());

        let inner_path = self.inner.path().into_iter().collect::<StorePath>();
        let keys = self
            .inner
            .keys()
            .expect("using keys on a store with no keys");
        let index = keys
            .with_field_keys(
                inner_path.clone(),
                |keys| keys.get(&self.key),
                || self.inner.latest_keys(),
            )
            .flatten()
            .map(|(_, idx)| idx)?;

        Some(WriteGuard::new(
            trigger.children,
            MappedMutArc::new(
                inner,
                move |n| &n[index],
                move |n| &mut n[index],
            ),
        ))
    }

    #[inline(always)]
    fn keys(&self) -> Option<KeyMap> {
        self.inner.keys()
    }
}

impl<Inner, Prev, K, T> DefinedAt for AtKeyed<Inner, Prev, K, T>
where
    for<'a> &'a T: IntoIterator,
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

impl<Inner, Prev, K, T> IsDisposed for AtKeyed<Inner, Prev, K, T>
where
    for<'a> &'a T: IntoIterator,
    Inner: IsDisposed,
{
    fn is_disposed(&self) -> bool {
        self.inner.is_disposed()
    }
}

impl<Inner, Prev, K, T> Notify for AtKeyed<Inner, Prev, K, T>
where
    K: Debug + Send + Sync + PartialEq + Eq + Hash + 'static,
    KeyedSubfield<Inner, Prev, K, T>: Clone,
    for<'a> &'a T: IntoIterator,
    Inner: StoreField<Value = Prev>,
    Prev: 'static,
    T: IndexMut<usize>,
    T::Output: Sized,
{
    fn notify(&self) {
        let trigger = self.get_trigger(self.path().into_iter().collect());
        trigger.this.notify();
        trigger.children.notify();
    }
}

impl<Inner, Prev, K, T> Track for AtKeyed<Inner, Prev, K, T>
where
    K: Debug + Send + Sync + PartialEq + Eq + Hash + 'static,
    KeyedSubfield<Inner, Prev, K, T>: Clone,
    for<'a> &'a T: IntoIterator,
    Inner: StoreField<Value = Prev>,
    Prev: 'static,
    T: IndexMut<usize>,
    T::Output: Sized,
{
    fn track(&self) {
        self.track_field();
    }
}

impl<Inner, Prev, K, T> ReadUntracked for AtKeyed<Inner, Prev, K, T>
where
    K: Debug + Send + Sync + PartialEq + Eq + Hash + 'static,
    KeyedSubfield<Inner, Prev, K, T>: Clone,
    for<'a> &'a T: IntoIterator,
    Inner: StoreField<Value = Prev>,
    Prev: 'static,
    T: IndexMut<usize>,
    T::Output: Sized,
{
    type Value = <Self as StoreField>::Reader;

    fn try_read_untracked(&self) -> Option<Self::Value> {
        self.reader()
    }
}

impl<Inner, Prev, K, T> Write for AtKeyed<Inner, Prev, K, T>
where
    K: Debug + Send + Sync + PartialEq + Eq + Hash + 'static,
    KeyedSubfield<Inner, Prev, K, T>: Clone,
    for<'a> &'a T: IntoIterator,
    Inner: StoreField<Value = Prev>,
    Prev: 'static,
    T: IndexMut<usize>,
    T::Output: Sized + 'static,
{
    type Value = T::Output;

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

impl<Inner, Prev, K, T> KeyedSubfield<Inner, Prev, K, T>
where
    Self: Clone,
    for<'a> &'a T: IntoIterator,
    Inner: StoreField<Value = Prev>,
    Prev: 'static,
    K: Debug + Send + Sync + PartialEq + Eq + Hash + 'static,
{
    /// Generates a new set of keys and registers those keys with the parent store.
    pub fn update_keys(&self) {
        let inner_path = self.path().into_iter().collect();
        let keys = self
            .inner
            .keys()
            .expect("updating keys on a store with no keys");

        // generating the latest keys out here means that if we have
        // nested keyed fields, the second field will not try to take a
        // read-lock on the key map to get the field while the first field
        // is still holding the write-lock in the closure below
        let latest = self.latest_keys();
        keys.with_field_keys(
            inner_path,
            |keys| {
                keys.update(latest);
            },
            || self.latest_keys(),
        );
    }
}

impl<Inner, Prev, K, T> IntoIterator for KeyedSubfield<Inner, Prev, K, T>
where
    Self: Clone,
    for<'a> &'a T: IntoIterator,
    Inner: Clone + StoreField<Value = Prev> + 'static,
    Prev: 'static,
    K: Debug + Send + Sync + PartialEq + Eq + Hash + 'static,
    T: IndexMut<usize> + 'static,
    T::Output: Sized,
{
    type Item = AtKeyed<Inner, Prev, K, T>;
    type IntoIter = StoreFieldKeyedIter<Inner, Prev, K, T>;

    #[track_caller]
    fn into_iter(self) -> StoreFieldKeyedIter<Inner, Prev, K, T> {
        // reactively track changes to this field
        self.update_keys();
        self.track_field();

        // get the current length of the field by accessing slice
        let reader = self.reader();

        let keys = reader
            .map(|r| {
                r.into_iter()
                    .map(|item| (self.key_fn)(item))
                    .collect::<VecDeque<_>>()
            })
            .unwrap_or_default();

        // return the iterator
        StoreFieldKeyedIter { inner: self, keys }
    }
}

/// An iterator over a [`KeyedSubfield`].
pub struct StoreFieldKeyedIter<Inner, Prev, K, T>
where
    for<'a> &'a T: IntoIterator,
    T: IndexMut<usize>,
{
    inner: KeyedSubfield<Inner, Prev, K, T>,
    keys: VecDeque<K>,
}

impl<Inner, Prev, K, T> Iterator for StoreFieldKeyedIter<Inner, Prev, K, T>
where
    Inner: StoreField<Value = Prev> + Clone + 'static,
    T: IndexMut<usize> + 'static,
    T::Output: Sized + 'static,
    for<'a> &'a T: IntoIterator,
{
    type Item = AtKeyed<Inner, Prev, K, T>;

    fn next(&mut self) -> Option<Self::Item> {
        self.keys
            .pop_front()
            .map(|key| AtKeyed::new(self.inner.clone(), key))
    }
}

impl<Inner, Prev, K, T> DoubleEndedIterator
    for StoreFieldKeyedIter<Inner, Prev, K, T>
where
    Inner: StoreField<Value = Prev> + Clone + 'static,
    T: IndexMut<usize> + 'static,
    T::Output: Sized + 'static,
    for<'a> &'a T: IntoIterator,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        self.keys
            .pop_back()
            .map(|key| AtKeyed::new(self.inner.clone(), key))
    }
}
