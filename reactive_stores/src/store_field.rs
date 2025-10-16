use crate::{
    path::{StorePath, StorePathSegment},
    ArcStore, KeyMap, Store, StoreFieldTrigger,
};
use or_poisoned::OrPoisoned;
use reactive_graph::{
    owner::Storage,
    signal::{
        guards::{Plain, UntrackedWriteGuard, WriteGuard},
        ArcTrigger,
    },
    traits::{Track, UntrackableGuard},
};
use std::{iter, ops::Deref, sync::Arc};

/// Describes a type that can be accessed as a reactive store field.
pub trait StoreField: Sized {
    /// The value this field contains.
    type Value;
    /// A read guard to access this field.
    type Reader: Deref<Target = Self::Value>;
    /// A write guard to update this field.
    type Writer: UntrackableGuard<Target = Self::Value>;

    /// Returns the trigger that tracks access and updates for this field.
    #[track_caller]
    fn get_trigger(&self, path: StorePath) -> StoreFieldTrigger;

    /// Returns the trigger that tracks access and updates for this field.
    ///
    /// This uses *unkeyed* paths: i.e., if any field in the path is keyed, it will
    /// try to look up the key for the item at the index given in the path, rather than
    /// the keyed item.
    #[track_caller]
    fn get_trigger_unkeyed(&self, path: StorePath) -> StoreFieldTrigger;

    /// The path of this field (see [`StorePath`]).
    #[track_caller]
    fn path(&self) -> impl IntoIterator<Item = StorePathSegment>;

    /// Reactively tracks this field.
    #[track_caller]
    fn track_field(&self) {
        let path = self.path().into_iter().collect();
        let trigger = self.get_trigger(path);
        trigger.this.track();
        trigger.children.track();
    }

    /// Returns a read guard to access this field.
    #[track_caller]
    fn reader(&self) -> Option<Self::Reader>;

    /// Returns a write guard to update this field.
    #[track_caller]
    fn writer(&self) -> Option<Self::Writer>;

    /// The keys for this field, if it is a keyed field.
    #[track_caller]
    fn keys(&self) -> Option<KeyMap>;

    /// Returns triggers for this field, and all parent fields.
    fn triggers_for_current_path(&self) -> Vec<ArcTrigger> {
        self.triggers_for_path(self.path().into_iter().collect())
    }

    /// Returns triggers for the field at the given path, and all parent fields
    fn triggers_for_path(&self, path: StorePath) -> Vec<ArcTrigger> {
        let trigger = self.get_trigger(path.clone());
        let mut full_path = path;

        // build a list of triggers, starting with the full path to this node and ending with the root
        // this will mean that the root is the final item, and this path is first
        let mut triggers = Vec::with_capacity(full_path.len() + 2);
        triggers.push(trigger.this.clone());
        triggers.push(trigger.children.clone());
        while !full_path.is_empty() {
            full_path.pop();
            let inner = self.get_trigger(full_path.clone());
            triggers.push(inner.children.clone());
        }

        // when the WriteGuard is dropped, each trigger will be notified, in order
        // reversing the list will cause the triggers to be notified starting from the root,
        // then to each child down to this one
        //
        // notifying from the root down is important for things like OptionStoreExt::map()/unwrap(),
        // where it's really important that any effects that subscribe to .is_some() run before effects
        // that subscribe to the inner value, so that the inner effect can be canceled if the outer switches to `None`
        // (see https://github.com/leptos-rs/leptos/issues/3704)
        triggers.reverse();

        triggers
    }

    /// Returns triggers for the field at the given path, and all parent fields
    fn triggers_for_path_unkeyed(&self, path: StorePath) -> Vec<ArcTrigger> {
        // see notes on triggers_for_path() for additional comments on implementation

        let trigger = self.get_trigger_unkeyed(path.clone());
        let mut full_path = path;

        let mut triggers = Vec::with_capacity(full_path.len() + 2);
        triggers.push(trigger.this.clone());
        triggers.push(trigger.children.clone());
        while !full_path.is_empty() {
            full_path.pop();
            let inner = self.get_trigger_unkeyed(full_path.clone());
            triggers.push(inner.children.clone());
        }
        triggers.reverse();

        triggers
    }
}

impl<T> StoreField for ArcStore<T>
where
    T: 'static,
{
    type Value = T;
    type Reader = Plain<T>;
    type Writer = WriteGuard<ArcTrigger, UntrackedWriteGuard<T>>;

    #[track_caller]
    fn get_trigger(&self, path: StorePath) -> StoreFieldTrigger {
        let triggers = &self.signals;
        let trigger = triggers.write().or_poisoned().get_or_insert(path);
        trigger
    }

    fn get_trigger_unkeyed(&self, path: StorePath) -> StoreFieldTrigger {
        let orig_path = path.clone();

        let mut path = StorePath::with_capacity(orig_path.len());
        for segment in &orig_path {
            let parent_is_keyed = self.keys.contains_key(&path);

            if parent_is_keyed {
                let key = self
                    .keys
                    .get_key_for_index(&(path.clone(), segment.0))
                    .expect("could not find key for index");
                path.push(key);
            } else {
                path.push(*segment);
            }
        }
        self.get_trigger(path)
    }

    #[track_caller]
    fn path(&self) -> impl IntoIterator<Item = StorePathSegment> {
        iter::empty()
    }

    #[track_caller]
    fn reader(&self) -> Option<Self::Reader> {
        Plain::try_new(Arc::clone(&self.value))
    }

    #[track_caller]
    fn writer(&self) -> Option<Self::Writer> {
        let trigger = self.get_trigger(Default::default());
        let guard = UntrackedWriteGuard::try_new(Arc::clone(&self.value))?;
        Some(WriteGuard::new(trigger.children, guard))
    }

    #[track_caller]
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

    #[track_caller]
    fn get_trigger(&self, path: StorePath) -> StoreFieldTrigger {
        self.inner
            .try_get_value()
            .map(|n| n.get_trigger(path))
            .unwrap_or_default()
    }

    #[track_caller]
    fn get_trigger_unkeyed(&self, path: StorePath) -> StoreFieldTrigger {
        self.inner
            .try_get_value()
            .map(|n| n.get_trigger_unkeyed(path))
            .unwrap_or_default()
    }

    #[track_caller]
    fn path(&self) -> impl IntoIterator<Item = StorePathSegment> {
        self.inner
            .try_get_value()
            .map(|n| n.path().into_iter().collect::<Vec<_>>())
            .unwrap_or_default()
    }

    #[track_caller]
    fn reader(&self) -> Option<Self::Reader> {
        self.inner.try_get_value().and_then(|n| n.reader())
    }

    #[track_caller]
    fn writer(&self) -> Option<Self::Writer> {
        self.inner.try_get_value().and_then(|n| n.writer())
    }

    #[track_caller]
    fn keys(&self) -> Option<KeyMap> {
        self.inner.try_get_value().and_then(|inner| inner.keys())
    }
}
