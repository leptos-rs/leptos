use crate::{path::StorePath, KeyMap, KeyedAccess, KeyedSubfield, StoreField};
use indexmap::IndexMap;
use itertools::{EitherOrBoth, Itertools};
use reactive_graph::traits::{Notify, UntrackableGuard};
use std::{
    borrow::Cow,
    collections::HashMap,
    fmt::Debug,
    hash::Hash,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
    num::{
        NonZeroI128, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI8,
        NonZeroIsize, NonZeroU128, NonZeroU16, NonZeroU32, NonZeroU64,
        NonZeroU8, NonZeroUsize,
    },
    rc::Rc,
    sync::Arc,
};

/// Allows updating a store or field in place with a new value.
pub trait Patch {
    /// The type of the new value.
    type Value;

    /// Patches a store or field with a new value, only notifying fields that have changed.
    fn patch(&self, new: Self::Value);
}

impl<T> Patch for T
where
    T: StoreField,
    T::Value: PatchField,
{
    type Value = T::Value;

    fn patch(&self, new: Self::Value) {
        let path = self.path_unkeyed().into_iter().collect::<StorePath>();
        let keys = self.keys();

        if let Some(mut writer) = self.writer() {
            // don't track the writer for the whole store
            writer.untrack();
            let mut notify = |path: &StorePath| {
                self.triggers_for_path_unkeyed(path.to_owned()).notify();
            };
            writer.patch_field(new, &path, &mut notify, keys.as_ref());
        }
    }
}

impl<Inner, Prev, K, T> KeyedSubfield<Inner, Prev, K, T>
where
    Self: Clone,
    for<'a> &'a T: IntoIterator,
    Self: StoreField<Value = T>,
    <Self as StoreField>::Value: PatchFieldKeyed<K>,
    Inner: StoreField<Value = Prev>,
    T: PatchFieldKeyed<K>,
    K: Clone + Debug + Send + Sync + PartialEq + Eq + Hash + 'static,
    Prev: 'static,
{
    /// This implements a custom, keyed patch for keyed subfields.
    ///
    /// It is used in the same way as the [`Patch`] trait, but uses a keyed data diff for
    /// data structures that implement [`PatchFieldKeyed`].
    pub fn patch(&self, new: T) {
        let path = self.path_unkeyed().into_iter().collect::<StorePath>();
        let keys = self.keys();

        let structure_changed = if let Some(mut writer) = self.writer() {
            // don't track the writer for the whole store
            writer.untrack();
            let mut notify = |path: &StorePath| {
                self.triggers_for_path_unkeyed(path.to_owned()).notify();
            };
            writer.patch_field_keyed(
                new,
                &mut notify,
                keys.as_ref(),
                self.key_fn,
                |key| self.path_at_key(&path, key),
            )
        } else {
            false
        };

        if structure_changed {
            // Only notify `children` (not `this`) at the collection path, so that
            // individual keyed items — which track `this` on all ancestor paths —
            // are not spuriously notified when only the collection order has changed.
            let trigger = self.get_trigger_unkeyed(path.clone());
            trigger.children.notify();

            let mut ancestor_path = path;
            while !ancestor_path.is_empty() {
                ancestor_path.pop();
                let inner = self.get_trigger_unkeyed(ancestor_path.clone());
                inner.children.notify();
            }
        }

        self.update_keys();
    }
}

/// Allows patching a store field with some new value.
pub trait PatchField {
    /// Patches the field with some new value, only notifying if the value has changed.
    fn patch_field(
        &mut self,
        new: Self,
        path: &StorePath,
        notify: &mut dyn FnMut(&StorePath),
        keys: Option<&KeyMap>,
    );
}

/// Allows patching a collection in a store field with a new value, after doing a keyed diff.
///     
/// This takes a `key_fn` that is applied to each entry in the collection and returns a
/// unique key. Items in the old collection and new collection with the same key are treated
/// as the same value, and the items are patched using [`PatchField`].
///
/// The exact notification behavior will depend on the collection type. For example, patching
/// a vector or slice-like type should notify on the collection itself if the order of items changes.
/// If all the same keys are present in the same order, however, the parent collection will not
/// be notified; only the keyed items that have changed.
pub trait PatchFieldKeyed<K>
where
    Self: Sized + KeyedAccess<K>,
    for<'a> &'a Self: IntoIterator,
{
    /// Patches a collection with a new value.
    ///
    /// Returns `true` if the structure of the collection changed (items added, removed,
    /// or reordered). Individual item changes are notified via the `notify` callback.
    fn patch_field_keyed(
        &mut self,
        new: Self,
        notify: &mut dyn FnMut(&StorePath),
        keys: Option<&KeyMap>,
        key_fn: impl Fn(<&Self as IntoIterator>::Item) -> K,
        path_at_key: impl Fn(&K) -> Option<StorePath>,
    ) -> bool
    where
        K: Clone + Debug + Send + Sync + PartialEq + Eq + Hash + 'static;
}

