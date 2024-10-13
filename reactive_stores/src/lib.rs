use or_poisoned::OrPoisoned;
use reactive_graph::{
    owner::{ArenaItem, LocalStorage, Storage, SyncStorage},
    signal::{
        guards::{Plain, ReadGuard, WriteGuard},
        ArcTrigger,
    },
    traits::{
        DefinedAt, IsDisposed, Notify, ReadUntracked, Track, UntrackableGuard,
        Write,
    },
};
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
mod field;
mod iter;
mod keyed;
mod option;
mod patch;
mod path;
mod store_field;
mod subfield;

pub use arc_field::ArcField;
pub use field::Field;
pub use iter::*;
pub use keyed::*;
pub use option::*;
pub use patch::*;
pub use path::{StorePath, StorePathSegment};
pub use store_field::{StoreField, Then};
pub use subfield::Subfield;

#[derive(Debug, Default)]
struct TriggerMap(FxHashMap<StorePath, StoreFieldTrigger>);

#[derive(Debug, Clone, Default)]
pub struct StoreFieldTrigger {
    pub this: ArcTrigger,
    pub children: ArcTrigger,
}

impl StoreFieldTrigger {
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

pub struct FieldKeys<K> {
    spare_keys: Vec<StorePathSegment>,
    current_key: usize,
    keys: FxHashMap<K, (StorePathSegment, usize)>,
}

impl<K> FieldKeys<K>
where
    K: Debug + Hash + PartialEq + Eq,
{
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
    pub fn get(&self, key: &K) -> Option<(StorePathSegment, usize)> {
        self.keys.get(key).copied()
    }

    fn next_key(&mut self) -> StorePathSegment {
        self.spare_keys.pop().unwrap_or_else(|| {
            self.current_key += 1;
            self.current_key.into()
        })
    }

    pub fn update(&mut self, iter: impl IntoIterator<Item = K>) {
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

#[derive(Default, Clone)]
pub struct KeyMap(Arc<RwLock<HashMap<StorePath, Box<dyn Any + Send + Sync>>>>);

impl KeyMap {
    pub fn with_field_keys<K, T>(
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

pub struct ArcStore<T> {
    #[cfg(debug_assertions)]
    defined_at: &'static Location<'static>,
    pub(crate) value: Arc<RwLock<T>>,
    signals: Arc<RwLock<TriggerMap>>,
    keys: KeyMap,
}

impl<T> ArcStore<T> {
    pub fn new(value: T) -> Self {
        Self {
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
            value: Arc::new(RwLock::new(value)),
            signals: Default::default(),
            keys: Default::default(),
        }
    }
}

impl<T: Debug> Debug for ArcStore<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut f = f.debug_struct("ArcStore");
        #[cfg(debug_assertions)]
        let f = f.field("defined_at", &self.defined_at);
        f.field("value", &self.value)
            .field("signals", &self.signals)
            .finish()
    }
}

impl<T> Clone for ArcStore<T> {
    fn clone(&self) -> Self {
        Self {
            #[cfg(debug_assertions)]
            defined_at: self.defined_at,
            value: Arc::clone(&self.value),
            signals: Arc::clone(&self.signals),
            keys: self.keys.clone(),
        }
    }
}

impl<T> DefinedAt for ArcStore<T> {
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
        let trigger = self.get_trigger(Default::default());
        trigger.this.track();
        trigger.children.track();
    }
}

impl<T: 'static> Notify for ArcStore<T> {
    fn notify(&self) {
        let trigger = self.get_trigger(self.path().into_iter().collect());
        trigger.this.notify();
        trigger.children.notify();
    }
}

pub struct Store<T, S = SyncStorage> {
    #[cfg(debug_assertions)]
    defined_at: &'static Location<'static>,
    inner: ArenaItem<ArcStore<T>, S>,
}

impl<T> Store<T>
where
    T: Send + Sync + 'static,
{
    pub fn new(value: T) -> Self {
        Self {
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
            inner: ArenaItem::new_with_storage(ArcStore::new(value)),
        }
    }
}

impl<T> Store<T, LocalStorage>
where
    T: 'static,
{
    pub fn new_local(value: T) -> Self {
        Self {
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
            inner: ArenaItem::new_with_storage(ArcStore::new(value)),
        }
    }
}

impl<T: Debug, S> Debug for Store<T, S>
where
    S: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut f = f.debug_struct("Store");
        #[cfg(debug_assertions)]
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

impl<T, S> IsDisposed for Store<T, S>
where
    T: 'static,
{
    #[inline(always)]
    fn is_disposed(&self) -> bool {
        self.inner.is_disposed()
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

#[cfg(test)]
mod tests {
    use crate::{self as reactive_stores, Patch, Store, StoreFieldIterator};
    use reactive_graph::{
        effect::Effect,
        traits::{Read, ReadUntracked, Set, Update, Write},
    };
    use reactive_stores_macro::{Patch, Store};
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
                println!("{:?}", store.todos().iter().collect::<Vec<_>>());
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

    #[derive(Debug, Store)]
    pub struct StructWithOption {
        opt_field: Option<Todo>,
    }
}
