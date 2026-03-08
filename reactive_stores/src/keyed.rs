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
    ops::{Deref, DerefMut, Index, IndexMut},
    panic::Location,
};

/// Accesses an item from a keyed collection.
///
/// `K` is the identity key type used to uniquely identify entries. Collections
/// that are indexed by position (like `Vec`) can implement this for any `K`,
/// ignoring the key and using the `index` parameter instead. Collections that
/// are indexed by key (like `HashMap`) use the `key` parameter.
pub trait KeyedAccess<K> {
    /// Collection values.
    type Value;
    /// Acquire read-only access to a value.
    fn keyed(&self, index: usize, key: &K) -> &Self::Value;
    /// Acquire mutable access to a value.
    fn keyed_mut(&mut self, index: usize, key: &K) -> &mut Self::Value;
}
impl<K, T> KeyedAccess<K> for VecDeque<T> {
    type Value = T;
    fn keyed(&self, index: usize, _key: &K) -> &Self::Value {
        self.index(index)
    }
    fn keyed_mut(&mut self, index: usize, _key: &K) -> &mut Self::Value {
        self.index_mut(index)
    }
}
impl<K, T> KeyedAccess<K> for Vec<T> {
    type Value = T;
    fn keyed(&self, index: usize, _key: &K) -> &Self::Value {
        self.index(index)
    }
    fn keyed_mut(&mut self, index: usize, _key: &K) -> &mut Self::Value {
        self.index_mut(index)
    }
}
impl<K, T> KeyedAccess<K> for [T] {
    type Value = T;
    fn keyed(&self, index: usize, _key: &K) -> &Self::Value {
        self.index(index)
    }
    fn keyed_mut(&mut self, index: usize, _key: &K) -> &mut Self::Value {
        self.index_mut(index)
    }
}
impl<K: Ord, V> KeyedAccess<K> for std::collections::BTreeMap<K, V> {
    type Value = V;
    fn keyed(&self, _index: usize, key: &K) -> &Self::Value {
        self.get(key).expect("key does not exist")
    }
    fn keyed_mut(&mut self, _index: usize, key: &K) -> &mut Self::Value {
        self.get_mut(key).expect("key does not exist")
    }
}
impl<K: Hash + Eq, V> KeyedAccess<K> for std::collections::HashMap<K, V> {
    type Value = V;
    fn keyed(&self, _index: usize, key: &K) -> &Self::Value {
        self.get(key).expect("key does not exist")
    }
    fn keyed_mut(&mut self, _index: usize, key: &K) -> &mut Self::Value {
        self.get_mut(key).expect("key does not exist")
    }
}

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
    pub(crate) key_fn: fn(<&T as IntoIterator>::Item) -> K,
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
    type Writer = MappedMut<WriteGuard<Vec<ArcTrigger>, Inner::Writer>, T>;

    fn path(&self) -> impl IntoIterator<Item = StorePathSegment> {
        self.inner
            .path()
            .into_iter()
            .chain(iter::once(self.path_segment))
    }

    fn path_unkeyed(&self) -> impl IntoIterator<Item = StorePathSegment> {
        self.inner
            .path_unkeyed()
            .into_iter()
            .chain(iter::once(self.path_segment))
    }

    fn get_trigger(&self, path: StorePath) -> StoreFieldTrigger {
        self.inner.get_trigger(path)
    }

    fn get_trigger_unkeyed(&self, path: StorePath) -> StoreFieldTrigger {
        self.inner.get_trigger_unkeyed(path)
    }

    fn reader(&self) -> Option<Self::Reader> {
        let inner = self.inner.reader()?;
        Some(Mapped::new_with_guard(inner, self.read))
    }

    fn writer(&self) -> Option<Self::Writer> {
        let mut parent = self.inner.writer()?;
        parent.untrack();
        let triggers = self.triggers_for_current_path();
        let guard = WriteGuard::new(triggers, parent);
        Some(MappedMut::new(guard, self.read, self.write))
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

    pub(crate) fn path_at_key(
        &self,
        base_path: &StorePath,
        key: &K,
    ) -> Option<StorePath> {
        let keys = self.keys();
        let keys = keys.as_ref()?;
        let segment = keys
            .with_field_keys(
                base_path.clone(),
                |keys| (keys.get(key), vec![]),
                || self.latest_keys(),
            )
            .flatten()
            .map(|(_, idx)| idx)?;
        let mut path = base_path.clone();
        path.push(segment);
        Some(path)
    }
}

