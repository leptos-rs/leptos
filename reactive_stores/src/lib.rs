#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Stores are a primitive for creating deeply-nested reactive state, based on [`reactive_graph`].
//!
//! Reactive signals allow you to define atomic units of reactive state. However, signals are
//! imperfect as a mechanism for tracking reactive change in structs or collections, because
//! they do not allow you to track access to individual struct fields or individual items in a
//! collection, rather than the struct as a whole or the collection as a whole. Reactivity for
//! individual fields can be achieved by creating a struct of signals, but this has issues; it
//! means that a struct is no longer a plain data structure, but requires wrappers on each field.
//!
//! Stores attempt to solve this problem by allowing arbitrarily-deep access to the fields of some
//! data structure, while still maintaining fine-grained reactivity.
//!
//! The [`Store`](macro@Store) macro adds getters and setters for the fields of a struct. Call those getters or
//! setters on a reactive [`Store`](struct@Store) or [`ArcStore`], or to a subfield, gives you
//! access to a reactive subfield. This value of this field can be accessed via the ordinary signal
//! traits (`Get`, `Set`, and so on).
//!
//! The [`Patch`](macro@Patch) macro allows you to annotate a struct such that stores and fields have a
//! [`.patch()`](Patch::patch) method, which allows you to provide an entirely new value, but only
//! notify fields that have changed.
//!
//! Updating a field will notify its parents and children, but not its siblings.
//!
//! Stores can therefore
//! 1) work with plain Rust data types, and
//! 2) provide reactive access to individual fields
//!
//! ### Example
//!
//! ```rust
//! use reactive_graph::{
//!     effect::Effect,
//!     traits::{Read, Write},
//! };
//! use reactive_stores::{Patch, Store};
//!
//! #[derive(Debug, Store, Patch, Default)]
//! struct Todos {
//!     user: String,
//!     todos: Vec<Todo>,
//! }
//!
//! #[derive(Debug, Store, Patch, Default)]
//! struct Todo {
//!     label: String,
//!     completed: bool,
//! }
//!
//! let store = Store::new(Todos {
//!     user: "Alice".to_string(),
//!     todos: Vec::new(),
//! });
//!
//! # if false { // don't run effect in doctests
//! Effect::new(move |_| {
//!     // you can access individual store fields with a getter
//!     println!("user: {:?}", &*store.user().read());
//! });
//! # }
//!
//! // won't notify the effect that listens to `user`
//! store.todos().write().push(Todo {
//!     label: "Test".to_string(),
//!     completed: false,
//! });
//! ```
//! ### Generated traits
//! The [`Store`](macro@Store) macro generates traits for each `struct` to which it is applied.  When working
//! within a single file or module, this is not an issue.  However, when working with multiple modules
//! or files, one needs to `use` the generated traits.  The general pattern is that for each `struct`
//! named `Foo`, the macro generates a trait named `FooStoreFields`.  For example:
//! ```rust
//! pub mod foo {
//!   use reactive_stores::Store;

//!   #[derive(Store)]
//!   pub struct Foo {
//!     field: i32,
//!   }
//! }
//!
//! pub mod user {
//!   use leptos::prelude::*;
//!   use reactive_stores::Field;
//!   // Using FooStore fields here.
//!   use crate::foo::{ Foo, FooStoreFields };
//!
//!   #[component]
//!   pub fn UseFoo(foo: Field<Foo>) {
//!     // Without FooStoreFields, foo.field() would fail to compile.
//!     println!("field: {}", foo.field().read());
//!   }
//! }
//!
//! # fn main() {
//! # }
//! ```
//! ### Additional field types
//!
//! Most of the time, your structs will have fields as in the example above: the struct is comprised
//! of primitive types, builtin types like [String], or other structs that implement [Store](struct@Store) or [Field].
//! However, there are some special cases that require some additional understanding.
//!
//! #### Option
//! [`Option<T>`](std::option::Option) behaves pretty much as you would expect, utilizing [.is_some()](std::option::Option::is_some)
//! and [.is_none()](std::option::Option::is_none) to check the value and  [.unwrap()](OptionStoreExt::unwrap) method to access the inner value.  The [OptionStoreExt]
//! trait is required to use the [.unwrap()](OptionStoreExt::unwrap) method.  Here is a quick example:
//! ```rust
//! // Including the trait OptionStoreExt here is required to use unwrap()
//! use reactive_stores::{OptionStoreExt, Store};
//! use reactive_graph::traits::{Get, Read};
//!
//! #[derive(Store)]
//! struct StructWithOption {
//!     opt_field: Option<i32>,
//! }
//!
//! fn describe(store: &Store<StructWithOption>) -> String {
//!     if store.opt_field().read().is_some() {
//!         // Note here we need to use OptionStoreExt or unwrap() would not compile
//!         format!("store has a value {}", store.opt_field().unwrap().get())
//!     } else {
//!         format!("store has no value")
//!     }
//! }
//! let none_store = Store::new(StructWithOption { opt_field: None });
//! let some_store = Store::new(StructWithOption { opt_field: Some(42)});
//!
//! assert_eq!(describe(&none_store), "store has no value");
//! assert_eq!(describe(&some_store), "store has a value 42");
//! ```
//! #### Vec
//! [`Vec<T>`](std::vec::Vec) requires some special treatment when trying to access
//! elements of the vector directly.  Use the [StoreFieldIterator::at_unkeyed()] method to
//! access a particular value in a [struct@Store] or [Field] for a [std::vec::Vec].  For example:
//! ```rust
//! # use reactive_stores::Store;
//! // Needed to use at_unkeyed() on Vec
//! use reactive_stores::StoreFieldIter;
//! use reactive_stores::StoreFieldIterator;
//! use reactive_graph::traits::Read;
//! use reactive_graph::traits::Get;
//!
//! #[derive(Store)]
//! struct StructWithVec {
//!     vec_field: Vec<i32>,
//! }
//!
//! let store = Store::new(StructWithVec { vec_field: vec![1, 2, 3] });
//!
//! assert_eq!(store.vec_field().at_unkeyed(0).get(), 1);
//! assert_eq!(store.vec_field().at_unkeyed(1).get(), 2);
//! assert_eq!(store.vec_field().at_unkeyed(2).get(), 3);
//! ```
//! #### Enum
//! Enumerated types behave a bit differently as the [`Store`](macro@Store) macro builds underlying traits instead of alternate
//! enumerated structures.  Each element in an `Enum` generates methods to access it in the store: a
//! method with the name of the field gives a boolean if the `Enum` is that variant, and possible accessor
//! methods for anonymous fields of that variant.  For example:
//! ```rust
//! use reactive_stores::Store;
//! use reactive_graph::traits::{Read, Get};
//!
//! #[derive(Store, reactive_stores::Patch)]
//! enum Choices {
//!    First,
//!    Second(String),
//! }
//!
//! let choice_one = Store::new(Choices::First);
//! let choice_two = Store::new(Choices::Second("hello".to_string()));
//!
//! assert!(choice_one.first());
//! assert!(!choice_one.second());
//! // Note the use of the accessor method here .second_0()
//! assert_eq!(choice_two.second_0().unwrap().get(), "hello");
//!
//! use reactive_stores::Patch;
//! choice_two.patch(Choices::Second("updated".to_string()));
//! assert_eq!(choice_two.second_0().unwrap().get(), "updated");
//! choice_two.patch(Choices::First);
//! assert!(choice_two.first());
//! assert!(choice_two.second_0().is_none());
//! ```
//!
//! With [`Patch`](macro@Patch), matching variants recursively patch their fields;
//! patching an unchanged unit variant does not notify subscribers. Changing variants
//! replaces the enum and notifies subscribers at the enum's path. Named and tuple
//! fields support `#[patch(...)]` and `#[store(key: ...)]` just like struct fields.
//! A keyed accessor returns `Option<KeyedSubfield<...>>`:
//! ```rust
//! use reactive_stores::{Patch, Store};
//! use reactive_graph::traits::Get;
//!
//! #[derive(Store, Patch)]
//! struct Todo {
//!     id: usize,
//!     completed: bool,
//! }
//!
//! #[derive(Store, Patch)]
//! enum TodosState {
//!     Loading,
//!     Ready {
//!         #[store(key: usize = |todo| todo.id)]
//!         todos: Vec<Todo>,
//!     },
//! }
//!
//! let store = Store::new(TodosState::Ready {
//!     todos: vec![Todo { id: 1, completed: false }],
//! });
//! let todo = store.ready_todos().unwrap().at_key(1);
//! store.patch(TodosState::Ready {
//!     todos: vec![
//!         Todo { id: 2, completed: false },
//!         Todo { id: 1, completed: true },
//!     ],
//! });
//! assert!(todo.completed().get());
//! ```
//!
//! Keyed handles follow their key when a containing value is replaced. The registry
//! lazily refreshes a collection's key mapping on its first keyed access after a
//! store write; subsequent reads reuse the mapping. Rebuilding keys scans the
//! collection once; retiring removed items drops their descendant metadata
//! without scanning unrelated paths. Writes invalidate
//! cached mappings even when reactive notifications are suppressed. Invalidation
//! is conservatively store-wide, so a write to an unrelated field may also cause
//! a collection to refresh on its next access. Constructing an enum accessor does
//! not scan its collection.
//!
//! Patching the whole enum updates matching items by key; structural collection changes notify at
//! the collection's path. Calling `.patch()` on the keyed accessor itself preserves
//! the more selective collection-structure notifications of [`KeyedSubfield`].
//!
//! Access through `.at_unkeyed(index)` remains positional, even on a keyed
//! collection: after a reorder it selects the new item at that index, while
//! `.at_key(key)` follows the key. Indexed observers also track collection
//! structure changes, without subscribing to unrelated item payload writes.
//! Both access routes share the current item's keyed metadata and notifications.
//!
//! Enum accessors and variant checks track direct changes to the enum and its
//! ancestors, but not changes to its payload fields. Reading a field returned by an
//! accessor additionally tracks that field, without subscribing to its siblings.
//! Replacing or writing to the whole enum (or an ancestor) still notifies these
//! subscribers, even if the variant remains the same.
//! A previously obtained field handle must not be read or written after switching to
//! a different variant; reacquire it through the accessor instead, even if the enum
//! later returns to the original variant. Field paths are local to each variant.
//! Accessors and enum patches discard the previous variant's keyed metadata when
//! they observe a variant change, so reused paths can have different key types.
//!
//! #### Box
//! [`Box<T>`](std::boxed::Box) also requires some special treatment in how you dereference elements of the Box, especially
//! when trying to build a recursive data structure.  [DerefField](trait@DerefField) provides a [.deref_value()](DerefField::deref_field) method to access
//! the inner value.  For example:
//! ```rust
//! // Note here we need to use DerefField to use deref_field() and OptionStoreExt to use unwrap()
//! use reactive_stores::{Store, DerefField, OptionStoreExt};
//! use reactive_graph::traits::{ Read, Get };
//!
//! #[derive(Store)]
//! struct List {
//!     value: i32,
//!     #[store]
//!     child: Option<Box<List>>,
//! }
//!
//! let tree = Store::new(List {
//!     value: 1,
//!     child: Some(Box::new(List { value: 2, child: None })),
//! });
//!
//! assert_eq!(tree.child().unwrap().deref_field().value().get(), 2);
//! ```
//! ### Implementation Notes
//!
//! Every struct field can be understood as an index. For example, given the following definition
//! ```rust
//! # use reactive_stores::{Store, Patch};
//! #[derive(Debug, Store, Patch, Default)]
//! struct Name {
//!     first: String,
//!     last: String,
//! }
//! ```
//! We can think of `first` as `0` and `last` as `1`. This means that any deeply-nested field of a
//! struct can be described as a path of indices. So, for example:
//! ```rust
//! # use reactive_stores::{Store, Patch};
//! #[derive(Debug, Store, Patch, Default)]
//! struct User {
//!     user: Name,
//! }
//!
//! #[derive(Debug, Store, Patch, Default)]
//! struct Name {
//!     first: String,
//!     last: String,
//! }
//! ```
//! Here, given a `User`, `first` can be understood as [`0`, `0`] and `last` is [`0`, `1`].
//!
//! This means we can implement a store as the combination of two things:
//! 1) An `Arc<RwLock<T>>` that holds the actual value
//! 2) A map from field paths to reactive "triggers," which are signals that have no value but
//!    track reactivity
//!
//! Accessing a field via its getters returns an iterator-like data structure that describes how to
//! get to that subfield. Calling `.read()` returns a guard that dereferences to the value of that
//! field in the signal inner `Arc<RwLock<_>>`, and tracks the trigger that corresponds with its
//! path; calling `.write()` returns a writeable guard, and notifies that same trigger.