macro_rules! patch_primitives {
    ($($ty:ty),*) => {
        $(impl PatchField for $ty {
            fn patch_field(
                &mut self,
                new: Self,
                path: &StorePath,
                notify: &mut dyn FnMut(&StorePath),
                _keys: Option<&KeyMap>
            ) {
                if new != *self {
                    *self = new;
                    notify(path);
                }
            }
        })*
    };
}

patch_primitives! {
    &str,
    String,
    Arc<str>,
    Rc<str>,
    Cow<'_, str>,
    usize,
    u8,
    u16,
    u32,
    u64,
    u128,
    isize,
    i8,
    i16,
    i32,
    i64,
    i128,
    f32,
    f64,
    char,
    bool,
    IpAddr,
    SocketAddr,
    SocketAddrV4,
    SocketAddrV6,
    Ipv4Addr,
    Ipv6Addr,
    NonZeroI8,
    NonZeroU8,
    NonZeroI16,
    NonZeroU16,
    NonZeroI32,
    NonZeroU32,
    NonZeroI64,
    NonZeroU64,
    NonZeroI128,
    NonZeroU128,
    NonZeroIsize,
    NonZeroUsize
}

impl<T> PatchField for Option<T>
where
    T: PatchField,
{
    fn patch_field(
        &mut self,
        new: Self,
        path: &StorePath,
        notify: &mut dyn FnMut(&StorePath),
        keys: Option<&KeyMap>,
    ) {
        match (self, new) {
            (None, None) => {}
            (old @ Some(_), None) => {
                old.take();
                notify(path);
            }
            (old @ None, new @ Some(_)) => {
                *old = new;
                notify(path);
            }
            (Some(old), Some(new)) => {
                let mut new_path = path.to_owned();
                new_path.push(0);
                old.patch_field(new, &new_path, notify, keys);
            }
        }
    }
}

impl<T> PatchField for Vec<T>
where
    T: PatchField,
{
    fn patch_field(
        &mut self,
        new: Self,
        path: &StorePath,
        notify: &mut dyn FnMut(&StorePath),
        keys: Option<&KeyMap>,
    ) {
        if self.is_empty() && new.is_empty() {
            return;
        }

        if new.is_empty() {
            self.clear();
            notify(path);
        } else if self.is_empty() {
            self.extend(new);
            notify(path);
        } else {
            let mut adds = vec![];
            let mut removes_at_end = 0;
            let mut new_path = path.to_owned();
            new_path.push(0);
            for (idx, item) in
                new.into_iter().zip_longest(self.iter_mut()).enumerate()
            {
                match item {
                    EitherOrBoth::Both(new, old) => {
                        old.patch_field(new, &new_path, notify, keys);
                    }
                    EitherOrBoth::Left(new) => {
                        adds.push(new);
                    }
                    EitherOrBoth::Right(_) => {
                        removes_at_end += 1;
                    }
                }
                new_path.replace_last(idx + 1);
            }

            let length_changed = removes_at_end > 0 || !adds.is_empty();
            self.truncate(self.len() - removes_at_end);
            self.append(&mut adds);

            if length_changed {
                notify(path);
            }
        }
    }
}

impl<K, T> PatchFieldKeyed<K> for Vec<T>
where
    T: PatchField,
{
    fn patch_field_keyed(
        &mut self,
        mut new: Self,
        notify: &mut dyn FnMut(&StorePath),
        keys: Option<&KeyMap>,
        key_fn: impl Fn(<&Self as IntoIterator>::Item) -> K,
        path_at_key: impl Fn(&K) -> Option<StorePath>,
    ) -> bool
    where
        K: Clone + Debug + Send + Sync + PartialEq + Eq + Hash + 'static,
    {
        let mut has_changed = false;

        let mut old_keyed = HashMap::new();
        let mut new_keyed = IndexMap::new();

        // first, calculate keys and indices for all the old values
        for (idx, item) in self.drain(0..).enumerate() {
            let key = key_fn(&item);
            old_keyed.insert(key, (idx, item));
        }

        // then, calculate keys and indices for all the new values
        for (idx, item) in new.drain(0..).enumerate() {
            let key = key_fn(&item);
            new_keyed.insert(key, (idx, item));
        }

        // if there are any old keys not included in the new keys, the list has changed
        for old_key in old_keyed.keys() {
            if !new_keyed.contains_key(old_key) {
                has_changed = true;
            }
        }

        // iterate over the new entries, rebuilding the `new` Vec (which we emptied with `drain` above)
        //
        // because we're using an IndexMap, this will iterate over the values in the same order
        // as the new Vec had them
        //
        // for each entry, either
        // 1) push it directly into the `new` Vec again, or
        // 2) take the old
        for (key, (new_idx, new_value)) in new_keyed {
            let old_at_key = old_keyed.remove(&key);

            match old_at_key {
                None => {
                    // add this item into the new vec
                    new.push(new_value);

                    // not found in old map, list has changed and will trigger
                    has_changed = true;
                }
                // found in old map
                Some((old_idx, old_value)) => {
                    // if indices are different, list has changed
                    if old_idx != new_idx {
                        has_changed = true;
                    }

                    // if we had an old value for this key, we're actually going to push the *old*
                    // value into the vec, and then patch it with the new value; because we're iterating
                    // in the new order, it will be at the `new_idx`
                    new.push(old_value);
                    let field_to_patch = &mut new[new_idx];

                    // now we need to actually patch the old item with this key with the new item
                    // we do this by calling patch_field(); to get the correct path, we need to get the
                    // path to the field at this key

                    // we do th
                    if let Some(path) = path_at_key(&key) {
                        field_to_patch
                            .patch_field(new_value, &path, notify, keys);
                    } else {
                        has_changed = true;
                    }
                }
            }
        }

        // update the value
        *self = new;

        has_changed
    }
}

