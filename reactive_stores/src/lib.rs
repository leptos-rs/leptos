use reactive_graph::{
    owner::StoredValue,
    signal::{
        guards::{Mapped, Plain, ReadGuard},
        ArcTrigger,
    },
    traits::{DefinedAt, IsDisposed, ReadUntracked, Track, Trigger},
};
use rustc_hash::FxHashMap;
use std::{
    fmt::Debug,
    panic::Location,
    sync::{Arc, RwLock},
};

mod path;
mod read_store_field;
mod store_field;
mod subfield;

use path::StorePath;
use store_field::StoreField;
pub use subfield::Subfield;

#[derive(Debug, Default)]
struct TriggerMap(FxHashMap<StorePath, ArcTrigger>);

impl TriggerMap {
    fn get_or_insert(&mut self, key: StorePath) -> ArcTrigger {
        if let Some(trigger) = self.0.get(&key) {
            trigger.clone()
        } else {
            let new = ArcTrigger::new();
            self.0.insert(key, new.clone());
            new
        }
    }

    fn remove(&mut self, key: &StorePath) -> Option<ArcTrigger> {
        self.0.remove(key)
    }
}

pub struct ArcStore<T> {
    #[cfg(debug_assertions)]
    defined_at: &'static Location<'static>,
    pub(crate) value: Arc<RwLock<T>>,
    signals: Arc<RwLock<TriggerMap>>,
}

impl<T> ArcStore<T> {
    pub fn new(value: T) -> Self {
        Self {
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
            value: Arc::new(RwLock::new(value)),
            signals: Default::default(),
            /* inner: Arc::new(RwLock::new(SubscriberSet::new())), */
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

impl<T: 'static> Track for ArcStore<T> {
    fn track(&self) {
        self.get_trigger(Default::default()).trigger();
    }
}

impl<T: 'static> Trigger for ArcStore<T> {
    fn trigger(&self) {
        self.get_trigger(self.path().collect()).trigger();
    }
}

pub struct Store<T> {
    #[cfg(debug_assertions)]
    defined_at: &'static Location<'static>,
    inner: StoredValue<ArcStore<T>>,
}

impl<T> Store<T>
where
    T: Send + Sync + 'static,
{
    pub fn new(value: T) -> Self {
        Self {
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
            inner: StoredValue::new(ArcStore::new(value)),
        }
    }
}

impl<T: Debug> Debug for Store<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut f = f.debug_struct("Store");
        #[cfg(debug_assertions)]
        let f = f.field("defined_at", &self.defined_at);
        f.field("inner", &self.inner).finish()
    }
}

impl<T> Clone for Store<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Store<T> {}

impl<T> DefinedAt for Store<T> {
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

impl<T> IsDisposed for Store<T>
where
    T: 'static,
{
    #[inline(always)]
    fn is_disposed(&self) -> bool {
        !self.inner.exists()
    }
}

impl<T> ReadUntracked for Store<T>
where
    T: 'static,
{
    type Value = ReadGuard<T, Plain<T>>;

    fn try_read_untracked(&self) -> Option<Self::Value> {
        self.inner
            .try_get_value()
            .map(|inner| inner.read_untracked())
    }
}

impl<T: 'static> Track for Store<T> {
    fn track(&self) {
        if let Some(inner) = self.inner.try_get_value() {
            inner.track();
        }
    }
}

impl<T: 'static> Trigger for Store<T> {
    fn trigger(&self) {
        if let Some(inner) = self.inner.try_get_value() {
            inner.trigger();
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{self as reactive_stores, Store};
    use reactive_graph::{
        effect::Effect,
        traits::{Read, ReadUntracked, Set, Update, Writeable},
    };
    use reactive_stores_macro::Store;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    pub async fn tick() {
        tokio::time::sleep(std::time::Duration::from_micros(1)).await;
    }

    #[derive(Debug, Store)]
    struct Todos {
        user: String,
        todos: Vec<Todo>,
    }

    #[derive(Debug, Store)]
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
            move |prev| {
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

        store
            .todos()
            .write()
            .push(Todo::new("Create reactive stores"));
        tick().await;
        store.todos().write().push(Todo::new("???"));
        tick().await;
        store.todos().write().push(Todo::new("Profit!"));
        tick().await;
        // the effect doesn't read from `todos`, so the count should not have changed
        assert_eq!(combined_count.load(Ordering::Relaxed), 4);
    }
}