use or_poisoned::OrPoisoned;
use reactive_graph::{
    owner::{ArenaItem, LocalStorage, Storage, SyncStorage},
    signal::{
        ArcTrigger,
        guards::{Plain, ReadGuard},
    },
    traits::{
        DefinedAt, Dispose, IsDisposed, Notify, ReadUntracked, Track,
        UntrackableGuard, Write,
    },
};
pub use reactive_stores_macro::{Patch, Store};
use rustc_hash::FxHashMap;
use std::{
    any::Any,
    fmt::Debug,
    hash::Hash,
    ops::DerefMut,
    panic::Location,
    sync::{
        Arc, RwLock, Weak,
        atomic::{AtomicUsize, Ordering},
    },
};

mod arc_field;
mod deref;
mod field;
mod iter;
mod keyed;
mod len;
mod option;
mod patch;
mod path;
#[cfg(feature = "serde")]
mod serde;
mod signal_or_fn;
#[cfg(feature = "slotmap")]
mod slotmap;
mod store_field;
mod subfield;

pub use arc_field::ArcField;
pub use deref::*;
pub use field::Field;
pub use iter::*;
pub use keyed::*;
pub use len::Len;
pub use option::*;
pub use patch::*;
pub use path::{StorePath, StorePathSegment};
pub use store_field::StoreField;
pub use subfield::Subfield;

#[derive(Debug, Default)]
struct TriggerMap(FxHashMap<StorePath, StoreFieldTrigger>);

/// The reactive trigger that can be used to track updates to a store field.
#[derive(Debug, Clone, Default)]
pub struct StoreFieldTrigger {
    pub(crate) this: ArcTrigger,
    pub(crate) children: ArcTrigger,
}

impl StoreFieldTrigger {
    /// Creates a new trigger.
    pub fn new() -> Self {
        Self::default()
    }
}

impl TriggerMap {
    fn get_or_insert(&mut self, key: StorePath) -> StoreFieldTrigger {
        if let Some(trigger) = self.0.get(&key) {
            trigger.clone()
        } else {
            let new = StoreFieldTrigger::new();
            self.0.insert(key, new.clone());
            new
        }
    }

    #[allow(unused)]
    fn remove(&mut self, key: &StorePath) -> Option<StoreFieldTrigger> {
        self.0.remove(key)
    }
}

/// Manages the keys for a keyed field, including the ability to remove and reuse keys.
pub struct FieldKeys<K> {
    spare_keys: Vec<StorePathSegment>,
    current_key: usize,
    keys: FxHashMap<K, (StorePathSegment, usize)>,
    changed: bool,
    retired: Vec<StorePathSegment>,
}

impl<K> FieldKeys<K>
where
    K: Debug + Hash + PartialEq + Eq,
{
    /// Creates a new set of keys.
    pub fn new(from_keys: Vec<K>) -> Self {
        let mut keys = FxHashMap::with_capacity_and_hasher(
            from_keys.len(),
            Default::default(),
        );
        for (idx, key) in from_keys.into_iter().enumerate() {
            let segment = idx.into();
            keys.insert(key, (segment, idx));
        }

        Self {
            spare_keys: Vec::new(),
            current_key: keys.len().saturating_sub(1),
            keys,
            changed: true,
            retired: Vec::new(),
        }
    }
}

impl<K> FieldKeys<K>
where
    K: Hash + PartialEq + Eq,
{
    /// Returns a copy of the path segment to the value identified by the key
    ///
    /// # Usage
    ///
    /// You shouldn't call this method from your code, since it's a part of
    /// implementation details of `reactive_stores`. This method was exposed
    /// to implement the derive `Patch` macro for keyed fields.
    #[doc(hidden)]
    pub fn get(&self, key: &K) -> Option<(StorePathSegment, usize)> {
        self.keys.get(key).copied()
    }

    fn next_key(&mut self) -> StorePathSegment {
        self.spare_keys.pop().unwrap_or_else(|| {
            self.current_key += 1;
            self.current_key.into()
        })
    }

    fn update(&mut self, iter: impl IntoIterator<Item = K>) {
        self.changed = true;
        let new_keys = iter
            .into_iter()
            .enumerate()
            .map(|(idx, key)| (key, idx))
            .collect::<FxHashMap<K, usize>>();

        // remove old keys and recycle the slots
        self.keys.retain(|key, old_entry| match new_keys.get(key) {
            Some(idx) => {
                old_entry.1 = *idx;
                true
            }
            None => {
                self.spare_keys.push(old_entry.0);
                self.retired.push(old_entry.0);
                false
            }
        });

        // add new keys
        for (key, idx) in new_keys {
            if !self.keys.contains_key(&key) {
                let path = self.next_key();
                self.keys.insert(key, (path, idx));
            }
        }
    }
}

impl<K> Default for FieldKeys<K> {
    fn default() -> Self {
        Self {
            spare_keys: Default::default(),
            current_key: Default::default(),
            keys: Default::default(),
            changed: true,
            retired: Vec::new(),
        }
    }
}

struct KeyMapEntry {
    fields: Box<dyn Any + Send + Sync>,
    indices: Vec<Option<StorePathSegment>>,
    generation: usize,
}

#[derive(Default)]
struct KeyMapNode {
    // Intermediate paths carry no collection until a real snapshot is published.
    collection: Option<KeyMapEntry>,
    variant: Option<usize>,
    // Payload writes must not wake indexed observers unless their selection changes.
    index_selection: Option<ArcTrigger>,
    // Only in-flight initializers own strong references. Removing a subtree drops
    // its identities without invalidating snapshots for unrelated paths.
    pending: Weak<()>,
    children: KeyMapChildren,
}

// Most deep paths have only one child per level. Avoid hashing those segments;
// box the branching map so every node does not pay for its inline storage.
#[derive(Default)]
enum KeyMapChildren {
    #[default]
    Empty,
    One(StorePathSegment, Box<KeyMapNode>),
    Many(Box<FxHashMap<StorePathSegment, KeyMapNode>>),
}

impl KeyMapChildren {
    fn get(&self, segment: &StorePathSegment) -> Option<&KeyMapNode> {
        match self {
            Self::Empty => None,
            Self::One(key, node) => (key == segment).then_some(node.as_ref()),
            Self::Many(children) => children.get(segment),
        }
    }

