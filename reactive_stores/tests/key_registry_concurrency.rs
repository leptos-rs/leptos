use reactive_graph::traits::{GetUntracked, Set};
use reactive_stores::{
    ArcStore, KeyMap, Patch, Store, StoreField, StoreFieldTrigger, StorePath,
    StorePathSegment,
};
use std::{
    sync::{mpsc, Arc, Mutex},
    thread,
    time::Duration,
};

#[derive(Clone, Debug, PartialEq, Store, Patch)]
struct Item {
    id: usize,
    value: i32,
}

#[derive(Clone, Debug, PartialEq, Store)]
struct State {
    #[store(key: usize = |item| item.id)]
    items: Vec<Item>,
}

fn state(values: &[(usize, i32)]) -> State {
    State {
        items: values
            .iter()
            .map(|&(id, value)| Item { id, value })
            .collect(),
    }
}

fn check_delayed_initializer(seed_registry: bool, publish_new_mapping: bool) {
    let store = ArcStore::new(state(&[(10, 1), (20, 2)]));
    if seed_registry {
        assert_eq!(
            store.clone().items().at_key(10).get_untracked(),
            Item { id: 10, value: 1 }
        );
        // Leave an existing entry stale so the delayed call must initialize.
        store.set(state(&[(10, 1), (20, 2)]));
    }

    let keys = store.keys().unwrap();
    let path = store.clone().items().path().into_iter().collect();
    let snapshot_store = store.clone();
    let (snapshot_tx, snapshot_rx) = mpsc::channel();
    let (resume_tx, resume_rx) = mpsc::channel();
    let timeout = Duration::from_secs(10);
    let delayed = thread::spawn(move || {
        keys.with_field_keys(
            path,
            |keys| {
                assert!(
                    publish_new_mapping,
                    "a superseded initializer must not access a stale entry"
                );
                (
                    (
                        keys.get(&10).map(|(_, index)| index),
                        keys.get(&20).map(|(_, index)| index),
                    ),
                    vec![],
                )
            },
            || {
                // Snapshot through the store, then release its value lock before
                // pausing. The public keyed read has no scheduling seam here.
                let snapshot = snapshot_store
                    .get_untracked()
                    .items
                    .into_iter()
                    .map(|item| item.id)
                    .collect::<Vec<_>>();
                snapshot_tx.send(snapshot.clone()).unwrap();
                resume_rx
                    .recv_timeout(timeout)
                    .expect("initializer was not released");
                snapshot
            },
        )
    });

    assert_eq!(
        snapshot_rx
            .recv_timeout(timeout)
            .expect("initializer did not snapshot the old keys"),
        vec![10, 20]
    );
    store.set(state(&[(20, 22), (10, 11)]));
    let ten = store.clone().items().at_key(10);
    let twenty = store.clone().items().at_key(20);
    if publish_new_mapping {
        // Publish the new generation while the old snapshot is still paused.
        assert_eq!(ten.get_untracked(), Item { id: 10, value: 11 });
        assert_eq!(twenty.get_untracked(), Item { id: 20, value: 22 });
    }

    resume_tx.send(()).unwrap();
    let result = delayed.join().expect("delayed initializer panicked");
    if publish_new_mapping {
        assert_eq!(result, Some((Some(1), Some(0))));
    } else {
        assert_eq!(result, None);
    }

    // Check reads before any write can invalidate and repair a poisoned mapping.
    assert_eq!(ten.get_untracked(), Item { id: 10, value: 11 });
    assert_eq!(twenty.get_untracked(), Item { id: 20, value: 22 });
    ten.value().set(12);
    assert_eq!(store.get_untracked(), state(&[(20, 22), (10, 12)]));
    twenty.value().set(23);
    assert_eq!(store.get_untracked(), state(&[(20, 23), (10, 12)]));
}

#[test]
fn delayed_initializer_reuses_newer_mapping() {
    check_delayed_initializer(false, true);
}

#[test]
fn superseded_snapshot_does_not_create_registry_entry() {
    check_delayed_initializer(false, false);
}

#[test]
fn superseded_snapshot_does_not_refresh_stale_registry_entry() {
    check_delayed_initializer(true, false);
}

#[test]
fn uninitialized_direct_keyed_patch_after_parent_set() {
    let store = ArcStore::new(state(&[(10, 1), (20, 2)]));
    let items = store.clone().items();
    store.set(state(&[(20, 22), (10, 11)]));

    // No keyed read or iteration has initialized the registry before patching.
    items.patch(state(&[(10, 111), (20, 222)]).items);

    assert_eq!(store.get_untracked(), state(&[(10, 111), (20, 222)]));
    assert_eq!(
        items.at_key(10).get_untracked(),
        Item { id: 10, value: 111 }
    );
    assert_eq!(
        items.at_key(20).get_untracked(),
        Item { id: 20, value: 222 }
    );
}

#[derive(Clone)]
struct ReplaceBeforeWrite {
    store: ArcStore<State>,
    replacement: Arc<Mutex<Option<State>>>,
}

impl StoreField for ReplaceBeforeWrite {
    type Value = State;
    type Reader = <ArcStore<State> as StoreField>::Reader;
    type Writer = <ArcStore<State> as StoreField>::Writer;

    fn get_trigger(&self, path: StorePath) -> StoreFieldTrigger {
        self.store.get_trigger(path)
    }

    fn get_trigger_unkeyed(&self, path: StorePath) -> StoreFieldTrigger {
        self.store.get_trigger_unkeyed(path)
    }

    fn path(&self) -> impl IntoIterator<Item = StorePathSegment> {
        self.store.path()
    }

    fn reader(&self) -> Option<Self::Reader> {
        self.store.reader()
    }

    fn writer(&self) -> Option<Self::Writer> {
        // This public-interface seam forces invalidation after any pre-lock
        // refresh but before the patch acquires the actual value writer.
        let replacement = self.replacement.lock().unwrap().take();
        if let Some(replacement) = replacement {
            self.store.set(replacement);
        }
        self.store.writer()
    }

    fn keys(&self) -> Option<KeyMap> {
        self.store.keys()
    }
}

#[test]
fn direct_keyed_patch_refreshes_after_writer_acquisition() {
    let store = ArcStore::new(state(&[(10, 1), (20, 2)]));
    let replacement = Arc::new(Mutex::new(Some(state(&[(20, 22), (10, 11)]))));
    let items = ReplaceBeforeWrite {
        store: store.clone(),
        replacement: Arc::clone(&replacement),
    }
    .items();

    items.patch(state(&[(10, 111), (20, 222)]).items);

    assert!(replacement.lock().unwrap().is_none());
    assert_eq!(store.get_untracked(), state(&[(10, 111), (20, 222)]));
    let ten = store.clone().items().at_key(10);
    let twenty = store.clone().items().at_key(20);
    assert_eq!(ten.get_untracked(), Item { id: 10, value: 111 });
    assert_eq!(twenty.get_untracked(), Item { id: 20, value: 222 });
    twenty.value().set(223);
    assert_eq!(store.get_untracked(), state(&[(10, 111), (20, 223)]));
}
