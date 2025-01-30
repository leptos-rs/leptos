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
//!     // you can access individual store withs field a getter
//!     println!("todos: {:?}", &*store.todos().read());
//! });
//! # }
//!
//! // won't notify the effect that listen to `todos`
//! store.todos().write().push(Todo {
//!     label: "Test".to_string(),
//!     completed: false,
//! });
//! ```
//! ### Generated traits
//! The [`Store`](macro@Store) macro generates traits for each `struct` to which it is applied.  When working
//! within a single file more module, this is not an issue.  However, when working with multiple modules
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
//! 
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
//! use crate::reactive_stores::StoreFieldIterator;
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
//! #[derive(Store)]
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
//! ```
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
        guards::{Plain, ReadGuard, WriteGuard},
        ArcTrigger,
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
    collections::HashMap,
    fmt::Debug,
    hash::Hash,
    ops::DerefMut,
    panic::Location,
    sync::{Arc, RwLock},
};

mod arc_field;
mod deref;
mod field;
mod iter;
mod keyed;
mod option;
mod patch;
mod path;
mod store_field;
mod subfield;

pub use arc_field::ArcField;
pub use deref::*;
pub use field::Field;
pub use iter::*;
pub use keyed::*;
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
pub(crate) struct FieldKeys<K> {
    spare_keys: Vec<StorePathSegment>,
    current_key: usize,
    keys: FxHashMap<K, (StorePathSegment, usize)>,
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
            current_key: 0,
            keys,
        }
    }
}

impl<K> FieldKeys<K>
where
    K: Hash + PartialEq + Eq,
{
    fn get(&self, key: &K) -> Option<(StorePathSegment, usize)> {
        self.keys.get(key).copied()
    }

    fn next_key(&mut self) -> StorePathSegment {
        self.spare_keys.pop().unwrap_or_else(|| {
            self.current_key += 1;
            self.current_key.into()
        })
    }

    fn update(&mut self, iter: impl IntoIterator<Item = K>) {
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
                false
            }
        });

        // add new keys
        for (key, idx) in new_keys {
            // the suggestion doesn't compile because we need &mut for self.next_key(),
            // and we don't want to call that until after the check
            #[allow(clippy::map_entry)]
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
        }
    }
}

/// A map of the keys for a keyed subfield.
#[derive(Default, Clone)]
pub struct KeyMap(Arc<RwLock<HashMap<StorePath, Box<dyn Any + Send + Sync>>>>);

impl KeyMap {
    fn with_field_keys<K, T>(
        &self,
        path: StorePath,
        fun: impl FnOnce(&mut FieldKeys<K>) -> T,
        initialize: impl FnOnce() -> Vec<K>,
    ) -> Option<T>
    where
        K: Debug + Hash + PartialEq + Eq + Send + Sync + 'static,
    {
        // this incredibly defensive mechanism takes the guard twice
        // on initialization. unfortunately, this is because `initialize`, on
        // a nested keyed field can, when being initialized), can in fact try
        // to take the lock again, as we try to insert the keys of the parent
        // while inserting the keys on this child.
        //
        // see here https://github.com/leptos-rs/leptos/issues/3086
        let mut guard = self.0.write().or_poisoned();
        if guard.contains_key(&path) {
            let entry = guard.get_mut(&path)?;
            let entry = entry.downcast_mut::<FieldKeys<K>>()?;
            Some(fun(entry))
        } else {
            drop(guard);
            let keys = Box::new(FieldKeys::new(initialize()));
            let mut guard = self.0.write().or_poisoned();
            let entry = guard.entry(path).or_insert(keys);
            let entry = entry.downcast_mut::<FieldKeys<K>>()?;
            Some(fun(entry))
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
            .map(|writer| WriteGuard::new(self.clone(), writer))
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
        self.writer().map(|writer| WriteGuard::new(*self, writer))
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
            #[cfg(debug_assertions)]
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
        traits::{Read, ReadUntracked, Set, Update, Write},
    };
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
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

        _ = any_spawner::Executor::init_tokio();

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

    #[derive(Debug, Store)]
    pub struct StructWithOption {
        opt_field: Option<Todo>,
    }

    // regression test for https://github.com/leptos-rs/leptos/issues/3523
    #[tokio::test]
    async fn notifying_all_descendants() {
        use reactive_graph::traits::*;
        _ = any_spawner::Executor::init_tokio();

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
}