    fn get_mut(
        &mut self,
        segment: &StorePathSegment,
    ) -> Option<&mut KeyMapNode> {
        match self {
            Self::Empty => None,
            Self::One(key, node) => (key == segment).then_some(node.as_mut()),
            Self::Many(children) => children.get_mut(segment),
        }
    }

    fn get_or_insert(&mut self, segment: StorePathSegment) -> &mut KeyMapNode {
        match self {
            Self::Empty => *self = Self::One(segment, Box::default()),
            Self::One(key, _) if *key != segment => {
                let Self::One(key, node) = std::mem::take(self) else {
                    unreachable!()
                };
                let mut children = FxHashMap::default();
                children.insert(key, *node);
                *self = Self::Many(Box::new(children));
            }
            _ => {}
        }
        match self {
            Self::One(_, node) => node,
            Self::Many(children) => children.entry(segment).or_default(),
            Self::Empty => unreachable!(),
        }
    }

    fn remove(&mut self, segment: &StorePathSegment) -> Option<KeyMapNode> {
        match self {
            Self::Empty => None,
            Self::One(key, _) if key != segment => None,
            Self::One(_, _) => {
                let Self::One(_, node) = std::mem::take(self) else {
                    unreachable!()
                };
                Some(*node)
            }
            Self::Many(children) => {
                let removed = children.remove(segment)?;
                // Restore the fast path after a branch shrinks, rather than
                // making future lookup cost depend on its branching history.
                if children.len() == 1 {
                    let (key, node) = children.drain().next().unwrap();
                    *self = Self::One(key, Box::new(node));
                }
                Some(removed)
            }
        }
    }

    fn clear(&mut self) {
        *self = Self::Empty;
    }

    fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        match self {
            Self::Empty => 0,
            Self::One(..) => 1,
            Self::Many(children) => children.len(),
        }
    }
}

struct PendingSnapshot<'a> {
    state: &'a RwLock<KeyMapNode>,
    path: StorePath,
    token: Option<Arc<()>>,
}

impl Drop for PendingSnapshot<'_> {
    fn drop(&mut self) {
        // Field access can unwind while holding the registry lock. Clean up its
        // token without a second panic, but leave the registry poisoned.
        let mut state =
            self.state.write().unwrap_or_else(|err| err.into_inner());
        let token = self.token.take().expect("pending snapshot owns its token");
        state.finish_pending(&mut (&self.path).into_iter().copied(), &token);
        // Serialize the strong-reference drop too: otherwise two completions
        // could both see another owner and leave an expired token behind.
        drop(token);
    }
}

/// A map of the keys for a keyed subfield.
#[derive(Clone, Default)]
pub struct KeyMap(Arc<RwLock<KeyMapNode>>, Arc<AtomicUsize>);

impl KeyMap {
    fn track_index_selection(&self, path: StorePath) {
        let trigger = self
            .0
            .write()
            .or_poisoned()
            .node_or_insert(&path)
            .index_selection
            .get_or_insert_default()
            .clone();
        trigger.track();
    }

    fn notify_index_selection(&self, path: &StorePath) {
        let trigger = self
            .0
            .read()
            .or_poisoned()
            .node(path)
            .and_then(|node| node.index_selection.clone());
        if let Some(trigger) = trigger {
            trigger.notify();
        }
    }

    /// Records an enum variant at a canonical keyed path.
    ///
    /// Generated accessors must call this while holding the value reader. Enum
    /// patches record the current variant on entry and the new one on replacement.
    /// Callers must obtain a new accessor after switching variants; an unobserved
    /// roundtrip to the same variant does not guarantee fresh key identities.
    #[doc(hidden)]
    pub fn observe_variant(&self, path: StorePath, variant: usize) {
        let mut state = self.0.write().or_poisoned();
        state.observe_variant(path, variant);
    }

    /// Records an enum variant, translating collection indices under the registry
    /// lock. The caller must hold the value reader or writer while observing it.
    #[doc(hidden)]
    pub fn observe_variant_unkeyed(&self, path: &StorePath, variant: usize) {
        let mut state = self.0.write().or_poisoned();
        let path = state.keyed_path(path);
        state.observe_variant(path, variant);
    }

    /// Accesses the keys for a collection, initializing them if necessary.
    ///
    /// Both key-to-index and index-to-path mappings are maintained together by
    /// the registry. The callback's returned vector is retained for compatibility
    /// with older derived code; callers do not need to build a reverse mapping.
    /// The initializer runs outside the registry lock, because reading a nested
    /// collection can itself require looking up its parent's keys. It is also
    /// called on the first access after a store write invalidates the snapshot.
    ///
    /// Returns `None` if the registered field uses a different key type.
    /// This is an implementation detail used by generated code.
    #[doc(hidden)]
    pub fn with_field_keys<K, T>(
        &self,
        path: StorePath,
        fun: impl FnOnce(&mut FieldKeys<K>) -> (T, Vec<(usize, StorePathSegment)>),
        initialize: impl FnOnce() -> Vec<K>,
    ) -> Option<T>
    where
        K: Debug + Hash + PartialEq + Eq + Send + Sync + 'static,
    {
        self.try_with_field_keys(path, fun, || Some(initialize()))
    }

    fn try_with_field_keys<K, T>(
        &self,
        path: StorePath,
        fun: impl FnOnce(&mut FieldKeys<K>) -> (T, Vec<(usize, StorePathSegment)>),
        initialize: impl FnOnce() -> Option<Vec<K>>,
    ) -> Option<T>
    where
        K: Debug + Hash + Eq + Send + Sync + 'static,
    {
        self.try_with_entry(path, initialize, |node, generation, latest| {
            Self::access_entry(node, generation, latest, fun)
        })
    }

    fn try_index_segment<K>(
        &self,
        path: StorePath,
        index: usize,
        initialize: impl FnOnce() -> Option<Vec<K>>,
    ) -> Option<StorePathSegment>
    where
        K: Debug + Hash + Eq + Send + Sync + 'static,
    {
        self.try_with_entry(path, initialize, |node, generation, latest| {
            Self::access_entry(node, generation, latest, |_| ((), Vec::new()))?;
            node.collection
                .as_ref()?
                .indices
                .get(index)
                .copied()
                .flatten()
        })
    }

    fn try_with_entry<K, T>(
        &self,
        path: StorePath,
        initialize: impl FnOnce() -> Option<Vec<K>>,
        access: impl FnOnce(&mut KeyMapNode, usize, Option<Vec<K>>) -> Option<T>,
    ) -> Option<T>
    where
        K: Debug + Hash + Eq + Send + Sync + 'static,
    {
        // Associate the snapshot with the revision before reading the value.
        // A concurrent write must not make an older snapshot appear current.
        let generation = self.1.load(Ordering::Acquire);
        let mut initialize = Some(initialize);
        let mut latest = None;
        // Release the registry lock before this guard's cleanup, even on unwind.
        let mut snapshot: Option<PendingSnapshot<'_>> = None;
        loop {
            let mut entries = self.0.write().or_poisoned();
            let current = self.1.load(Ordering::Acquire);
            // Resolve once, but never recreate a path for a delayed snapshot.
            let node = if snapshot.is_some() || current != generation {
                entries.node_mut(&path)?
            } else {
                entries.node_or_insert(&path)
            };
            let retired = snapshot.as_ref().is_some_and(|snapshot| {
                node.pending.as_ptr()
                    != Arc::as_ptr(snapshot.token.as_ref().unwrap())
            });
            let entry = node.collection.as_ref();
            if current != generation || retired {
                // A delayed snapshot must never overwrite a newer mapping (or
                // resurrect a retired subtree). Reuse a current entry only.
                if entry?.generation != current {
                    return None;
                }
                return access(node, current, None);
            }
            let needs_refresh =
                entry.is_none_or(|entry| entry.generation != generation);
            if needs_refresh && latest.is_none() {
                let pending = &mut node.pending;
                let token = pending.upgrade().unwrap_or_else(|| {
                    let token = Arc::new(());
                    *pending = Arc::downgrade(&token);
                    token
                });
                snapshot = Some(PendingSnapshot {
                    state: &self.0,
                    path: path.clone(),
                    token: Some(token),
                });
                // The initializer may recursively initialize an ancestor's keys.
                // Recheck the entry afterwards: that can also retire this subtree.
                drop(entries);
                let initialize =
                    initialize.take().expect("initializer is only called once");
                latest = Some(initialize()?);
                continue;
            }
            return access(node, generation, latest);
        }
    }

    fn update_field_keys<K>(&self, path: StorePath, latest: Vec<K>)
    where
        K: Debug + Hash + Eq + Send + Sync + 'static,
    {
        let generation = self.1.load(Ordering::Acquire);
        Self::access_entry(
            self.0.write().or_poisoned().node_or_insert(&path),
            generation,
            Some(latest),
            |_| ((), Vec::new()),
        );
    }

