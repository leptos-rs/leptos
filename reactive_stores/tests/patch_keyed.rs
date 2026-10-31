//! Regression tests for `Patch` on a struct with a `#[store(key)]` field.
//!
//! The derive looked the keys up under the struct's path instead of the
//! field's, so patching panicked with "could not find key for index" once a
//! row changed. Rows that nothing had read through their keys kept their old
//! values, and the lookup left an empty key set behind for later reads.

use reactive_graph::{
    effect::Effect,
    owner::Owner,
    traits::{Get, GetUntracked},
};
use reactive_stores::{Patch, Store};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

async fn tick() {
    tokio::time::sleep(Duration::from_micros(10)).await;
}

#[derive(Debug, Clone, PartialEq, Store, Patch)]
struct Page {
    title: String,
    #[store(key: u32 = |row| row.id)]
    rows: Vec<Line>,
}

#[derive(Debug, Clone, PartialEq, Store, Patch)]
struct Line {
    id: u32,
    text: String,
}

fn page(first: &str) -> Page {
    Page {
        title: "t".into(),
        rows: vec![
            Line {
                id: 1,
                text: first.into(),
            },
            Line {
                id: 2,
                text: "same".into(),
            },
        ],
    }
}

fn texts(store: Store<Page>) -> Vec<String> {
    store
        .rows()
        .into_iter()
        .map(|row| row.text().get_untracked())
        .collect()
}

#[test]
fn patch_updates_keyed_rows_that_were_read() {
    let owner = Owner::new();
    owner.set();
    let store = Store::new(page("old"));
    assert_eq!(texts(store), ["old", "same"]);

    // previously: "could not find key for index (StorePath([]), 1)"
    store.patch(page("new"));
    assert_eq!(texts(store), ["new", "same"]);
}

#[test]
fn patch_updates_keyed_rows_never_read() {
    let owner = Owner::new();
    owner.set();
    let store = Store::new(page("old"));

    store.patch(page("new"));
    assert_eq!(store.get_untracked(), page("new"));

    // rows read only now follow the next patch
    assert_eq!(texts(store), ["new", "same"]);
    store.patch(page("newer"));
    assert_eq!(texts(store), ["newer", "same"]);
}

#[tokio::test]
async fn patch_runs_only_the_readers_of_the_row_that_changed() {
    _ = any_spawner::Executor::init_tokio();
    let store = Store::new(page("old"));
    let seen = Arc::new(Mutex::new(Vec::new()));
    for row in store.rows().into_iter() {
        let seen = seen.clone();
        Effect::new_sync(move |_| {
            seen.lock().unwrap().push(row.text().get());
        });
    }
    tick().await;
    tick().await;
    assert_eq!(*seen.lock().unwrap(), ["old", "same"]);

    store.patch(page("new"));
    tick().await;
    tick().await;
    assert_eq!(*seen.lock().unwrap(), ["old", "same", "new"]);
}