impl<K, V> PatchFieldKeyed<K> for HashMap<K, V>
where
    V: PatchField,
    K: Eq + Hash,
{
    fn patch_field_keyed(
        &mut self,
        mut new: Self,
        notify: &mut dyn FnMut(&StorePath),
        keys: Option<&KeyMap>,
        key_fn: impl Fn(<&Self as IntoIterator>::Item) -> K,
        path_at_key: impl Fn(&K) -> Option<StorePath>,
    ) -> bool
    where
        K: Clone + Debug + Send + Sync + PartialEq + Eq + Hash + 'static,
    {
        let mut has_changed = false;

        let mut old_keyed = HashMap::new();
        let mut new_keyed = HashMap::new();

        // first, calculate keys for all the old values
        for item in self.drain() {
            let key = key_fn((&item.0, &item.1));
            old_keyed.insert(key, item);
        }

        // then, calculate keys and indices for all the new values
        for item in new.drain() {
            let key = key_fn((&item.0, &item.1));
            new_keyed.insert(key, item);
        }

        // if there are any old keys not included in the new keys, the map has changed
        for old_key in old_keyed.keys() {
            if !new_keyed.contains_key(old_key) {
                has_changed = true;
            }
        }

        // iterate over the new entries, rebuilding the `new` map (which we emptied with `drain` above)
        //
        // for each entry, either
        // 1) push it directly into the `new` map again, or
        // 2) take the old value and patch it
        for (key, new_value) in new_keyed {
            let old_at_key = old_keyed.remove(&key);

            match old_at_key {
                None => {
                    // add this item into the new map
                    new.insert(new_value.0, new_value.1);

                    // not found in old map, list has changed and will trigger
                    has_changed = true;
                }
                // found in old map
                Some(mut old_value) => {
                    // now we need to actually patch the old item with this key with the new item
                    // we do this by calling patch_field(); to get the correct path, we need to get the
                    // path to the field at this key
                    if let Some(path) = path_at_key(&key) {
                        old_value.1.patch_field(
                            new_value.1,
                            &path,
                            notify,
                            keys,
                        );
                    } else {
                        has_changed = true;
                    }

                    // and we'll insert it into the new map
                    new.insert(new_value.0, old_value.1);
                }
            }
        }

        // update the value
        *self = new;

        has_changed
    }
}

macro_rules! patch_tuple {
	($($ty:ident),*) => {
		impl<$($ty),*> PatchField for ($($ty,)*)
		where
			$($ty: PatchField),*,
		{
            fn patch_field(
                &mut self,
                new: Self,
                path: &StorePath,
                notify: &mut dyn FnMut(&StorePath),
                keys: Option<&KeyMap>
            ) {
                let mut idx = 0;
                let mut new_path = path.to_owned();
                new_path.push(0);

                paste::paste! {
                    #[allow(non_snake_case)]
                    let ($($ty,)*) = self;
                    let ($([<new_ $ty:lower>],)*) = new;
                    $(
                        $ty.patch_field([<new_ $ty:lower>], &new_path, notify, keys);
                        idx += 1;
                        new_path.replace_last(idx);
                    )*
                }
            }
        }
    }
}

impl PatchField for () {
    fn patch_field(
        &mut self,
        _new: Self,
        _path: &StorePath,
        _notify: &mut dyn FnMut(&StorePath),
        _keys: Option<&KeyMap>,
    ) {
    }
}

patch_tuple!(A);
patch_tuple!(A, B);
patch_tuple!(A, B, C);
patch_tuple!(A, B, C, D);
patch_tuple!(A, B, C, D, E);
patch_tuple!(A, B, C, D, E, F);
patch_tuple!(A, B, C, D, E, F, G);
patch_tuple!(A, B, C, D, E, F, G, H);
patch_tuple!(A, B, C, D, E, F, G, H, I);
patch_tuple!(A, B, C, D, E, F, G, H, I, J);
patch_tuple!(A, B, C, D, E, F, G, H, I, J, K);
patch_tuple!(A, B, C, D, E, F, G, H, I, J, K, L);
patch_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M);
patch_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N);
patch_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
patch_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);
patch_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q);
patch_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R);
patch_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S);
patch_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T);
patch_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U);
patch_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V);
patch_tuple!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W
);
patch_tuple!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X
);
patch_tuple!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y
);
patch_tuple!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y,
    Z
);