    fn access_entry<K, T>(
        node: &mut KeyMapNode,
        generation: usize,
        mut latest: Option<Vec<K>>,
        fun: impl FnOnce(&mut FieldKeys<K>) -> (T, Vec<(usize, StorePathSegment)>),
    ) -> Option<T>
    where
        K: Debug + Hash + Eq + Send + Sync + 'static,
    {
        let entry = node.collection.get_or_insert_with(|| KeyMapEntry {
            fields: Box::new(FieldKeys::new(
                latest.take().expect("new key entries require a snapshot"),
            )),
            indices: Vec::new(),
            generation,
        });
        let fields = entry.fields.downcast_mut::<FieldKeys<K>>()?;
        if let Some(latest) = latest {
            fields.update(latest);
        }
        let (result, _) = fun(fields);
        if std::mem::take(&mut fields.changed) {
            // Replace, rather than merge, so shrinking and empty collections do
            // not retain old index mappings. Read-only lookups do not scan keys.
            entry.indices.clear();
            for &(segment, idx) in fields.keys.values() {
                if entry.indices.len() <= idx {
                    entry.indices.resize(idx + 1, None);
                }
                entry.indices[idx] = Some(segment);
            }
        }
        let retired = std::mem::take(&mut fields.retired);
        entry.generation = generation;
        // A recycled slot must not inherit the previous item's nested metadata
        // or pending snapshots. Keep the collection and its selection trigger.
        for segment in retired {
            node.children.remove(&segment);
        }
        Some(result)
    }

    fn invalidate(&self) {
        self.1.fetch_add(1, Ordering::Release);
    }

    #[track_caller]
    fn keyed_path(&self, unkeyed: &StorePath) -> StorePath {
        self.0.read().or_poisoned().keyed_path(unkeyed)
    }
}

impl KeyMapNode {
    fn observe_variant(&mut self, path: StorePath, variant: usize) {
        let node = self.node_or_insert(&path);
        if node.variant == Some(variant) {
            return;
        }
        // First observations also retire registries whose variant was never
        // recorded, since local field paths can alias across variants.
        node.children.clear();
        node.variant = Some(variant);
    }

    fn node(&self, path: &StorePath) -> Option<&Self> {
        let mut node = self;
        for segment in path {
            node = node.children.get(segment)?;
        }
        Some(node)
    }

    fn node_mut(&mut self, path: &StorePath) -> Option<&mut Self> {
        let mut node = self;
        for segment in path {
            node = node.children.get_mut(segment)?;
        }
        Some(node)
    }

    fn node_or_insert(&mut self, path: &StorePath) -> &mut Self {
        let mut node = self;
        for segment in path {
            node = node.children.get_or_insert(*segment);
        }
        node
    }

    // Walk only the completed path, pruning empty ancestors on the way back.
    // A retired initializer must not clear a replacement initializer's token.
    fn finish_pending(
        &mut self,
        path: &mut impl Iterator<Item = StorePathSegment>,
        token: &Arc<()>,
    ) -> bool {
        if let Some(segment) = path.next() {
            if let Some(child) = self.children.get_mut(&segment) {
                if child.finish_pending(path, token) {
                    self.children.remove(&segment);
                }
            }
        } else if self.pending.as_ptr() == Arc::as_ptr(token)
            && Arc::strong_count(token) == 1
        {
            self.pending = Weak::new();
        }
        self.collection.is_none()
            && self.variant.is_none()
            && self.index_selection.is_none()
            && self.pending.strong_count() == 0
            && self.children.is_empty()
    }

    #[track_caller]
    fn keyed_path(&self, unkeyed: &StorePath) -> StorePath {
        let mut path = StorePath::with_capacity(unkeyed.len());
        let mut node = Some(self);
        for segment in unkeyed {
            let segment = if let Some(entry) =
                node.and_then(|node| node.collection.as_ref())
            {
                entry
                    .indices
                    .get(segment.0)
                    .copied()
                    .flatten()
                    .unwrap_or_else(|| {
                        panic!(
                            "could not find key for index {:?}",
                            (path.clone(), segment.0)
                        )
                    })
            } else {
                *segment
            };
            path.push(segment);
            node = node.and_then(|node| node.children.get(&segment));
        }
        path
    }
}

#[cfg(test)]
mod key_map_variant_tests {
    use super::*;
    use std::{
        panic::{AssertUnwindSafe, catch_unwind},
        sync::{Barrier, mpsc},
        thread,
        time::Duration,
    };

    fn path(segments: &[usize]) -> StorePath {
        segments.iter().copied().map(Into::into).collect()
    }

    #[test]
    fn child_storage_transitions_preserve_metadata_and_pending_identity() {
        let mut children = KeyMapChildren::default();
        assert!(children.is_empty());
        assert!(children.get(&7.into()).is_none());
        assert!(children.get_mut(&7.into()).is_none());
        assert!(children.remove(&7.into()).is_none());
        let token = Arc::new(());
        let node = children.get_or_insert(7.into());
        node.variant = Some(70);
        node.pending = Arc::downgrade(&token);
        node.children.get_or_insert(9.into()).variant = Some(90);
        assert_eq!(children.get_or_insert(7.into()).variant, Some(70));
        assert!(matches!(children, KeyMapChildren::One(..)));
        assert!(children.remove(&8.into()).is_none());

        children.get_or_insert(8.into()).variant = Some(80);
        assert!(matches!(children, KeyMapChildren::Many(..)));
        assert_eq!(children.len(), 2);
        assert!(
            children
                .get(&7.into())
                .unwrap()
                .pending
                .ptr_eq(&Arc::downgrade(&token))
        );
        assert!(children.remove(&99.into()).is_none());
        assert_eq!(children.remove(&8.into()).unwrap().variant, Some(80));
        assert!(matches!(children, KeyMapChildren::One(..)));
        assert_eq!(
            children
                .get(&7.into())
                .unwrap()
                .children
                .get(&9.into())
                .unwrap()
                .variant,
            Some(90)
        );
        assert!(
            children
                .get(&7.into())
                .unwrap()
                .pending
                .ptr_eq(&Arc::downgrade(&token))
        );
        children.get_mut(&7.into()).unwrap().variant = Some(71);
        assert_eq!(children.remove(&7.into()).unwrap().variant, Some(71));
        assert!(children.is_empty());

        children.get_or_insert(3.into());
        children.clear();
        assert!(children.is_empty());
        children.get_or_insert(3.into());
        children.get_or_insert(4.into());
        children.clear();
        assert!(children.is_empty());
    }

    #[test]
    fn child_storage_matches_hashmap_through_mixed_operations() {
        let mut children = KeyMapChildren::default();
        let mut reference =
            FxHashMap::<StorePathSegment, KeyMapNode>::default();
        let mut rng = 0x123456789abcdef_u64;
        for i in 0..20_000 {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            let key = ((rng >> 8) as usize % 16).into();
            match rng % 32 {
                0 => {
                    children.clear();
                    reference.clear();
                }
                1..=12 => assert_eq!(
                    children.remove(&key).map(|n| n.variant),
                    reference.remove(&key).map(|n| n.variant)
                ),
                13..=16 => {
                    if let Some(n) = children.get_mut(&key) {
                        n.variant = Some(i);
                    }
                    if let Some(n) = reference.get_mut(&key) {
                        n.variant = Some(i);
                    }
                }
                _ => {
                    children.get_or_insert(key).variant = Some(i);
                    reference.entry(key).or_default().variant = Some(i);
                }
            }
            assert_eq!(children.len(), reference.len());
            assert_eq!(children.is_empty(), reference.is_empty());
            for key in 0..16 {
                assert_eq!(
                    children.get(&key.into()).map(|n| n.variant),
                    reference.get(&key.into()).map(|n| n.variant)
                );
            }
        }
    }

    fn seed(keys: &KeyMap, segments: &[usize]) {
        keys.update_field_keys(path(segments), vec![10usize]);
    }

    fn registered_key(
        keys: &KeyMap,
        segments: &[usize],
        key: usize,
    ) -> Option<(StorePathSegment, usize)> {
        keys.try_with_field_keys(
            path(segments),
            |fields| (fields.get(&key), vec![]),
            || None,
        )
        .flatten()
    }

    fn pending(keys: &KeyMap, segments: &[usize]) -> Weak<()> {
        keys.0
            .read()
            .or_poisoned()
            .node(&path(segments))
            .unwrap()
            .pending
            .clone()
    }

    #[test]
    fn observations_retire_only_strict_descendants() {
        let keys = KeyMap::default();
        seed(&keys, &[]);
        seed(&keys, &[0]);
        seed(&keys, &[0, 1]);
        seed(&keys, &[0, 1, 2]);
        seed(&keys, &[1]);
        keys.observe_variant(path(&[0, 3]), 0);
        keys.observe_variant(path(&[1]), 0);
        let generation = keys.1.load(Ordering::Acquire);

        // An initial observation must also clean up unobserved legacy topology.
        keys.observe_variant(path(&[0]), 0);
        assert_eq!(keys.1.load(Ordering::Acquire), generation);
        for preserved in [&[][..], &[0], &[1]] {
            assert_eq!(
                registered_key(&keys, preserved, 10),
                Some((0.into(), 0))
            );
        }
        assert_eq!(registered_key(&keys, &[0, 1], 10), None);
        assert_eq!(registered_key(&keys, &[0, 1, 2], 10), None);
        {
            let state = keys.0.read().or_poisoned();
            assert!(state.node(&path(&[0, 3])).is_none());
            assert_eq!(state.node(&path(&[1])).unwrap().variant, Some(0));
        }
        seed(&keys, &[0, 1]);
        keys.update_field_keys(path(&[0, 1]), vec![20usize, 10]);
        let identity = registered_key(&keys, &[0, 1], 10);
        keys.observe_variant(path(&[0]), 0);
        assert_eq!(registered_key(&keys, &[0, 1], 10), identity);
        keys.observe_variant(path(&[0]), 1);

        assert_eq!(keys.1.load(Ordering::Acquire), generation);
        assert_eq!(registered_key(&keys, &[0, 1], 10), None);
        assert_eq!(
            keys.0
                .read()
                .or_poisoned()
                .node(&path(&[0]))
                .unwrap()
                .variant,
            Some(1),
        );
    }

