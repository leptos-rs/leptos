use reactive_graph::traits::{GetUntracked, Set, Write};
use reactive_stores::{Patch, Store, StoreField};

#[derive(Clone, Debug, PartialEq, Store, Patch)]
struct Item {
    id: usize,
    value: i32,
}

#[derive(Clone, Store, Patch)]
enum State {
    Ready {
        #[store(key: usize = |item| item.id)]
        items: Vec<Item>,
    },
}

fn ready(values: &[(usize, i32)]) -> State {
    State::Ready {
        items: values
            .iter()
            .map(|&(id, value)| Item { id, value })
            .collect(),
    }
}

#[derive(Clone, Store, Patch)]
struct Group {
    id: usize,
    state: State,
}

#[derive(Store, Patch)]
struct Groups {
    #[store(key: usize = |group| group.id)]
    groups: Vec<Group>,
}

#[test]
fn unavailable_collection_is_not_cached_as_empty() {
    let store = Store::new(Groups {
        groups: vec![Group {
            id: 100,
            state: ready(&[(10, 1)]),
        }],
    });
    let parent = store.groups().at_key(100);
    let guard = store.write_untracked();
    let _ = parent.path().into_iter().collect::<Vec<_>>();
    drop(guard);
    assert!(parent.try_get_untracked().is_some());
}

#[test]
fn saved_handle_follows_its_key_after_parent_replacement() {
    let store = Store::new(ready(&[(10, 1), (20, 2)]));
    let ten = store.ready_items().unwrap().at_key(10);
    assert_eq!(ten.value().get_untracked(), 1);

    store.set(ready(&[(20, 22), (10, 11)]));
    assert_eq!(ten.value().get_untracked(), 11);

    // Writing through the same handle must not overwrite the new first item.
    store.set(ready(&[(10, 111), (20, 222)]));
    ten.value().set(112);
    let State::Ready { items } = store.get_untracked();
    assert_eq!(
        items,
        vec![Item { id: 10, value: 112 }, Item { id: 20, value: 222 }]
    );
}

#[test]
fn repeated_keyed_access_only_refreshes_after_a_write() {
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    #[derive(Store)]
    struct CountedItem {
        id: usize,
        calls: Arc<AtomicUsize>,
    }
    fn key(item: &CountedItem) -> usize {
        item.calls.fetch_add(1, Ordering::Relaxed);
        item.id
    }
    #[derive(Store)]
    enum CountedState {
        Ready(#[store(key: usize = |item| key(item))] Vec<CountedItem>),
    }
    let calls = Arc::new(AtomicUsize::new(0));
    let data = |ids: &[usize]| {
        CountedState::Ready(
            ids.iter()
                .map(|&id| CountedItem {
                    id,
                    calls: Arc::clone(&calls),
                })
                .collect(),
        )
    };
    let store = Store::new(data(&[10, 20]));
    assert_eq!(store.ready_0().unwrap().at_key(10).id().get_untracked(), 10);
    let initial = calls.load(Ordering::Relaxed);
    for _ in 0..3 {
        assert_eq!(
            store.ready_0().unwrap().at_key(10).id().get_untracked(),
            10
        );
    }
    assert_eq!(calls.load(Ordering::Relaxed), initial);

    store.set(data(&[20, 10]));
    assert_eq!(calls.load(Ordering::Relaxed), initial);
    for _ in 0..3 {
        assert_eq!(
            store.ready_0().unwrap().at_key(10).id().get_untracked(),
            10
        );
    }
    assert_eq!(calls.load(Ordering::Relaxed), initial + 2);
}

#[test]
fn lazy_nested_patch_does_not_require_collection_iteration() {
    let store = Store::new(Groups {
        groups: vec![Group {
            id: 100,
            state: ready(&[(10, 1)]),
        }],
    });
    store.groups().at_key(100).state().patch(ready(&[(10, 2)]));
    assert_eq!(
        store
            .groups()
            .at_key(100)
            .state()
            .ready_items()
            .unwrap()
            .at_key(10)
            .value()
            .get_untracked(),
        2
    );
}
