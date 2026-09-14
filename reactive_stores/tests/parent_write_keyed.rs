//! Regression tests for <https://github.com/leptos-rs/leptos/issues/4475>.
//!
//! Writing to the *parent* of a keyed subfield (rather than to the keyed
//! subfield itself) used to leave the subfield's keys stale, so that a
//! previously-obtained `AtKeyed` handle would resolve to an out-of-bounds or
//! wrong index.

use reactive_graph::{
    effect::Effect,
    traits::{Get, Read, Set, Update, Write},
};
use reactive_stores::Store;
use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

async fn tick() {
    tokio::time::sleep(Duration::from_micros(10)).await;
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Store)]
pub struct MyStore {
    #[store(key: i32 = |row| row.id)]
    pub inners: Vec<MyInner>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Store)]
pub struct MyInner {
    pub id: i32,
    #[store(key: i32 = |row| row.id)]
    pub subs: Vec<MySubInner>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Store)]
pub struct MySubInner {
    pub id: i32,
    pub text: String,
}

fn sub(id: i32) -> MySubInner {
    MySubInner {
        id,
        text: format!("sub_{id}"),
    }
}

fn data(with_subs: bool) -> MyStore {
    let subs = |base: i32| {
        if with_subs {
            vec![sub(base + 1), sub(base + 2)]
        } else {
            vec![]
        }
    };
    MyStore {
        inners: vec![
            MyInner {
                id: 1,
                subs: subs(100),
            },
            MyInner {
                id: 2,
                subs: subs(200),
            },
        ],
    }
}

#[tokio::test]
async fn parent_write_removing_earlier_sibling() {
    _ = any_spawner::Executor::init_tokio();
    let store = Store::new(data(true));

    let inner = store.inners().into_iter().next().unwrap();
    let sub_102 = inner.subs().into_iter().nth(1).unwrap();
    assert_eq!(sub_102.text().get(), "sub_102");

    // remove sub 101 by writing to the parent, not to `subs()`
    inner.update(|inn| inn.subs.retain(|s| s.id != 101));

    // previously: "index out of bounds: the len is 1 but the index is 1"
    assert_eq!(sub_102.text().get(), "sub_102");
    assert_eq!(sub_102.id().get(), 102);
}

#[tokio::test]
async fn parent_write_reordering_keeps_handles_pointing_at_right_element() {
    _ = any_spawner::Executor::init_tokio();
    let store = Store::new(data(true));

    let inner = store.inners().into_iter().next().unwrap();
    let sub_101 = inner.subs().into_iter().next().unwrap();
    let sub_102 = inner.subs().into_iter().nth(1).unwrap();

    // same length, different order: a stale index would silently read the
    // wrong element rather than panic
    inner.update(|inn| inn.subs.reverse());

    assert_eq!(sub_101.text().get(), "sub_101");
    assert_eq!(sub_102.text().get(), "sub_102");
}

#[tokio::test]
async fn parent_write_then_write_through_keyed_handle() {
    _ = any_spawner::Executor::init_tokio();
    let store = Store::new(data(true));

    let inner = store.inners().into_iter().next().unwrap();
    let sub_102 = inner.subs().into_iter().nth(1).unwrap();

    inner.update(|inn| inn.subs.retain(|s| s.id != 101));

    // writing through the stale handle must also resolve to the right element
    sub_102.text().set("edited".to_string());
    assert_eq!(inner.subs().read()[0].text, "edited");
    assert_eq!(inner.subs().read().len(), 1);
}

#[tokio::test]
async fn parent_write_removing_the_item_itself() {
    _ = any_spawner::Executor::init_tokio();
    let store = Store::new(data(true));

    let inner = store.inners().into_iter().next().unwrap();
    let sub_102 = inner.subs().into_iter().nth(1).unwrap();

    inner.update(|inn| inn.subs.retain(|s| s.id != 102));

    // the item is gone: reading should fail cleanly rather than panic
    assert!(sub_102.text().try_read().is_none());
    assert!(sub_102.text().try_write().is_none());
}