    #[test]
    fn unkeyed_observation_translates_and_slot_retirement_clears_variants() {
        let keys = KeyMap::default();
        keys.update_field_keys(path(&[0]), vec![10usize, 20]);
        keys.observe_variant(path(&[0, 0]), 0);
        seed(&keys, &[0, 0, 0]);
        keys.update_field_keys(path(&[0]), vec![20usize, 10]);
        keys.observe_variant_unkeyed(&path(&[0, 1]), 1);
        {
            let state = keys.0.read().or_poisoned();
            assert_eq!(state.node(&path(&[0, 0])).unwrap().variant, Some(1));
            assert!(state.node(&path(&[0, 0, 0])).is_none());
        }
        keys.observe_variant(path(&[0, 1]), 0);
        keys.update_field_keys(path(&[0]), vec![20usize, 30]);
        let state = keys.0.read().or_poisoned();
        assert!(state.node(&path(&[0, 0])).is_none());
        assert_eq!(state.node(&path(&[0, 1])).unwrap().variant, Some(0));
    }

    #[test]
    fn slot_retirement_rejects_delayed_snapshot_without_staling_parent() {
        for publish_new_mapping in [false, true] {
            let keys = KeyMap::default();
            seed(&keys, &[0]);
            let generation = keys.1.load(Ordering::Acquire);

            let result = keys.with_field_keys(
                path(&[0, 0, 1]),
                |fields| (fields.get(&30usize).map(|(_, index)| index), vec![]),
                || {
                    keys.update_field_keys(path(&[0]), vec![20usize]);
                    if publish_new_mapping {
                        keys.update_field_keys(path(&[0, 0, 1]), vec![30usize]);
                    }
                    vec![10usize]
                },
            );
            assert_eq!(result, publish_new_mapping.then_some(Some(0)));

            assert_eq!(keys.1.load(Ordering::Acquire), generation);
            assert_eq!(
                keys.with_field_keys(
                    path(&[0]),
                    |fields| (
                        fields.get(&20usize).map(|(_, index)| index),
                        vec![]
                    ),
                    || panic!("retirement must not refresh the parent mapping"),
                ),
                Some(Some(0)),
            );
        }
    }

    #[test]
    fn same_path_initializers_share_identity_until_last_completion() {
        let keys = KeyMap::default();
        let result = keys.with_field_keys(
            path(&[0]),
            |fields| (fields.get(&10usize).is_some(), vec![]),
            || {
                let token = pending(&keys, &[0]);
                let nested = keys.with_field_keys(
                    path(&[0]),
                    |_| ((), vec![]),
                    || {
                        assert!(pending(&keys, &[0]).ptr_eq(&token));
                        assert_eq!(token.strong_count(), 2);
                        vec![10usize]
                    },
                );
                assert_eq!(nested, Some(()));
                assert!(pending(&keys, &[0]).ptr_eq(&token));
                assert_eq!(token.strong_count(), 1);
                vec![10usize]
            },
        );
        assert_eq!(result, Some(true));
        assert!(pending(&keys, &[0]).ptr_eq(&Weak::new()));
        assert_eq!(registered_key(&keys, &[0], 10), Some((0.into(), 0)));
    }

    #[test]
    fn failed_shared_initializer_preserves_survivor() {
        for unwind in [false, true] {
            let keys = KeyMap::default();
            let result = keys.with_field_keys(
                path(&[0, 1]),
                |fields| (fields.get(&10usize), vec![]),
                || {
                    let token = pending(&keys, &[0, 1]);
                    let nested = catch_unwind(AssertUnwindSafe(|| {
                        keys.try_with_field_keys::<usize, ()>(
                            path(&[0, 1]),
                            |_| {
                                panic!(
                                    "failed initializer must not access keys"
                                )
                            },
                            || {
                                assert_eq!(token.strong_count(), 2);
                                if unwind {
                                    panic!("initializer failed");
                                }
                                None
                            },
                        )
                    }));
                    if unwind {
                        assert!(nested.is_err());
                    } else {
                        assert_eq!(nested.unwrap(), None);
                    }
                    assert!(pending(&keys, &[0, 1]).ptr_eq(&token));
                    assert_eq!(token.strong_count(), 1);
                    vec![10usize]
                },
            );
            assert_eq!(result, Some(Some((0.into(), 0))));
            assert!(pending(&keys, &[0, 1]).ptr_eq(&Weak::new()));
        }
    }

    #[test]
    fn failed_initializers_prune_only_empty_ancestors() {
        for unwind in [false, true] {
            let keys = KeyMap::default();
            keys.observe_variant(path(&[0]), 0);
            seed(&keys, &[1]);
            keys.track_index_selection(path(&[2]));
            for segments in [
                vec![],
                vec![0, 4, 5],
                vec![1, 4, 5],
                vec![2, 4, 5],
                vec![3, 4, 5],
            ] {
                let result = catch_unwind(AssertUnwindSafe(|| {
                    keys.try_with_field_keys::<usize, ()>(
                        path(&segments),
                        |_| panic!("failed initializer must not access keys"),
                        || {
                            if unwind {
                                panic!("initializer failed");
                            }
                            None
                        },
                    )
                }));
                if unwind {
                    assert!(result.is_err());
                } else {
                    assert_eq!(result.unwrap(), None);
                }
            }
            assert_eq!(registered_key(&keys, &[1], 10), Some((0.into(), 0)));
            let state = keys.0.read().or_poisoned();
            assert!(state.collection.is_none());
            assert!(state.pending.ptr_eq(&Weak::new()));
            assert_eq!(state.children.len(), 3);
            assert_eq!(state.node(&path(&[0])).unwrap().variant, Some(0));
            assert!(state.node(&path(&[2])).unwrap().index_selection.is_some());
            for segment in 0..3 {
                assert!(
                    state.node(&path(&[segment])).unwrap().children.is_empty()
                );
            }
        }
    }

    #[test]
    fn failed_parent_initializer_preserves_published_descendants() {
        let keys = KeyMap::default();
        assert_eq!(
            keys.try_with_field_keys::<usize, ()>(
                path(&[0]),
                |_| panic!("failed initializer must not access keys"),
                || {
                    seed(&keys, &[0, 1, 2]);
                    None
                },
            ),
            None,
        );
        assert_eq!(registered_key(&keys, &[0, 1, 2], 10), Some((0.into(), 0)));
        let state = keys.0.read().or_poisoned();
        let parent = state.node(&path(&[0])).unwrap();
        assert!(parent.collection.is_none());
        assert!(parent.pending.ptr_eq(&Weak::new()));
    }

    #[test]
    fn callback_unwind_cleans_pending_without_hiding_poison() {
        let keys = KeyMap::default();
        let result = catch_unwind(AssertUnwindSafe(|| {
            keys.with_field_keys::<usize, ()>(
                path(&[0, 1]),
                |_| panic!("field callback failed"),
                || vec![10],
            )
        }));
        assert!(result.is_err());
        assert!(keys.0.is_poisoned());
        let state = keys.0.read().unwrap_or_else(|err| err.into_inner());
        assert!(
            state
                .node(&path(&[0, 1]))
                .unwrap()
                .pending
                .ptr_eq(&Weak::new())
        );
    }

    #[test]
    fn concurrent_completions_remove_the_last_pending_token() {
        let keys = KeyMap::default();
        let barrier = Arc::new(Barrier::new(3));
        let (ready_tx, ready_rx) = mpsc::channel();
        let workers = (0..2)
            .map(|_| {
                let keys = keys.clone();
                let barrier = barrier.clone();
                let ready = ready_tx.clone();
                thread::spawn(move || {
                    keys.try_with_field_keys::<usize, ()>(
                        path(&[0, 1, 2]),
                        |_| panic!("failed initializer must not access keys"),
                        || {
                            ready.send(pending(&keys, &[0, 1, 2])).unwrap();
                            barrier.wait();
                            None
                        },
                    )
                })
            })
            .collect::<Vec<_>>();
        let first = ready_rx.recv_timeout(Duration::from_secs(10)).unwrap();
        let second = ready_rx.recv_timeout(Duration::from_secs(10)).unwrap();
        assert!(first.ptr_eq(&second));
        assert_eq!(first.strong_count(), 2);
        barrier.wait();
        for worker in workers {
            assert_eq!(worker.join().unwrap(), None);
        }
        assert!(keys.0.read().or_poisoned().children.is_empty());
        assert_eq!(first.strong_count(), 0);
    }

