use reactive_graph::{
    effect::Effect,
    traits::{Get, GetUntracked, Set, UpdateUntracked, Write},
};
use reactive_stores::{ArcStore, Store};
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, PartialEq, Store)]
struct Item {
    id: usize,
    value: i32,
}

#[derive(Clone, Debug, PartialEq, Store)]
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

#[derive(Clone, Store)]
struct Group {
    id: usize,
    state: State,
}

#[derive(Store)]
struct Groups {
    #[store(key: usize = |group| group.id)]
    groups: Vec<Group>,
}

fn group(id: usize, values: &[(usize, i32)]) -> Group {
    Group {
        id,
        state: ready(values),
    }
}

async fn tick() {
    tokio::time::sleep(std::time::Duration::from_millis(1)).await;
}

fn observe(
    read: impl Fn() -> i32 + Send + Sync + 'static,
) -> Arc<Mutex<Vec<i32>>> {
    let values = Arc::new(Mutex::new(Vec::new()));
    Effect::new_sync({
        let values = Arc::clone(&values);
        move |_| values.lock().unwrap().push(read())
    });
    values
}

#[tokio::test]
async fn root_same_variant_set_preserves_retained_handle_and_observer() {
    _ = any_spawner::Executor::init_tokio();
    let store = Store::new(ready(&[(10, 1), (20, 2)]));
    let ten = store.ready_items().unwrap().at_key(10);
    let seen = observe(move || ten.value().get());
    tick().await;

    store.set(ready(&[(20, 22), (30, 33), (10, 11)]));
    assert_eq!(ten.get_untracked(), Item { id: 10, value: 11 });
    tick().await;
    assert_eq!(*seen.lock().unwrap(), vec![1, 11]);

    // A fresh handle must notify the observer attached through the saved handle.
    store.ready_items().unwrap().at_key(10).value().set(12);
    tick().await;
    assert_eq!(*seen.lock().unwrap(), vec![1, 11, 12]);
    ten.value().set(13);
    tick().await;
    assert_eq!(*seen.lock().unwrap(), vec![1, 11, 12, 13]);
    assert_eq!(
        store.get_untracked(),
        ready(&[(20, 22), (30, 33), (10, 13)])
    );
}

#[tokio::test]
async fn intermediate_parent_set_preserves_retained_handle_and_observer() {
    _ = any_spawner::Executor::init_tokio();
    let store = Store::new(group(100, &[(10, 1), (20, 2)]));
    let ten = store.state().ready_items().unwrap().at_key(10);
    let seen = observe(move || ten.value().get());
    tick().await;

    store.state().set(ready(&[(20, 22), (10, 11)]));
    assert_eq!(ten.get_untracked(), Item { id: 10, value: 11 });
    tick().await;
    assert_eq!(*seen.lock().unwrap(), vec![1, 11]);

    store
        .state()
        .ready_items()
        .unwrap()
        .at_key(10)
        .value()
        .set(12);
    tick().await;
    assert_eq!(*seen.lock().unwrap(), vec![1, 11, 12]);
    ten.value().set(13);
    tick().await;
    assert_eq!(*seen.lock().unwrap(), vec![1, 11, 12, 13]);
    assert_eq!(store.state().get_untracked(), ready(&[(20, 22), (10, 13)]));
}

async fn check_untracked_parent_replacement(use_write_guard: bool) {
    _ = any_spawner::Executor::init_tokio();
    let store = Store::new(group(100, &[(10, 1), (20, 2)]));
    let ten = store.state().ready_items().unwrap().at_key(10);
    let seen = observe(move || ten.value().get());
    tick().await;
    assert_eq!(*seen.lock().unwrap(), vec![1]);

    if use_write_guard {
        *store.state().write_untracked() = ready(&[(20, 22), (10, 11)]);
    } else {
        store.state().update_untracked(|state| {
            *state = ready(&[(20, 22), (10, 11)]);
        });
    }
    tick().await;
    assert_eq!(*seen.lock().unwrap(), vec![1]);
    assert_eq!(ten.get_untracked(), Item { id: 10, value: 11 });

    ten.value().set(12);
    tick().await;
    assert_eq!(*seen.lock().unwrap(), vec![1, 12]);
    assert_eq!(store.state().get_untracked(), ready(&[(20, 22), (10, 12)]));
}

#[tokio::test]
async fn parent_write_untracked_refreshes_keys_without_notifying() {
    check_untracked_parent_replacement(true).await;
}

#[tokio::test]
async fn parent_update_untracked_refreshes_keys_without_notifying() {
    check_untracked_parent_replacement(false).await;
}

#[test]
fn absent_key_resolves_new_data_after_reinsertion() {
    let store = ArcStore::new(ready(&[(10, 1), (20, 2)]));
    let ten = store.clone().ready_items().unwrap().at_key(10);
    assert_eq!(ten.get_untracked(), Item { id: 10, value: 1 });

    store.set(ready(&[(20, 22)]));
    assert_eq!(ten.try_get_untracked(), None);

    store.set(ready(&[(30, 33), (20, 222), (10, 111)]));
    assert_eq!(ten.try_get_untracked(), Some(Item { id: 10, value: 111 }));
    ten.value().set(112);
    assert_eq!(
        store.get_untracked(),
        ready(&[(30, 33), (20, 222), (10, 112)])
    );
}

#[test]
fn replacing_outer_key_gives_fresh_child_handle_new_identity_and_data() {
    let store = Store::new(Groups {
        groups: vec![group(100, &[(10, 1), (20, 2)])],
    });
    let old_child = store
        .groups()
        .at_key(100)
        .state()
        .ready_items()
        .unwrap()
        .at_key(10);
    assert_eq!(old_child.get_untracked(), Item { id: 10, value: 1 });

    store.set(Groups {
        groups: vec![group(200, &[(20, 222), (30, 333)])],
    });
    let new_parent = store.groups().at_key(200);
    assert_eq!(new_parent.id().get_untracked(), 200);
    let children = new_parent.state().ready_items().unwrap();
    assert_eq!(children.at_key(10).try_get_untracked(), None);
    let new_child = children.at_key(20);
    assert_eq!(new_child.get_untracked(), Item { id: 20, value: 222 });
    assert_eq!(
        children.at_key(30).get_untracked(),
        Item { id: 30, value: 333 }
    );

    new_child.value().set(223);
    assert_eq!(
        new_parent.state().get_untracked(),
        ready(&[(20, 223), (30, 333)])
    );
}