impl<Inner, Prev, K, T> KeyedSubfield<Inner, Prev, K, T>
where
    Self: Clone,
    for<'a> &'a T: IntoIterator,
    Inner: StoreField<Value = Prev>,
{
    /// Keyed access to a keyed subfield of a store.
    pub fn at_key(&self, key: K) -> AtKeyed<Inner, Prev, K, T> {
        AtKeyed::new(self.clone(), key)
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
    untracked: bool,
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
        self.untracked = true;
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

        if !self.untracked {
            self.inner.notify();
        }

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
            untracked: false,
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
            untracked: true,
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

impl<Inner, Prev, K, T> AtKeyed<Inner, Prev, K, T>
where
    for<'a> &'a T: IntoIterator,
    K: Clone,
{
    /// Key used for keyed collection access.
    pub fn key(&self) -> K {
        self.key.clone()
    }
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

impl<Inner, Prev, K, T> AtKeyed<Inner, Prev, K, T>
where
    K: Clone + Debug + Send + Sync + PartialEq + Eq + Hash + 'static,
    KeyedSubfield<Inner, Prev, K, T>: Clone,
    for<'a> &'a T: IntoIterator,
    Inner: StoreField<Value = Prev>,
    Prev: 'static,
    T: KeyedAccess<K>,
    T::Value: Sized,
{
    /// Attempt to resolve the inner index if is still exists.
    fn resolve_index(&self) -> Option<usize> {
        let inner_path = self.inner.path().into_iter().collect();
        let keys = self.inner.keys()?;
        keys.with_field_keys(
            inner_path,
            |keys| (keys.get(&self.key), vec![]),
            || self.inner.latest_keys(),
        )
        .flatten()
        .map(|(_, idx)| idx)
    }
}

impl<Inner, Prev, K, T> StoreField for AtKeyed<Inner, Prev, K, T>
where
    K: Clone + Debug + Send + Sync + PartialEq + Eq + Hash + 'static,
    KeyedSubfield<Inner, Prev, K, T>: Clone,
    for<'a> &'a T: IntoIterator,
    Inner: StoreField<Value = Prev>,
    Prev: 'static,
    T: KeyedAccess<K>,
    T::Value: Sized,
{
    type Value = T::Value;
    type Reader = MappedMutArc<
        <KeyedSubfield<Inner, Prev, K, T> as StoreField>::Reader,
        T::Value,
    >;
    type Writer = WriteGuard<
        Vec<ArcTrigger>,
        MappedMutArc<
            <KeyedSubfield<Inner, Prev, K, T> as StoreField>::Writer,
            T::Value,
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
                |keys| (keys.get(&self.key), vec![]),
                || self.inner.latest_keys(),
            )
            .flatten()
            .map(|(path, _)| path);
        inner.into_iter().chain(this)
    }

    fn path_unkeyed(&self) -> impl IntoIterator<Item = StorePathSegment> {
        let inner =
            self.inner.path_unkeyed().into_iter().collect::<StorePath>();
        let keys = self
            .inner
            .keys()
            .expect("using keys on a store with no keys");
        let this = keys
            .with_field_keys(
                inner.clone(),
                |keys| (keys.get(&self.key), vec![]),
                || self.inner.latest_keys(),
            )
            .flatten()
            .map(|(_, idx)| StorePathSegment(idx));
        inner.into_iter().chain(this)
    }

    fn get_trigger(&self, path: StorePath) -> StoreFieldTrigger {
        self.inner.get_trigger(path)
    }

    fn get_trigger_unkeyed(&self, path: StorePath) -> StoreFieldTrigger {
        self.inner.get_trigger_unkeyed(path)
    }

    fn reader(&self) -> Option<Self::Reader> {
        let inner = self.inner.reader()?;
        let index = self.resolve_index()?;
        Some(MappedMutArc::new(
            inner,
            {
                let key = self.key.clone();
                move |n| n.keyed(index, &key)
            },
            {
                let key = self.key.clone();
                move |n| n.keyed_mut(index, &key)
            },
        ))
    }

    fn writer(&self) -> Option<Self::Writer> {
        let mut inner = self.inner.writer()?;
        inner.untrack();
        let index = self.resolve_index()?;
        let triggers = self.triggers_for_current_path();
        Some(WriteGuard::new(
            triggers,
            MappedMutArc::new(
                inner,
                {
                    let key = self.key.clone();
                    move |n| n.keyed(index, &key)
                },
                {
                    let key = self.key.clone();
                    move |n| n.keyed_mut(index, &key)
                },
            ),
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
    K: Clone + Debug + Send + Sync + PartialEq + Eq + Hash + 'static,
    KeyedSubfield<Inner, Prev, K, T>: Clone,
    for<'a> &'a T: IntoIterator,
    Inner: StoreField<Value = Prev>,
    Prev: 'static,
    T: KeyedAccess<K>,
    T::Value: Sized,
{
    fn notify(&self) {
        let trigger = self.get_trigger(self.path().into_iter().collect());
        trigger.this.notify();
        trigger.children.notify();
    }
}

impl<Inner, Prev, K, T> Track for AtKeyed<Inner, Prev, K, T>
where
    K: Clone + Debug + Send + Sync + PartialEq + Eq + Hash + 'static,
    KeyedSubfield<Inner, Prev, K, T>: Clone,
    for<'a> &'a T: IntoIterator,
    Inner: StoreField<Value = Prev>,
    Prev: 'static,
    T: KeyedAccess<K>,
    T::Value: Sized,
{
    fn track(&self) {
        self.track_field();
    }
}

impl<Inner, Prev, K, T> ReadUntracked for AtKeyed<Inner, Prev, K, T>
where
    K: Clone + Debug + Send + Sync + PartialEq + Eq + Hash + 'static,
    KeyedSubfield<Inner, Prev, K, T>: Clone,
    for<'a> &'a T: IntoIterator,
    Inner: StoreField<Value = Prev>,
    Prev: 'static,
    T: KeyedAccess<K>,
    T::Value: Sized,
{
    type Value = <Self as StoreField>::Reader;

    fn try_read_untracked(&self) -> Option<Self::Value> {
        self.reader()
    }
}

impl<Inner, Prev, K, T> Write for AtKeyed<Inner, Prev, K, T>
where
    K: Clone + Debug + Send + Sync + PartialEq + Eq + Hash + 'static,
    KeyedSubfield<Inner, Prev, K, T>: Clone,
    for<'a> &'a T: IntoIterator,
    Inner: StoreField<Value = Prev>,
    Prev: 'static,
    T: KeyedAccess<K>,
    T::Value: Sized + 'static,
{
    type Value = T::Value;

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
            |keys| ((), keys.update(latest)),
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
    K: Clone + Debug + Send + Sync + PartialEq + Eq + Hash + 'static,
    T: KeyedAccess<K> + 'static,
    T::Value: Sized,
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
    T: KeyedAccess<K>,
{
    inner: KeyedSubfield<Inner, Prev, K, T>,
    keys: VecDeque<K>,
}

impl<Inner, Prev, K, T> Iterator for StoreFieldKeyedIter<Inner, Prev, K, T>
where
    Inner: StoreField<Value = Prev> + Clone + 'static,
    T: KeyedAccess<K> + 'static,
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
    T: KeyedAccess<K> + 'static,
    T::Value: Sized + 'static,
    for<'a> &'a T: IntoIterator,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        self.keys
            .pop_back()
            .map(|key| AtKeyed::new(self.inner.clone(), key))
    }
}

#[cfg(test)]
mod tests {
    use crate::{self as reactive_stores, tests::tick, AtKeyed, Store};
    use reactive_graph::{
        effect::Effect,
        traits::{Get, GetUntracked, ReadUntracked, Set, Track, Write},
    };
    use reactive_stores::Patch;
    use std::{
        collections::{BTreeMap, BTreeSet, HashMap},
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        },
    };

    #[derive(Debug, Store, Default, Patch)]
    struct TodoVec {
        #[store(key: usize = |todo| todo.id)]
        todos: Vec<Todo>,
    }
    impl TodoVec {
        fn test_data() -> Self {
            Self {
                todos: vec![
                    Todo {
                        id: 10,
                        label: "A".to_string(),
                    },
                    Todo {
                        id: 11,
                        label: "B".to_string(),
                    },
                    Todo {
                        id: 12,
                        label: "C".to_string(),
                    },
                ],
            }
        }
    }

    #[derive(Debug, Store, Default)]
    struct TodoBTreeMap {
        #[store(key: usize = |(key, _)| *key)]
        todos: BTreeMap<usize, Todo>,
    }
    impl TodoBTreeMap {
        fn test_data() -> Self {
            Self {
                todos: [
                    Todo {
                        id: 10,
                        label: "A".to_string(),
                    },
                    Todo {
                        id: 11,
                        label: "B".to_string(),
                    },
                    Todo {
                        id: 12,
                        label: "C".to_string(),
                    },
                ]
                .into_iter()
                .map(|todo| (todo.id, todo))
                .collect(),
            }
        }
    }

    #[derive(Debug, Store, Default)]
    struct TodoHashMap {
        #[store(key: String = |(key, _)| key.clone())]
        todos: HashMap<String, Todo>,
    }
    impl TodoHashMap {
        fn test_data() -> Self {
            Self {
                todos: [
                    Todo {
                        id: 10,
                        label: "A".to_string(),
                    },
                    Todo {
                        id: 11,
                        label: "B".to_string(),
                    },
                    Todo {
                        id: 12,
                        label: "C".to_string(),
                    },
                ]
                .into_iter()
                .map(|todo| (todo.label.clone(), todo))
                .collect(),
            }
        }
    }

    #[derive(
        Debug, Store, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Patch,
    )]
    struct Todo {
        id: usize,
        label: String,
    }

    impl Todo {
        pub fn new(id: usize, label: impl ToString) -> Self {
            Self {
                id,
                label: label.to_string(),
            }
        }
    }

    #[tokio::test]
    async fn keyed_fields_can_be_moved() {
        _ = any_spawner::Executor::init_tokio();

        let store = Store::new(TodoVec::test_data());
        assert_eq!(store.read_untracked().todos.len(), 3);

        // create an effect to read from each keyed field
        let a_count = Arc::new(AtomicUsize::new(0));
        let b_count = Arc::new(AtomicUsize::new(0));
        let c_count = Arc::new(AtomicUsize::new(0));

        let a = AtKeyed::new(store.todos(), 10);
        let b = AtKeyed::new(store.todos(), 11);
        let c = AtKeyed::new(store.todos(), 12);

        Effect::new_sync({
            let a_count = Arc::clone(&a_count);
            move || {
                a.track();
                a_count.fetch_add(1, Ordering::Relaxed);
            }
        });
        Effect::new_sync({
            let b_count = Arc::clone(&b_count);
            move || {
                b.track();
                b_count.fetch_add(1, Ordering::Relaxed);
            }
        });
        Effect::new_sync({
            let c_count = Arc::clone(&c_count);
            move || {
                c.track();
                c_count.fetch_add(1, Ordering::Relaxed);
            }
        });

        tick().await;
        assert_eq!(a_count.load(Ordering::Relaxed), 1);
        assert_eq!(b_count.load(Ordering::Relaxed), 1);
        assert_eq!(c_count.load(Ordering::Relaxed), 1);

        // writing at a key doesn't notify siblings
        *a.label().write() = "Foo".into();
        tick().await;
        assert_eq!(a_count.load(Ordering::Relaxed), 2);
        assert_eq!(b_count.load(Ordering::Relaxed), 1);
        assert_eq!(c_count.load(Ordering::Relaxed), 1);

        // the keys can be reorganized
        store.todos().write().swap(0, 2);
        let after = store.todos().get_untracked();
        assert_eq!(
            after,
            vec![Todo::new(12, "C"), Todo::new(11, "B"), Todo::new(10, "Foo")]
        );

        tick().await;
        assert_eq!(a_count.load(Ordering::Relaxed), 3);
        assert_eq!(b_count.load(Ordering::Relaxed), 2);
        assert_eq!(c_count.load(Ordering::Relaxed), 2);

        // and after we move the keys around, they still update the moved items
        a.label().set("Bar".into());
        let after = store.todos().get_untracked();
        assert_eq!(
            after,
            vec![Todo::new(12, "C"), Todo::new(11, "B"), Todo::new(10, "Bar")]
        );
        tick().await;
        assert_eq!(a_count.load(Ordering::Relaxed), 4);
        assert_eq!(b_count.load(Ordering::Relaxed), 2);
        assert_eq!(c_count.load(Ordering::Relaxed), 2);

        // we can remove a key and add a new one
        store.todos().write().pop();
        store.todos().write().push(Todo::new(13, "New"));
        let after = store.todos().get_untracked();
        assert_eq!(
            after,
            vec![Todo::new(12, "C"), Todo::new(11, "B"), Todo::new(13, "New")]
        );
        tick().await;
        assert_eq!(a_count.load(Ordering::Relaxed), 5);
        assert_eq!(b_count.load(Ordering::Relaxed), 3);
        assert_eq!(c_count.load(Ordering::Relaxed), 3);
    }

    #[tokio::test]
    async fn untracked_write_on_keyed_subfield_shouldnt_notify() {
        _ = any_spawner::Executor::init_tokio();

        let store = Store::new(TodoVec::test_data());
        assert_eq!(store.read_untracked().todos.len(), 3);

        // create an effect to read from the keyed subfield
        let todos_count = Arc::new(AtomicUsize::new(0));
        Effect::new_sync({
            let todos_count = Arc::clone(&todos_count);
            move || {
                store.todos().track();
                todos_count.fetch_add(1, Ordering::Relaxed);
            }
        });

        tick().await;
        assert_eq!(todos_count.load(Ordering::Relaxed), 1);

        // writing to keyed subfield notifies the iterator
        store.todos().write().push(Todo {
            id: 13,
            label: "D".into(),
        });
        tick().await;
        assert_eq!(todos_count.load(Ordering::Relaxed), 2);

        // but an untracked write doesn't
        store.todos().write_untracked().push(Todo {
            id: 14,
            label: "E".into(),
        });
        tick().await;
        assert_eq!(todos_count.load(Ordering::Relaxed), 2);
    }

    #[tokio::test]
    async fn btree_keyed_fields_can_be_moved() {
        _ = any_spawner::Executor::init_tokio();

        let store = Store::new(TodoBTreeMap::test_data());
        assert_eq!(store.read_untracked().todos.len(), 3);

        // create an effect to read from each keyed field
        let a_count = Arc::new(AtomicUsize::new(0));
        let b_count = Arc::new(AtomicUsize::new(0));
        let c_count = Arc::new(AtomicUsize::new(0));

        let a = AtKeyed::new(store.todos(), 10);
        let b = AtKeyed::new(store.todos(), 11);
        let c = AtKeyed::new(store.todos(), 12);

        Effect::new_sync({
            let a_count = Arc::clone(&a_count);
            move || {
                a.track();
                a_count.fetch_add(1, Ordering::Relaxed);
            }
        });
        Effect::new_sync({
            let b_count = Arc::clone(&b_count);
            move || {
                b.track();
                b_count.fetch_add(1, Ordering::Relaxed);
            }
        });
        Effect::new_sync({
            let c_count = Arc::clone(&c_count);
            move || {
                c.track();
                c_count.fetch_add(1, Ordering::Relaxed);
            }
        });

        tick().await;
        assert_eq!(a_count.load(Ordering::Relaxed), 1);
        assert_eq!(b_count.load(Ordering::Relaxed), 1);
        assert_eq!(c_count.load(Ordering::Relaxed), 1);

        // writing at a key doesn't notify siblings
        *a.label().write() = "Foo".into();
        tick().await;
        assert_eq!(a_count.load(Ordering::Relaxed), 2);
        assert_eq!(b_count.load(Ordering::Relaxed), 1);
        assert_eq!(c_count.load(Ordering::Relaxed), 1);
        let after = store.todos().get_untracked();
        assert_eq!(
            after.values().cloned().collect::<Vec<_>>(),
            vec![Todo::new(10, "Foo"), Todo::new(11, "B"), Todo::new(12, "C"),]
        );

        a.label().set("Bar".into());
        let after = store.todos().get_untracked();
        assert_eq!(
            after.values().cloned().collect::<Vec<_>>(),
            vec![Todo::new(10, "Bar"), Todo::new(11, "B"), Todo::new(12, "C"),]
        );
        tick().await;
        assert_eq!(a_count.load(Ordering::Relaxed), 3);
        assert_eq!(b_count.load(Ordering::Relaxed), 1);
        assert_eq!(c_count.load(Ordering::Relaxed), 1);

        // we can remove a key and add a new one
        store.todos().write().remove(&12);
        store.todos().write().insert(13, Todo::new(13, "New"));
        let after = store.todos().get_untracked();
        assert_eq!(
            after.values().cloned().collect::<Vec<_>>(),
            vec![
                Todo::new(10, "Bar"),
                Todo::new(11, "B"),
                Todo::new(13, "New")
            ]
        );
        tick().await;
        assert_eq!(a_count.load(Ordering::Relaxed), 4);
        assert_eq!(b_count.load(Ordering::Relaxed), 2);
        assert_eq!(c_count.load(Ordering::Relaxed), 2);

        assert_eq!(
            after.keys().copied().collect::<BTreeSet<usize>>(),
            BTreeSet::from([10, 11, 13])
        );

        let at_existing_key = AtKeyed::new(store.todos(), 13);
        let existing = at_existing_key.try_get();
        assert!(existing.is_some());
        assert_eq!(existing, Some(Todo::new(13, "New")));

        let at_faulty_key = AtKeyed::new(store.todos(), 999);
        let missing = at_faulty_key.try_get();
        assert!(missing.is_none(), "faulty key should return none.")
    }

    #[tokio::test]
    async fn hashmap_keyed_fields_can_be_moved() {
        _ = any_spawner::Executor::init_tokio();

        let store = Store::new(TodoHashMap::test_data());
        assert_eq!(store.read_untracked().todos.len(), 3);

        // create an effect to read from each keyed field
        let a_count = Arc::new(AtomicUsize::new(0));
        let b_count = Arc::new(AtomicUsize::new(0));
        let c_count = Arc::new(AtomicUsize::new(0));

        let a = AtKeyed::new(store.todos(), "A".to_string());
        let b = AtKeyed::new(store.todos(), "B".to_string());
        let c = AtKeyed::new(store.todos(), "C".to_string());

        Effect::new_sync({
            let a_count = Arc::clone(&a_count);
            let a = a.clone();
            move || {
                a.track();
                a_count.fetch_add(1, Ordering::Relaxed);
            }
        });
        Effect::new_sync({
            let b_count = Arc::clone(&b_count);
            move || {
                b.track();
                b_count.fetch_add(1, Ordering::Relaxed);
            }
        });
        Effect::new_sync({
            let c_count = Arc::clone(&c_count);
            move || {
                c.track();
                c_count.fetch_add(1, Ordering::Relaxed);
            }
        });

        tick().await;
        assert_eq!(a_count.load(Ordering::Relaxed), 1);
        assert_eq!(b_count.load(Ordering::Relaxed), 1);
        assert_eq!(c_count.load(Ordering::Relaxed), 1);

        // writing at a key doesn't notify siblings
        *a.clone().label().write() = "Foo".to_string();
        tick().await;
        assert_eq!(a_count.load(Ordering::Relaxed), 2);
        assert_eq!(b_count.load(Ordering::Relaxed), 1);
        assert_eq!(c_count.load(Ordering::Relaxed), 1);
        let after = store.todos().get_untracked();
        assert_eq!(
            after.values().cloned().collect::<BTreeSet<_>>(),
            BTreeSet::from([
                Todo::new(10, "Foo"),
                Todo::new(11, "B"),
                Todo::new(12, "C"),
            ])
        );

        a.clone().label().set("Bar".into());
        let after = store.todos().get_untracked();
        assert_eq!(
            after.values().cloned().collect::<BTreeSet<_>>(),
            BTreeSet::from([
                Todo::new(10, "Bar"),
                Todo::new(11, "B"),
                Todo::new(12, "C")
            ]),
        );
        tick().await;
        assert_eq!(a_count.load(Ordering::Relaxed), 3);
        assert_eq!(b_count.load(Ordering::Relaxed), 1);
        assert_eq!(c_count.load(Ordering::Relaxed), 1);

        // we can remove a key and add a new one
        store.todos().write().remove(&"C".to_string());
        store
            .todos()
            .write()
            .insert("New".to_string(), Todo::new(13, "New"));
        let after = store.todos().get_untracked();
        assert_eq!(
            after.values().cloned().collect::<BTreeSet<_>>(),
            BTreeSet::from([
                Todo::new(10, "Bar"),
                Todo::new(11, "B"),
                Todo::new(13, "New"),
            ])
        );
        tick().await;
        assert_eq!(a_count.load(Ordering::Relaxed), 4);
        assert_eq!(b_count.load(Ordering::Relaxed), 2);
        assert_eq!(c_count.load(Ordering::Relaxed), 2);

        assert_eq!(
            after.keys().cloned().collect::<BTreeSet<String>>(),
            BTreeSet::from([
                "A".to_string(),
                "B".to_string(),
                "New".to_string()
            ])
        );

        let at_existing_key = AtKeyed::new(store.todos(), "New".to_string());
        let existing = at_existing_key.try_get();
        assert!(existing.is_some());
        assert_eq!(existing, Some(Todo::new(13, "New")));

        let at_faulty_key = AtKeyed::new(store.todos(), "faulty".to_string());
        let missing = at_faulty_key.try_get();
        assert!(missing.is_none(), "faulty key should return none.")
    }

    #[test]
    fn non_usize_keys_work_for_vec() {
        #[derive(Clone, PartialEq, Eq, Hash, Debug)]
        struct MyIdType(u32);

        #[derive(Debug, Store)]
        struct Item {
            id: MyIdType,
            _value: String,
        }

        #[derive(Debug, Store)]
        struct MyStore {
            #[store(key: MyIdType = |item| item.id.clone())]
            items: Vec<Item>,
        }

        let store = Store::new(MyStore { items: Vec::new() });

        let _fields = store.items().into_iter();
    }

    #[tokio::test]
    async fn patching_keyed_field_only_notifies_changed_keys() {
        _ = any_spawner::Executor::init_tokio();

        let store = Store::new(TodoVec::test_data());
        assert_eq!(store.read_untracked().todos.len(), 3);

        // create an effect to read from each keyed field
        let whole_count = Arc::new(AtomicUsize::new(0));
        let a_count = Arc::new(AtomicUsize::new(0));
        let b_count = Arc::new(AtomicUsize::new(0));
        let c_count = Arc::new(AtomicUsize::new(0));

        let whole = store.todos();
        let a = AtKeyed::new(store.todos(), 10);
        let b = AtKeyed::new(store.todos(), 11);
        let c = AtKeyed::new(store.todos(), 12);

        Effect::new_sync({
            let whole_count = Arc::clone(&whole_count);
            move || {
                whole.track();
                whole_count.fetch_add(1, Ordering::Relaxed);
            }
        });
        Effect::new_sync({
            let a_count = Arc::clone(&a_count);
            move || {
                a.track();
                a_count.fetch_add(1, Ordering::Relaxed);
            }
        });
        Effect::new_sync({
            let b_count = Arc::clone(&b_count);
            move || {
                b.track();
                b_count.fetch_add(1, Ordering::Relaxed);
            }
        });
        Effect::new_sync({
            let c_count = Arc::clone(&c_count);
            move || {
                c.track();
                c_count.fetch_add(1, Ordering::Relaxed);
            }
        });

        tick().await;
        assert_eq!(whole_count.load(Ordering::Relaxed), 1);
        assert_eq!(a_count.load(Ordering::Relaxed), 1);
        assert_eq!(b_count.load(Ordering::Relaxed), 1);
        assert_eq!(c_count.load(Ordering::Relaxed), 1);

        // patching only notifies changed keys
        let mut new_data = store.todos().get_untracked();
        new_data.swap(0, 2);
        store.todos().patch(new_data.clone());
        let after = store.todos().get_untracked();
        assert_eq!(
            after,
            vec![Todo::new(12, "C"), Todo::new(11, "B"), Todo::new(10, "A")]
        );

        tick().await;
        assert_eq!(whole_count.load(Ordering::Relaxed), 2);
        assert_eq!(a_count.load(Ordering::Relaxed), 1);
        assert_eq!(b_count.load(Ordering::Relaxed), 1);
        assert_eq!(c_count.load(Ordering::Relaxed), 1);

        // and after we move the keys around, they still update the moved items
        a.label().set("Bar".into());
        let after = store.todos().get_untracked();
        assert_eq!(
            after,
            vec![Todo::new(12, "C"), Todo::new(11, "B"), Todo::new(10, "Bar")]
        );
        tick().await;
        assert_eq!(whole_count.load(Ordering::Relaxed), 3);
        assert_eq!(a_count.load(Ordering::Relaxed), 2);
        assert_eq!(b_count.load(Ordering::Relaxed), 1);
        assert_eq!(c_count.load(Ordering::Relaxed), 1);

        // regular writes to the collection notify all keyed children
        store.todos().write().pop();
        store.todos().write().push(Todo::new(13, "New"));
        let after = store.todos().get_untracked();
        assert_eq!(
            after,
            vec![Todo::new(12, "C"), Todo::new(11, "B"), Todo::new(13, "New")]
        );
        tick().await;
        assert_eq!(whole_count.load(Ordering::Relaxed), 4);
        assert_eq!(a_count.load(Ordering::Relaxed), 3);
        assert_eq!(b_count.load(Ordering::Relaxed), 2);
        assert_eq!(c_count.load(Ordering::Relaxed), 2);
    }
}