    #[test]
    fn retired_completion_does_not_clear_replacement_token() {
        let keys = KeyMap::default();
        let (ready_tx, ready_rx) = mpsc::channel();
        let (resume_tx, resume_rx) = mpsc::channel();
        let (done_tx, done_rx) = mpsc::channel();
        let timeout = Duration::from_secs(10);
        let old_keys = keys.clone();
        let worker = thread::spawn(move || {
            let result = old_keys.with_field_keys::<usize, ()>(
                path(&[0, 1, 2]),
                |_| panic!("retired snapshot must not access keys"),
                || {
                    ready_tx.send(pending(&old_keys, &[0, 1, 2])).unwrap();
                    resume_rx.recv_timeout(timeout).unwrap();
                    vec![10]
                },
            );
            done_tx.send(result).unwrap();
        });
        let old = ready_rx.recv_timeout(timeout).unwrap();
        keys.observe_variant(path(&[0]), 0);
        let result = keys.with_field_keys(
            path(&[0, 1, 2]),
            |fields| (fields.get(&20usize), vec![]),
            || {
                let replacement = pending(&keys, &[0, 1, 2]);
                assert!(!replacement.ptr_eq(&old));
                resume_tx.send(()).unwrap();
                assert_eq!(done_rx.recv_timeout(timeout).unwrap(), None);
                assert!(pending(&keys, &[0, 1, 2]).ptr_eq(&replacement));
                assert_eq!(replacement.strong_count(), 1);
                vec![20usize]
            },
        );
        worker.join().unwrap();
        assert_eq!(result, Some(Some((0.into(), 0))));
        assert!(pending(&keys, &[0, 1, 2]).ptr_eq(&Weak::new()));
        assert_eq!(registered_key(&keys, &[0, 1, 2], 10), None);
    }

    #[test]
    fn sparse_nodes_translate_then_retire_on_first_observation() {
        let keys = KeyMap::default();
        keys.update_field_keys(path(&[0, 4, 5]), vec![10usize, 20]);
        keys.update_field_keys(path(&[0, 4, 5]), vec![20usize, 10]);
        keys.update_field_keys(path(&[0, 4, 5, 1, 6]), vec![30usize, 40]);
        keys.update_field_keys(path(&[0, 4, 5, 1, 6]), vec![40usize, 30]);
        seed(&keys, &[9, 5]);
        assert_eq!(
            keys.keyed_path(&path(&[0, 4, 5, 0, 6, 0, 7])),
            path(&[0, 4, 5, 1, 6, 1, 7]),
        );
        {
            let state = keys.0.read().or_poisoned();
            for segments in [&[][..], &[0], &[0, 4]] {
                assert!(
                    state.node(&path(segments)).unwrap().collection.is_none()
                );
            }
        }
        keys.observe_variant(path(&[0, 4]), 0);
        assert_eq!(registered_key(&keys, &[0, 4, 5], 10), None);
        assert_eq!(registered_key(&keys, &[0, 4, 5, 1, 6], 40), None);
        assert_eq!(registered_key(&keys, &[9, 5], 10), Some((0.into(), 0)));
        assert_eq!(
            keys.keyed_path(&path(&[0, 4, 5, 0, 6])),
            path(&[0, 4, 5, 0, 6])
        );
    }

    #[tokio::test]
    async fn slot_retirement_preserves_parent_selection_subscribers() {
        _ = any_spawner::Executor::init_tokio();
        let keys = KeyMap::default();
        seed(&keys, &[0]);
        seed(&keys, &[0, 0, 1]);
        let runs = Arc::new(AtomicUsize::new(0));
        let effect_keys = keys.clone();
        let effect_runs = runs.clone();
        let effect = reactive_graph::effect::Effect::new_sync(move || {
            effect_keys.track_index_selection(path(&[0]));
            effect_runs.fetch_add(1, Ordering::Relaxed);
        });
        tokio::time::sleep(Duration::from_millis(10)).await;
        assert_eq!(runs.load(Ordering::Relaxed), 1);
        keys.update_field_keys(path(&[0]), vec![20usize]);
        keys.notify_index_selection(&path(&[0]));
        tokio::time::sleep(Duration::from_millis(10)).await;
        assert_eq!(runs.load(Ordering::Relaxed), 2);
        assert_eq!(registered_key(&keys, &[0, 0, 1], 10), None);
        effect.dispose();
    }

    #[test]
    fn sibling_initializer_survives_unrelated_retirement() {
        let keys = KeyMap::default();
        keys.observe_variant(path(&[0]), 0);
        let result = keys.with_field_keys(
            path(&[1]),
            |fields| (fields.get(&10usize).map(|(_, index)| index), vec![]),
            || {
                keys.observe_variant(path(&[0]), 1);
                vec![10usize]
            },
        );
        assert_eq!(result, Some(Some(0)));
    }

    #[test]
    fn enum_retirement_rejects_delayed_snapshot() {
        for first_observation in [false, true] {
            let keys = KeyMap::default();
            if !first_observation {
                keys.observe_variant(path(&[0]), 0);
            }
            // The initializer runs unlocked: retire the enum after its snapshot
            // generation was captured but before it can publish its keys.
            let result = keys.with_field_keys(
                path(&[0, 1]),
                |_| panic!("retired snapshot must not access the entry"),
                || {
                    keys.observe_variant(path(&[0]), 1);
                    vec![10usize]
                },
            );
            assert_eq!(result, None::<()>);
            assert_eq!(registered_key(&keys, &[0, 1], 10), None);
        }
    }
}

/// A reference-counted container for a reactive store.
///
/// The type `T` should be a struct that has been annotated with `#[derive(Store)]`.
///
/// This adds a getter method for each field to `Store<T>`, which allow accessing reactive versions
/// of each individual field of the struct.
pub struct ArcStore<T> {
    #[cfg(any(debug_assertions, leptos_debuginfo))]
    defined_at: &'static Location<'static>,
    pub(crate) value: Arc<RwLock<T>>,
    signals: Arc<RwLock<TriggerMap>>,
    keys: KeyMap,
}

impl<T> ArcStore<T> {
    /// Creates a new store from the initial value.
    pub fn new(value: T) -> Self {
        Self {
            #[cfg(any(debug_assertions, leptos_debuginfo))]
            defined_at: Location::caller(),
            value: Arc::new(RwLock::new(value)),
            signals: Default::default(),
            keys: Default::default(),
        }
    }
}

impl<T: Default> Default for ArcStore<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T: Debug> Debug for ArcStore<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut f = f.debug_struct("ArcStore");
        #[cfg(any(debug_assertions, leptos_debuginfo))]
        let f = f.field("defined_at", &self.defined_at);
        f.field("value", &self.value)
            .field("signals", &self.signals)
            .finish()
    }
}

impl<T> Clone for ArcStore<T> {
    fn clone(&self) -> Self {
        Self {
            #[cfg(any(debug_assertions, leptos_debuginfo))]
            defined_at: self.defined_at,
            value: Arc::clone(&self.value),
            signals: Arc::clone(&self.signals),
            keys: self.keys.clone(),
        }
    }
}

impl<T> DefinedAt for ArcStore<T> {
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

impl<T> IsDisposed for ArcStore<T> {
    #[inline(always)]
    fn is_disposed(&self) -> bool {
        false
    }
}

impl<T> ReadUntracked for ArcStore<T>
where
    T: 'static,
{
    type Value = ReadGuard<T, Plain<T>>;

    fn try_read_untracked(&self) -> Option<Self::Value> {
        Plain::try_new(Arc::clone(&self.value)).map(ReadGuard::new)
    }
}

impl<T> Write for ArcStore<T>
where
    T: 'static,
{
    type Value = T;

    fn try_write(&self) -> Option<impl UntrackableGuard<Target = Self::Value>> {
        self.writer()
    }

    fn try_write_untracked(
        &self,
    ) -> Option<impl DerefMut<Target = Self::Value>> {
        let mut writer = self.writer()?;
        writer.untrack();
        Some(writer)
    }
}

impl<T: 'static> Track for ArcStore<T> {
    fn track(&self) {
        self.track_field();
    }
}

impl<T: 'static> Notify for ArcStore<T> {
    fn notify(&self) {
        let trigger = self.get_trigger(self.path().into_iter().collect());
        trigger.this.notify();
        trigger.children.notify();
    }
}

/// An arena-allocated container for a reactive store.
///
/// The type `T` should be a struct that has been annotated with `#[derive(Store)]`.
///
/// This adds a getter method for each field to `Store<T>`, which allow accessing reactive versions
/// of each individual field of the struct.
///
/// This follows the same ownership rules as arena-allocated types like
/// [`RwSignal`](reactive_graph::signal::RwSignal).
pub struct Store<T, S = SyncStorage> {
    #[cfg(any(debug_assertions, leptos_debuginfo))]
    defined_at: &'static Location<'static>,
    inner: ArenaItem<ArcStore<T>, S>,
}

impl<T> Store<T>
where
    T: Send + Sync + 'static,
{
    /// Creates a new store with the initial value.
    pub fn new(value: T) -> Self {
        Self {
            #[cfg(any(debug_assertions, leptos_debuginfo))]
            defined_at: Location::caller(),
            inner: ArenaItem::new_with_storage(ArcStore::new(value)),
        }
    }
}

impl<T, S> PartialEq for Store<T, S> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<T, S> Eq for Store<T, S> {}

impl<T> Store<T, LocalStorage>
where
    T: 'static,
{
    /// Creates a new store for a type that is `!Send`.
    ///
    /// This pins the value to the current thread. Accessing it from any other thread will panic.
    pub fn new_local(value: T) -> Self {
        Self {
            #[cfg(any(debug_assertions, leptos_debuginfo))]
            defined_at: Location::caller(),
            inner: ArenaItem::new_with_storage(ArcStore::new(value)),
        }
    }
}

impl<T> Default for Store<T>
where
    T: Default + Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T> Default for Store<T, LocalStorage>
where
    T: Default + 'static,
{
    fn default() -> Self {
        Self::new_local(T::default())
    }
}

impl<T: Debug, S> Debug for Store<T, S>
where
    S: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut f = f.debug_struct("Store");
        #[cfg(any(debug_assertions, leptos_debuginfo))]
        let f = f.field("defined_at", &self.defined_at);
        f.field("inner", &self.inner).finish()
    }
}

