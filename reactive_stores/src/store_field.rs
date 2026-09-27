use crate::{
    ArcStore, KeyMap, Store, StoreFieldTrigger,
    path::{StorePath, StorePathSegment},
};
use or_poisoned::OrPoisoned;
use reactive_graph::{
    owner::Storage,
    signal::{
        ArcTrigger,
        guards::{Plain, UntrackedWriteGuard, WriteGuard},
    },
    traits::{Track, UntrackableGuard},
};
use std::{
    iter,
    ops::{Deref, DerefMut},
    sync::Arc,
};

/// Invalidates cached key metadata before releasing a store's value lock.
#[doc(hidden)]
pub struct StoreWriteGuard<T: 'static> {
    inner: UntrackedWriteGuard<T>,
    keys: KeyMap,
    mutated: bool,
}

impl<T> Deref for StoreWriteGuard<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.inner
    }
}

impl<T> DerefMut for StoreWriteGuard<T> {
    fn deref_mut(&mut self) -> &mut T {
        self.mutated = true;
        &mut self.inner
    }
}

impl<T> Drop for StoreWriteGuard<T> {
    fn drop(&mut self) {
        if self.mutated {
            // Metadata validity is independent of reactive notification tracking.
            // The inner value lock is released only after this invalidation.
            self.keys.invalidate();
        }
    }
}

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

    /// The canonical path of this field (see [`StorePath`]). Keyed collection
    /// items use their stable key segments, including when accessed by index.
    #[track_caller]
    fn path(&self) -> impl IntoIterator<Item = StorePathSegment>;

    /// The path of this field (see [`StorePath`]). Uses unkeyed indices for any keyed fields.
    #[track_caller]
    fn path_unkeyed(&self) -> impl IntoIterator<Item = StorePathSegment> {
        // TODO remove default impl next time we do a breaking release
        self.path()
    }

    /// Resolves one collection index to its canonical path segment.
    ///
    /// A supplied value is borrowed from an already-held reader or writer and
    /// must be used instead of acquiring another value lock. Without a value,
    /// keyed fields may read the collection to refresh stale key metadata.
    /// Ordinary collections keep positional segments.
    #[doc(hidden)]
    fn index_path_segment(
        &self,
        index: usize,
        _value: Option<&Self::Value>,
    ) -> Option<StorePathSegment> {
        Some(index.into())
    }

    /// Tracks collection changes that can change the key selected by an index.
    /// Ordinary positional collections need no additional dependency beyond
    /// the item and ancestor triggers; keyed collections also track reorders.
    #[doc(hidden)]
    fn track_index_selection(&self) {}

    /// Reactively tracks this field.
    #[track_caller]
    fn track_field(&self) {
        self.track_self_and_ancestors();
        self.get_trigger(self.path().into_iter().collect())
            .children
            .track();
    }

    /// Tracks direct changes to this field and its ancestors, but not its children.
    ///
    /// Used by enum accessors to detect variant changes without subscribing to
    /// changes in the variant's payload. Ancestor tracking also detects replacement
    /// of a containing value. Index projections into keyed collections also
    /// track collection changes so that a reorder can change the selected item.
    #[doc(hidden)]
    #[track_caller]
    fn track_self_and_ancestors(&self) {
        let mut path = self.path().into_iter().collect::<StorePath>();
        loop {
            self.get_trigger(path.clone()).this.track();
            if path.pop().is_none() {
                break;
            }
        }
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
    type Writer = WriteGuard<Vec<ArcTrigger>, StoreWriteGuard<T>>;

    #[track_caller]
    fn get_trigger(&self, path: StorePath) -> StoreFieldTrigger {
        let triggers = &self.signals;

        triggers.write().or_poisoned().get_or_insert(path)
    }

    #[track_caller]
    fn get_trigger_unkeyed(&self, path: StorePath) -> StoreFieldTrigger {
        self.get_trigger(self.keys.keyed_path(&path))
    }

    #[track_caller]
    fn path(&self) -> impl IntoIterator<Item = StorePathSegment> {
        iter::empty()
    }

    #[track_caller]
    fn path_unkeyed(&self) -> impl IntoIterator<Item = StorePathSegment> {
        iter::empty()
    }

    #[track_caller]
    fn reader(&self) -> Option<Self::Reader> {
        Plain::try_new(Arc::clone(&self.value))
    }

    #[track_caller]
    fn writer(&self) -> Option<Self::Writer> {
        let triggers = self.triggers_for_current_path();
        let guard = UntrackedWriteGuard::try_new(Arc::clone(&self.value))?;
        Some(WriteGuard::new(
            triggers,
            StoreWriteGuard {
                inner: guard,
                keys: self.keys.clone(),
                mutated: false,
            },
        ))
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
    type Writer = <ArcStore<T> as StoreField>::Writer;

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
    fn path_unkeyed(&self) -> impl IntoIterator<Item = StorePathSegment> {
        self.inner
            .try_get_value()
            .map(|n| n.path_unkeyed().into_iter().collect::<Vec<_>>())
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
