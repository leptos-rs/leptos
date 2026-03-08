//! SlotMap support for keyed fields based on their map types.
use crate::KeyedAccess;

impl<K: slotmap::Key, V> KeyedAccess<K> for slotmap::SlotMap<K, V> {
    type Value = V;
    fn keyed(&self, _index: usize, key: &K) -> &Self::Value {
        self.get(*key).expect("key does not exist.")
    }
    fn keyed_mut(&mut self, _index: usize, key: &K) -> &mut Self::Value {
        self.get_mut(*key).expect("key does not exist")
    }
}
impl<K: slotmap::Key, V> KeyedAccess<K> for slotmap::DenseSlotMap<K, V> {
    type Value = V;
    fn keyed(&self, _index: usize, key: &K) -> &Self::Value {
        self.get(*key).expect("key does not exist.")
    }
    fn keyed_mut(&mut self, _index: usize, key: &K) -> &mut Self::Value {
        self.get_mut(*key).expect("key does not exist")
    }
}
#[allow(deprecated)]
impl<K: slotmap::Key, V> KeyedAccess<K> for slotmap::HopSlotMap<K, V> {
    type Value = V;
    fn keyed(&self, _index: usize, key: &K) -> &Self::Value {
        self.get(*key).expect("key does not exist.")
    }
    fn keyed_mut(&mut self, _index: usize, key: &K) -> &mut Self::Value {
        self.get_mut(*key).expect("key does not exist")
    }
}
impl<K: slotmap::Key, V> KeyedAccess<K> for slotmap::SecondaryMap<K, V> {
    type Value = V;
    fn keyed(&self, _index: usize, key: &K) -> &Self::Value {
        self.get(*key).expect("key does not exist.")
    }
    fn keyed_mut(&mut self, _index: usize, key: &K) -> &mut Self::Value {
        self.get_mut(*key).expect("key does not exist")
    }
}
impl<K: slotmap::Key, V> KeyedAccess<K> for slotmap::SparseSecondaryMap<K, V> {
    type Value = V;
    fn keyed(&self, _index: usize, key: &K) -> &Self::Value {
        self.get(*key).expect("key does not exist.")
    }
    fn keyed_mut(&mut self, _index: usize, key: &K) -> &mut Self::Value {
        self.get_mut(*key).expect("key does not exist")
    }
}

#[cfg(test)]
mod tests {
    use crate::{self as reactive_stores, tests::tick, AtKeyed, Store};
    use reactive_graph::{
        effect::Effect,
        traits::{GetUntracked, ReadUntracked, Set, Track, Write},
    };
    use slotmap::{DefaultKey, SlotMap};
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    #[derive(Debug, Default, Store)]
    struct TodoSlotMap {
        #[store(key: DefaultKey = |(k,_)| k)]
        todos: SlotMap<DefaultKey, Todo>,
    }
    impl TodoSlotMap {
        pub fn add(&mut self, label: impl ToString) -> DefaultKey {
            self.todos.insert_with_key(|key| Todo::new(key, label))
        }

        pub fn test_data() -> (Self, Vec<DefaultKey>) {
            let mut todos = TodoSlotMap::default();

            let ids = ["A", "B", "C"]
                .into_iter()
                .map(|label| todos.add(label))
                .collect();

            (todos, ids)
        }
    }

    #[derive(Debug, Store, Default, Clone, PartialEq, Eq)]
    struct Todo {
        id: DefaultKey,
        label: String,
    }

    impl Todo {
        pub fn new(id: DefaultKey, label: impl ToString) -> Self {
            Self {
                id,
                label: label.to_string(),
            }
        }
    }

    #[tokio::test]
    async fn slotmap_keyed_fields_can_be_moved() {
        _ = any_spawner::Executor::init_tokio();

        let (todos, ids) = TodoSlotMap::test_data();
        let store = Store::new(todos);
        assert_eq!(store.read_untracked().todos.len(), 3);

        // create an effect to read from each keyed field
        let a_count = Arc::new(AtomicUsize::new(0));
        let b_count = Arc::new(AtomicUsize::new(0));
        let c_count = Arc::new(AtomicUsize::new(0));

        let a = AtKeyed::new(store.todos(), ids[0]);
        let b = AtKeyed::new(store.todos(), ids[1]);
        let c = AtKeyed::new(store.todos(), ids[2]);

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
            vec![
                Todo::new(ids[0], "Foo"),
                Todo::new(ids[1], "B"),
                Todo::new(ids[2], "C"),
            ]
        );

        a.label().set("Bar".into());
        let after = store.todos().get_untracked();
        assert_eq!(
            after.values().cloned().collect::<Vec<_>>(),
            vec![
                Todo::new(ids[0], "Bar"),
                Todo::new(ids[1], "B"),
                Todo::new(ids[2], "C"),
            ]
        );
        tick().await;
        assert_eq!(a_count.load(Ordering::Relaxed), 3);
        assert_eq!(b_count.load(Ordering::Relaxed), 1);
        assert_eq!(c_count.load(Ordering::Relaxed), 1);

        // we can remove a key and add a new one
        store.todos().write().remove(ids[2]);
        let new_id = store
            .todos()
            .write()
            .insert_with_key(|key| Todo::new(key, "New"));
        let after = store.todos().get_untracked();
        assert_eq!(
            after.values().cloned().collect::<Vec<_>>(),
            vec![
                Todo::new(ids[0], "Bar"),
                Todo::new(ids[1], "B"),
                Todo::new(new_id, "New")
            ]
        );
        tick().await;
        assert_eq!(a_count.load(Ordering::Relaxed), 4);
        assert_eq!(b_count.load(Ordering::Relaxed), 2);
        assert_eq!(c_count.load(Ordering::Relaxed), 2);
    }
}