impl<T, S> Clone for Store<T, S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T, S> Copy for Store<T, S> {}

impl<T, S> DefinedAt for Store<T, S> {
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

impl<T, S> IsDisposed for Store<T, S>
where
    T: 'static,
{
    #[inline(always)]
    fn is_disposed(&self) -> bool {
        self.inner.is_disposed()
    }
}

impl<T, S> Dispose for Store<T, S>
where
    T: 'static,
{
    fn dispose(self) {
        self.inner.dispose();
    }
}

impl<T, S> ReadUntracked for Store<T, S>
where
    T: 'static,
    S: Storage<ArcStore<T>>,
{
    type Value = ReadGuard<T, Plain<T>>;

    fn try_read_untracked(&self) -> Option<Self::Value> {
        self.inner
            .try_get_value()
            .and_then(|inner| inner.try_read_untracked())
    }
}

impl<T, S> Write for Store<T, S>
where
    T: 'static,
    S: Storage<ArcStore<T>>,
{
    type Value = T;

    fn try_write(&self) -> Option<impl UntrackableGuard<Target = Self::Value>> {
        self.writer()
    }

    fn try_write_untracked(
        &self,
    ) -> Option<impl DerefMut<Target = Self::Value>> {
        let mut writer = self.writer()?;
        writer.untrack();
        Some(writer)
    }
}

impl<T, S> Track for Store<T, S>
where
    T: 'static,
    S: Storage<ArcStore<T>>,
{
    fn track(&self) {
        if let Some(inner) = self.inner.try_get_value() {
            inner.track();
        }
    }
}

impl<T, S> Notify for Store<T, S>
where
    T: 'static,
    S: Storage<ArcStore<T>>,
{
    fn notify(&self) {
        if let Some(inner) = self.inner.try_get_value() {
            inner.notify();
        }
    }
}

impl<T, S> From<ArcStore<T>> for Store<T, S>
where
    T: 'static,
    S: Storage<ArcStore<T>>,
{
    fn from(value: ArcStore<T>) -> Self {
        Self {
            #[cfg(any(debug_assertions, leptos_debuginfo))]
            defined_at: value.defined_at,
            inner: ArenaItem::new_with_storage(value),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{self as reactive_stores, Patch, Store, StoreFieldIterator};
    use reactive_graph::{
        effect::Effect,
        owner::StoredValue,
        traits::{Read, ReadUntracked, Set, Track, Update, Write},
    };
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    pub async fn tick() {
        tokio::time::sleep(std::time::Duration::from_micros(1)).await;
    }

    #[derive(Debug, Store, Patch, Default)]
    struct Todos {
        user: String,
        todos: Vec<Todo>,
    }

    #[derive(Debug, Store, Patch, Default)]
    struct Todo {
        label: String,
        completed: bool,
    }

    impl Todo {
        pub fn new(label: impl ToString) -> Self {
            Self {
                label: label.to_string(),
                completed: false,
            }
        }
    }

    fn data() -> Todos {
        Todos {
            user: "Bob".to_string(),
            todos: vec![
                Todo {
                    label: "Create reactive store".to_string(),
                    completed: true,
                },
                Todo {
                    label: "???".to_string(),
                    completed: false,
                },
                Todo {
                    label: "Profit".to_string(),
                    completed: false,
                },
            ],
        }
    }

    #[derive(Debug, Clone, Store, Patch, Default)]
    struct Foo {
        id: i32,
        bar: Bar,
    }

    #[derive(Debug, Clone, Store, Patch, Default)]
    struct Bar {
        bar_signature: i32,
        baz: Baz,
    }

    #[derive(Debug, Clone, Store, Patch, Default)]
    struct Baz {
        more_data: i32,
        baw: Baw,
    }

    #[derive(Debug, Clone, Store, Patch, Default)]
    struct Baw {
        more_data: i32,
        end: i32,
    }

    #[tokio::test]
    async fn mutating_field_triggers_effect() {
        _ = any_spawner::Executor::init_tokio();

        let combined_count = Arc::new(AtomicUsize::new(0));

        let store = Store::new(data());
        assert_eq!(store.read_untracked().todos.len(), 3);
        assert_eq!(store.user().read_untracked().as_str(), "Bob");
        Effect::new_sync({
            let combined_count = Arc::clone(&combined_count);
            move |prev: Option<()>| {
                if prev.is_none() {
                    println!("first run");
                } else {
                    println!("next run");
                }
                println!("{:?}", *store.user().read());
                combined_count.fetch_add(1, Ordering::Relaxed);
            }
        });
        tick().await;
        tick().await;
        store.user().set("Greg".into());
        tick().await;
        store.user().set("Carol".into());
        tick().await;
        store.user().update(|name| name.push_str("!!!"));
        tick().await;
        // the effect reads from `user`, so it should trigger every time
        assert_eq!(combined_count.load(Ordering::Relaxed), 4);
    }

    #[tokio::test]
    async fn other_field_does_not_notify() {
        _ = any_spawner::Executor::init_tokio();

        let combined_count = Arc::new(AtomicUsize::new(0));

        let store = Store::new(data());

        Effect::new_sync({
            let combined_count = Arc::clone(&combined_count);
            move |prev: Option<()>| {
                if prev.is_none() {
                    println!("first run");
                } else {
                    println!("next run");
                }
                println!("{:?}", *store.todos().read());
                combined_count.fetch_add(1, Ordering::Relaxed);
            }
        });
        tick().await;
        store.user().set("Greg".into());
        tick().await;
        store.user().set("Carol".into());
        tick().await;
        store.user().update(|name| name.push_str("!!!"));
        tick().await;
        // the effect reads from `todos`, so it shouldn't trigger every time
        assert_eq!(combined_count.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn parent_does_notify() {
        _ = any_spawner::Executor::init_tokio();

        let combined_count = Arc::new(AtomicUsize::new(0));

        let store = Store::new(data());

        Effect::new_sync({
            let combined_count = Arc::clone(&combined_count);
            move |prev: Option<()>| {
                if prev.is_none() {
                    println!("first run");
                } else {
                    println!("next run");
                }
                println!("{:?}", *store.todos().read());
                combined_count.fetch_add(1, Ordering::Relaxed);
            }
        });
        tick().await;
        tick().await;
        store.set(Todos::default());
        tick().await;
        store.set(data());
        tick().await;
        assert_eq!(combined_count.load(Ordering::Relaxed), 3);
    }

    #[tokio::test]
    async fn changes_do_notify_parent() {
        _ = any_spawner::Executor::init_tokio();

        let combined_count = Arc::new(AtomicUsize::new(0));

        let store = Store::new(data());

        Effect::new_sync({
            let combined_count = Arc::clone(&combined_count);
            move |prev: Option<()>| {
                if prev.is_none() {
                    println!("first run");
                } else {
                    println!("next run");
                }
                println!("{:?}", *store.read());
                combined_count.fetch_add(1, Ordering::Relaxed);
            }
        });
        tick().await;
        tick().await;
        store.user().set("Greg".into());
        tick().await;
        store.user().set("Carol".into());
        tick().await;
        store.user().update(|name| name.push_str("!!!"));
        tick().await;
        store.todos().write().clear();
        tick().await;
        assert_eq!(combined_count.load(Ordering::Relaxed), 5);
    }

    #[tokio::test]
    async fn iterator_tracks_the_field() {
        _ = any_spawner::Executor::init_tokio();

        let combined_count = Arc::new(AtomicUsize::new(0));

        let store = Store::new(data());

        Effect::new_sync({
            let combined_count = Arc::clone(&combined_count);
            move |prev: Option<()>| {
                if prev.is_none() {
                    println!("first run");
                } else {
                    println!("next run");
                }
                println!(
                    "{:?}",
                    store.todos().iter_unkeyed().collect::<Vec<_>>()
                );
                combined_count.store(1, Ordering::Relaxed);
            }
        });

        tick().await;
        store
            .todos()
            .write()
            .push(Todo::new("Create reactive store?"));
        tick().await;
        store.todos().write().push(Todo::new("???"));
        tick().await;
        store.todos().write().push(Todo::new("Profit!"));
        // the effect only reads from `todos`, so it should trigger only the first time
        assert_eq!(combined_count.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn patching_only_notifies_changed_field() {
        _ = any_spawner::Executor::init_tokio();

        let combined_count = Arc::new(AtomicUsize::new(0));

        let store = Store::new(Todos {
            user: "Alice".into(),
            todos: vec![],
        });

        Effect::new_sync({
            let combined_count = Arc::clone(&combined_count);
            move |prev: Option<()>| {
                if prev.is_none() {
                    println!("first run");
                } else {
                    println!("next run");
                }
                println!("{:?}", *store.todos().read());
                combined_count.fetch_add(1, Ordering::Relaxed);
            }
        });
        tick().await;
        tick().await;
        store.patch(Todos {
            user: "Bob".into(),
            todos: vec![],
        });
        tick().await;
        store.patch(Todos {
            user: "Carol".into(),
            todos: vec![],
        });
        tick().await;
        assert_eq!(combined_count.load(Ordering::Relaxed), 1);

        store.patch(Todos {
            user: "Carol".into(),
            todos: vec![Todo {
                label: "First Todo".into(),
                completed: false,
            }],
        });
        tick().await;
        assert_eq!(combined_count.load(Ordering::Relaxed), 2);
    }

    #[tokio::test]
    async fn patching_only_notifies_changed_field_with_custom_patch() {
        _ = any_spawner::Executor::init_tokio();

        #[derive(Debug, Store, Patch, Default)]
        struct CustomTodos {
            #[patch(|this, new| *this = new)]
            user: String,
            todos: Vec<CustomTodo>,
        }

        #[derive(Debug, Store, Patch, Default)]
        struct CustomTodo {
            label: String,
            completed: bool,
        }

        let combined_count = Arc::new(AtomicUsize::new(0));

        let store = Store::new(CustomTodos {
            user: "Alice".into(),
            todos: vec![],
        });

        Effect::new_sync({
            let combined_count = Arc::clone(&combined_count);
            move |prev: Option<()>| {
                if prev.is_none() {
                    println!("first run");
                } else {
                    println!("next run");
                }
                println!("{:?}", *store.user().read());
                combined_count.fetch_add(1, Ordering::Relaxed);
            }
        });
        tick().await;
        tick().await;
        store.patch(CustomTodos {
            user: "Bob".into(),
            todos: vec![],
        });
        tick().await;
        assert_eq!(combined_count.load(Ordering::Relaxed), 2);
        store.patch(CustomTodos {
            user: "Carol".into(),
            todos: vec![],
        });
        tick().await;
        assert_eq!(combined_count.load(Ordering::Relaxed), 3);

        store.patch(CustomTodos {
            user: "Carol".into(),
            todos: vec![CustomTodo {
                label: "First CustomTodo".into(),
                completed: false,
            }],
        });
        tick().await;
        assert_eq!(combined_count.load(Ordering::Relaxed), 3);
    }

    // regression test for https://github.com/leptos-rs/leptos/issues/3523
    #[tokio::test]
    async fn notifying_all_descendants() {
        use reactive_graph::traits::*;

        _ = any_spawner::Executor::init_tokio();

        let store = Store::new(Foo {
            id: 42,
            bar: Bar {
                bar_signature: 69,
                baz: Baz {
                    more_data: 9999,
                    baw: Baw {
                        more_data: 22,
                        end: 1112,
                    },
                },
            },
        });

        let store_runs = StoredValue::new(0);
        let id_runs = StoredValue::new(0);
        let bar_runs = StoredValue::new(0);
        let bar_signature_runs = StoredValue::new(0);
        let bar_baz_runs = StoredValue::new(0);
        let more_data_runs = StoredValue::new(0);
        let baz_baw_end_runs = StoredValue::new(0);

        Effect::new_sync(move |_| {
            println!("foo: {:?}", store.get());
            *store_runs.write_value() += 1;
        });

        Effect::new_sync(move |_| {
            println!("foo.id: {:?}", store.id().get());
            *id_runs.write_value() += 1;
        });

        Effect::new_sync(move |_| {
            println!("foo.bar: {:?}", store.bar().get());
            *bar_runs.write_value() += 1;
        });

        Effect::new_sync(move |_| {
            println!(
                "foo.bar.bar_signature: {:?}",
                store.bar().bar_signature().get()
            );
            *bar_signature_runs.write_value() += 1;
        });

        Effect::new_sync(move |_| {
            println!("foo.bar.baz: {:?}", store.bar().baz().get());
            *bar_baz_runs.write_value() += 1;
        });

        Effect::new_sync(move |_| {
            println!(
                "foo.bar.baz.more_data: {:?}",
                store.bar().baz().more_data().get()
            );
            *more_data_runs.write_value() += 1;
        });

        Effect::new_sync(move |_| {
            println!(
                "foo.bar.baz.baw.end: {:?}",
                store.bar().baz().baw().end().get()
            );
            *baz_baw_end_runs.write_value() += 1;
        });

        println!("[INITIAL EFFECT RUN]");
        tick().await;
        println!("\n\n[SETTING STORE]");
        store.set(Default::default());
        tick().await;
        println!("\n\n[SETTING STORE.BAR.BAZ]");
        store.bar().baz().set(Default::default());
        tick().await;

        assert_eq!(store_runs.get_value(), 3);
        assert_eq!(id_runs.get_value(), 2);
        assert_eq!(bar_runs.get_value(), 3);
        assert_eq!(bar_signature_runs.get_value(), 2);
        assert_eq!(bar_baz_runs.get_value(), 3);
        assert_eq!(more_data_runs.get_value(), 3);
        assert_eq!(baz_baw_end_runs.get_value(), 3);
    }

    #[tokio::test]
    async fn changing_parent_notifies_subfield() {
        _ = any_spawner::Executor::init_tokio();

        let combined_count = Arc::new(AtomicUsize::new(0));

        let store = Store::new(Foo {
            id: 42,
            bar: Bar {
                bar_signature: 69,
                baz: Baz {
                    more_data: 9999,
                    baw: Baw {
                        more_data: 22,
                        end: 1112,
                    },
                },
            },
        });

        let tracked_field = store.bar().baz().more_data();

        Effect::new_sync({
            let combined_count = Arc::clone(&combined_count);
            move |prev: Option<()>| {
                if prev.is_none() {
                    println!("first run");
                } else {
                    println!("next run");
                }

                // we only track `more`, but this should still be notified
                // when its parent fields `bar` or `baz` change
                println!("{:?}", *tracked_field.read());
                combined_count.fetch_add(1, Ordering::Relaxed);
            }
        });
        tick().await;
        tick().await;

        store.bar().baz().set(Baz {
            more_data: 42,
            baw: Baw {
                more_data: 11,
                end: 31,
            },
        });
        tick().await;
        store.bar().set(Bar {
            bar_signature: 23,
            baz: Baz {
                more_data: 32,
                baw: Baw {
                    more_data: 432,
                    end: 423,
                },
            },
        });
        tick().await;

        assert_eq!(combined_count.load(Ordering::Relaxed), 3);
    }

    #[tokio::test]
    async fn changing_parent_notifies_unkeyed_child() {
        _ = any_spawner::Executor::init_tokio();

        let combined_count = Arc::new(AtomicUsize::new(0));

        let store = Store::new(data());

        let tracked_field = store.todos().at_unkeyed(0);

        Effect::new_sync({
            let combined_count = Arc::clone(&combined_count);
            move |prev: Option<()>| {
                if prev.is_none() {
                    println!("first run");
                } else {
                    println!("next run");
                }

                // we only track `more`, but this should still be notified
                // when its parent fields `bar` or `baz` change
                println!("{:?}", *tracked_field.read());
                combined_count.fetch_add(1, Ordering::Relaxed);
            }
        });
        tick().await;
        tick().await;

        store.todos().write().pop();
        tick().await;

        store.todos().write().push(Todo {
            label: "another one".into(),
            completed: false,
        });
        tick().await;

        assert_eq!(combined_count.load(Ordering::Relaxed), 3);
    }

    #[tokio::test]
    async fn writing_unkeyed_element_does_not_notify_sibling() {
        _ = any_spawner::Executor::init_tokio();

        let element0_count = Arc::new(AtomicUsize::new(0));
        let element1_count = Arc::new(AtomicUsize::new(0));

        let store = Store::new(data());

        Effect::new_sync({
            let element0_count = Arc::clone(&element0_count);
            move |_| {
                store.todos().at_unkeyed(0).track();
                element0_count.fetch_add(1, Ordering::Relaxed);
            }
        });
        Effect::new_sync({
            let element1_count = Arc::clone(&element1_count);
            move |_| {
                store.todos().at_unkeyed(1).track();
                element1_count.fetch_add(1, Ordering::Relaxed);
            }
        });

        tick().await;
        assert_eq!(element0_count.load(Ordering::Relaxed), 1);
        assert_eq!(element1_count.load(Ordering::Relaxed), 1);

        // writing element 0 should notify element 0's observer but NOT element 1's
        *store.todos().at_unkeyed(0).write() = Todo::new("changed");
        tick().await;
        assert_eq!(element0_count.load(Ordering::Relaxed), 2);
        assert_eq!(element1_count.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn untracked_write_on_subfield_shouldnt_notify() {
        _ = any_spawner::Executor::init_tokio();

        let name_count = Arc::new(AtomicUsize::new(0));

        let store = Store::new(data());

        let tracked_field = store.user();

        Effect::new_sync({
            let name_count = Arc::clone(&name_count);
            move |_| {
                tracked_field.track();
                name_count.fetch_add(1, Ordering::Relaxed);
            }
        });

        tick().await;
        assert_eq!(name_count.load(Ordering::Relaxed), 1);

        tracked_field.write().push('!');
        tick().await;
        assert_eq!(name_count.load(Ordering::Relaxed), 2);

        tracked_field.write_untracked().push('!');
        tick().await;
        assert_eq!(name_count.load(Ordering::Relaxed), 2);
    }

    #[derive(Debug, Clone, Store, Patch, Default)]
    struct Settings {
        label: Option<String>,
        retries: u8,
    }

    #[tokio::test]
    async fn subfield_converts_into_maybe_prop() {
        use reactive_graph::{traits::GetUntracked, wrappers::read::MaybeProp};

        _ = any_spawner::Executor::init_tokio();

        let store = Store::new(Settings::default());
        let label: MaybeProp<String> = store.label().into();
        let retries: MaybeProp<u8> = store.retries().into();

        assert_eq!(label.get_untracked(), None);
        assert_eq!(retries.get_untracked(), Some(0));

        // The conversion derives from the field rather than copying it, so a
        // later write is visible through the prop.
        store.label().set(Some("ready".to_string()));
        store.retries().set(3);

        assert_eq!(label.get_untracked(), Some("ready".to_string()));
        assert_eq!(retries.get_untracked(), Some(3));
    }
}