#[tokio::test]
async fn grandparent_write_with_nested_keyed_fields() {
    _ = any_spawner::Executor::init_tokio();
    let store = Store::new(data(true));

    let inner_2 = store.inners().into_iter().nth(1).unwrap();
    let sub_202 = inner_2.subs().into_iter().nth(1).unwrap();

    // remove inner 1 and sub 201 in a single write to the root
    store.update(|s| {
        s.inners.retain(|i| i.id != 1);
        s.inners[0].subs.retain(|s| s.id != 201);
    });

    assert_eq!(inner_2.id().get(), 2);
    assert_eq!(sub_202.text().get(), "sub_202");
}

#[tokio::test]
async fn parent_write_under_effects() {
    _ = any_spawner::Executor::init_tokio();
    let store = Store::new(data(true));
    let inner = store.inners().into_iter().next().unwrap();

    let seen = Arc::new(Mutex::new(Vec::<String>::new()));

    // "outer For": tracks subs and creates a child effect per item
    Effect::new_sync({
        let seen = seen.clone();
        move |_| {
            for sub in inner.subs().into_iter() {
                let seen = seen.clone();
                Effect::new_sync(move |_| {
                    seen.lock().unwrap().push(sub.text().get());
                });
            }
        }
    });
    tick().await;
    tick().await;

    inner.update(|inn| inn.subs.retain(|s| s.id != 101));
    tick().await;
    tick().await;

    let seen = seen.lock().unwrap();
    assert!(seen.iter().all(|s| s == "sub_101" || s == "sub_102"));
    assert!(seen.iter().filter(|s| *s == "sub_102").count() >= 2);
}

// The second scenario from the issue: replacing the whole store with the same
// inner ids but different subs must notify readers of the nested keyed field.
#[tokio::test]
async fn parent_set_notifies_nested_keyed_field() {
    _ = any_spawner::Executor::init_tokio();
    let store = Store::new(data(false));
    let inner = store.inners().into_iter().next().unwrap();

    let count = Arc::new(AtomicUsize::new(0));
    let last_len = Arc::new(AtomicUsize::new(999));
    Effect::new_sync({
        let count = count.clone();
        let last_len = last_len.clone();
        move |_| {
            let n = inner.subs().into_iter().count();
            last_len.store(n, Ordering::Relaxed);
            count.fetch_add(1, Ordering::Relaxed);
        }
    });
    tick().await;
    tick().await;
    assert_eq!(count.load(Ordering::Relaxed), 1);
    assert_eq!(last_len.load(Ordering::Relaxed), 0);

    store.set(data(true));
    tick().await;
    tick().await;
    assert_eq!(count.load(Ordering::Relaxed), 2);
    assert_eq!(last_len.load(Ordering::Relaxed), 2);
}

#[derive(Clone, Debug, Default, Store)]
pub struct MapStore {
    #[store(key: String = |(k, _)| k.clone())]
    pub items: HashMap<String, i32>,
}

#[tokio::test]
async fn parent_write_on_map_keyed_field() {
    _ = any_spawner::Executor::init_tokio();
    let store = Store::new(MapStore {
        items: HashMap::from([
            ("a".to_string(), 1),
            ("b".to_string(), 2),
            ("c".to_string(), 3),
        ]),
    });

    let b = store.items().at_key("b".to_string());
    assert_eq!(b.get(), 2);

    store.update(|s| {
        s.items.remove("a");
        s.items.insert("d".to_string(), 4);
    });
    assert_eq!(b.get(), 2);

    store.update(|s| {
        s.items.remove("b");
    });
    assert!(b.try_read().is_none());

    *store.write() = MapStore {
        items: HashMap::from([("b".to_string(), 20)]),
    };
    assert_eq!(b.get(), 20);
}
