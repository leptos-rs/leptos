use reactive_graph::{
    effect::Effect,
    traits::{Get, GetUntracked, Set},
};
use reactive_stores::{Patch, PatchField, Store, StorePath};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Debug, Clone, PartialEq, Store, Patch)]
struct Item {
    id: usize,
    value: i32,
}

fn items(values: &[(usize, i32)]) -> Vec<Item> {
    values
        .iter()
        .map(|&(id, value)| Item { id, value })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Store, Patch)]
struct TextItem {
    id: String,
    value: i32,
}

#[derive(Debug, Clone, PartialEq, Store, Patch)]
enum State {
    Idle,
    Named {
        marker: i32,
        #[store(key: usize = |item: &Item| item.id)]
        items: Vec<Item>,
    },
    Tuple(i32, #[store(key: usize = |item: &Item| item.id)] Vec<Item>),
    Text {
        marker: i32,
        #[store(key: String = |item: &TextItem| item.id.clone())]
        items: Vec<TextItem>,
    },
}

fn named(values: &[(usize, i32)]) -> State {
    State::Named {
        marker: 7,
        items: items(values),
    }
}

fn tuple(values: &[(usize, i32)]) -> State {
    State::Tuple(7, items(values))
}

async fn tick() {
    tokio::time::sleep(std::time::Duration::from_millis(1)).await;
}

fn observe<T>(
    read: impl Fn() -> T + Send + Sync + 'static,
) -> Arc<AtomicUsize> {
    let count = Arc::new(AtomicUsize::new(0));
    Effect::new_sync({
        let count = Arc::clone(&count);
        move |_| {
            _ = read();
            count.fetch_add(1, Ordering::Relaxed);
        }
    });
    count
}

macro_rules! keyed_variant_tests {
    ($module:ident, $make:ident, $accessor:ident) => {
        mod $module {
            use super::*;

            #[test]
            fn first_whole_patch_updates_retained_items_without_accessor_or_iteration() {
                let store = Store::new($make(&[(10, 1), (20, 2)]));
                store.patch($make(&[(20, 22), (30, 3), (10, 11)]));
                assert_eq!(store.get_untracked(), $make(&[(20, 22), (30, 3), (10, 11)]));
                let collection = store.$accessor().unwrap();
                assert_eq!(collection.at_key(10).get_untracked(), items(&[(10, 11)])[0]);
                assert_eq!(collection.at_key(20).get_untracked(), items(&[(20, 22)])[0]);
                assert_eq!(collection.at_key(30).get_untracked(), items(&[(30, 3)])[0]);
            }

            #[test]
            fn saved_handles_follow_whole_enum_reorder_add_and_remove() {
                let store = Store::new($make(&[(10, 1), (20, 2), (30, 3)]));
                let collection = store.$accessor().unwrap();
                let ten = collection.at_key(10);
                let thirty = collection.at_key(30);
                store.patch($make(&[(30, 33), (40, 4), (10, 11)]));
                assert_eq!(ten.get_untracked(), items(&[(10, 11)])[0]);
                assert_eq!(thirty.get_untracked(), items(&[(30, 33)])[0]);
                let forty = collection.at_key(40);
                assert_eq!(forty.get_untracked(), items(&[(40, 4)])[0]);
                store.patch($make(&[(40, 44), (10, 111)]));
                assert_eq!(ten.get_untracked(), items(&[(10, 111)])[0]);
                assert_eq!(forty.get_untracked(), items(&[(40, 44)])[0]);
                assert_eq!(store.get_untracked(), $make(&[(40, 44), (10, 111)]));
            }

            #[test]
            fn reacquired_handles_read_current_items_after_variant_round_trip() {
                let store = Store::new($make(&[(10, 1), (20, 2), (30, 3)]));
                assert_eq!(store.$accessor().unwrap().at_key(30).value().get_untracked(), 3);
                store.patch(State::Idle);
                store.patch($make(&[(30, 33), (10, 11)]));
                let collection = store.$accessor().unwrap();
                let ten = collection.at_key(10);
                let thirty = collection.at_key(30);
                assert_eq!(thirty.get_untracked(), items(&[(30, 33)])[0]);
                assert_eq!(ten.get_untracked(), items(&[(10, 11)])[0]);
            }

            #[test]
            fn accessor_refreshes_keys_after_set_variant_round_trip() {
                let store = Store::new($make(&[(10, 1), (20, 2), (30, 3)]));
                assert_eq!(store.$accessor().unwrap().at_key(30).value().get_untracked(), 3);
                store.set(State::Idle);
                store.set($make(&[(30, 33), (40, 4)]));
                let collection = store.$accessor().unwrap();
                assert_eq!(collection.at_key(30).get_untracked(), items(&[(30, 33)])[0]);
                assert_eq!(collection.at_key(40).get_untracked(), items(&[(40, 4)])[0]);
            }

            #[tokio::test]
            async fn direct_accessor_patch_preserves_keyed_notifications_and_handles() {
                _ = any_spawner::Executor::init_tokio();
                let store = Store::new($make(&[(10, 1), (20, 2), (30, 3)]));
                let collection = store.$accessor().unwrap();
                let ten = collection.at_key(10);
                let twenty = collection.at_key(20);
                let ten_runs = observe(move || ten.value().get());
                let twenty_runs = observe(move || twenty.value().get());
                tick().await;
                collection.patch(items(&[(30, 3), (20, 2), (10, 1)]));
                tick().await;
                assert_eq!(ten_runs.load(Ordering::Relaxed), 1);
                assert_eq!(twenty_runs.load(Ordering::Relaxed), 1);
                assert_eq!(ten.get_untracked(), items(&[(10, 1)])[0]);
                collection.patch(items(&[(40, 4), (10, 11), (20, 2)]));
                tick().await;
                assert_eq!(ten_runs.load(Ordering::Relaxed), 2);
                assert_eq!(twenty_runs.load(Ordering::Relaxed), 1);
                assert_eq!(ten.get_untracked(), items(&[(10, 11)])[0]);
                assert_eq!(twenty.get_untracked(), items(&[(20, 2)])[0]);
                assert_eq!(collection.at_key(40).get_untracked(), items(&[(40, 4)])[0]);
                assert_eq!(store.get_untracked(), $make(&[(40, 4), (10, 11), (20, 2)]));
            }

            #[tokio::test]
            async fn stable_order_whole_patch_notifies_changed_item_not_sibling() {
                _ = any_spawner::Executor::init_tokio();
                let store = Store::new($make(&[(10, 1), (20, 2)]));
                let collection = store.$accessor().unwrap();
                let ten = collection.at_key(10);
                let twenty = collection.at_key(20);
                let ten_runs = observe(move || ten.value().get());
                let twenty_runs = observe(move || twenty.value().get());
                tick().await;
                store.patch($make(&[(10, 11), (20, 2)]));
                tick().await;
                assert_eq!(ten_runs.load(Ordering::Relaxed), 2);
                assert_eq!(twenty_runs.load(Ordering::Relaxed), 1);
                assert_eq!(ten.value().get_untracked(), 11);
                store.patch($make(&[(10, 11), (20, 2)]));
                tick().await;
                assert_eq!(ten_runs.load(Ordering::Relaxed), 2);
                assert_eq!(twenty_runs.load(Ordering::Relaxed), 1);
            }

            #[tokio::test]
            async fn predicates_ignore_direct_and_whole_collection_patches() {
                _ = any_spawner::Executor::init_tokio();
                let store = Store::new($make(&[(10, 1), (20, 2)]));
                let runs = [
                    observe(move || store.idle()),
                    observe(move || store.named()),
                    observe(move || store.tuple()),
                    observe(move || store.text()),
                ];
                tick().await;
                store.$accessor().unwrap().patch(items(&[(20, 22), (30, 3)]));
                tick().await;
                for count in &runs {
                    assert_eq!(count.load(Ordering::Relaxed), 1);
                }
                store.patch($make(&[(30, 33), (40, 4)]));
                tick().await;
                for count in &runs {
                    assert_eq!(count.load(Ordering::Relaxed), 1);
                }
                store.patch(State::Idle);
                tick().await;
                for count in &runs {
                    assert_eq!(count.load(Ordering::Relaxed), 2);
                }
            }

            #[test]
            fn patch_field_without_keys_updates_values() {
                let mut old = $make(&[(10, 1), (20, 2)]);
                let path: StorePath = [7usize, 9].into_iter().map(Into::into).collect();
                let mut notifications = Vec::new();
                old.patch_field(
                    $make(&[(20, 22), (30, 3), (10, 11)]),
                    &path,
                    &mut |path| notifications.push(path.clone()),
                    None,
                );
                assert_eq!(old, $make(&[(20, 22), (30, 3), (10, 11)]));
                assert!(!notifications.is_empty());
                old.patch_field($make(&[]), &path, &mut |_| {}, None);
                assert_eq!(old, $make(&[]));
            }
        }
    };
}

keyed_variant_tests!(named_field, named, named_items);
keyed_variant_tests!(tuple_field, tuple, tuple_1);

#[derive(Store, Patch)]
enum CustomPatchState {
    Ready {
        marker: i32,
        #[store(key: usize = |item: &Item| item.id)]
        #[patch(|this, new| *this = new)]
        items: Vec<Item>,
    },
}

#[test]
fn custom_keyed_patch_refreshes_saved_handles_after_reorder() {
    let make = |values: &[(usize, i32)]| CustomPatchState::Ready {
        marker: 7,
        items: items(values),
    };
    let store = Store::new(make(&[(10, 1), (20, 2), (30, 3)]));
    let collection = store.ready_items().unwrap();
    let ten = collection.at_key(10);
    let thirty = collection.at_key(30);
    store.patch(make(&[(30, 33), (40, 4), (10, 11)]));
    assert_eq!(thirty.get_untracked(), items(&[(30, 33)])[0]);
    assert_eq!(ten.get_untracked(), items(&[(10, 11)])[0]);
    let forty = collection.at_key(40);
    assert_eq!(forty.get_untracked(), items(&[(40, 4)])[0]);
    store.patch(make(&[(10, 111), (40, 44)]));
    assert_eq!(ten.get_untracked(), items(&[(10, 111)])[0]);
    assert_eq!(forty.get_untracked(), items(&[(40, 44)])[0]);
}

#[derive(Debug, Clone, PartialEq, Store, Patch)]
struct Group {
    id: usize,
    state: State,
}

#[derive(Store, Patch)]
struct Groups {
    marker: i32,
    #[store(key: usize = |group: &Group| group.id)]
    groups: Vec<Group>,
}

fn group(id: usize, values: &[(usize, i32)]) -> Group {
    Group {
        id,
        state: named(values),
    }
}

#[tokio::test]
async fn nested_key_maps_follow_outer_keys_after_outer_reorder() {
    _ = any_spawner::Executor::init_tokio();
    let store = Store::new(Groups {
        marker: 7,
        groups: vec![
            group(100, &[(10, 1), (20, 2), (30, 3)]),
            group(200, &[(30, 103), (10, 101), (20, 102)]),
        ],
    });
    let groups = store.groups();
    let first = groups.at_key(100).state().named_items().unwrap();
    let second = groups.at_key(200).state().named_items().unwrap();
    let first_ten = first.at_key(10).value();
    let first_thirty = first.at_key(30).value();
    let second_ten = second.at_key(10).value();
    let first_seen = Arc::new(AtomicUsize::new(0));
    let second_seen = Arc::new(AtomicUsize::new(0));
    let first_runs = observe({
        let seen = Arc::clone(&first_seen);
        move || seen.store(first_ten.get() as usize, Ordering::Relaxed)
    });
    let second_runs = observe({
        let seen = Arc::clone(&second_seen);
        move || seen.store(second_ten.get() as usize, Ordering::Relaxed)
    });
    tick().await;
    assert_eq!(first_seen.load(Ordering::Relaxed), 1);
    assert_eq!(second_seen.load(Ordering::Relaxed), 101);

    // Move the outer items before patching their children, so their positional
    // paths no longer agree with the paths used to register the nested keys.
    groups.patch(vec![
        group(200, &[(30, 103), (10, 101), (20, 102)]),
        group(100, &[(10, 1), (20, 2), (30, 3)]),
    ]);
    tick().await;
    assert_eq!(first_ten.get_untracked(), 1);
    assert_eq!(second_ten.get_untracked(), 101);
    let first_before = first_runs.load(Ordering::Relaxed);
    let second_before = second_runs.load(Ordering::Relaxed);

    store.patch(Groups {
        marker: 7,
        groups: vec![
            group(200, &[(10, 111), (40, 104)]),
            group(100, &[(30, 33), (40, 4), (10, 11)]),
        ],
    });
    // No new enum accessor or collection iteration may repair these maps.
    assert_eq!(first_ten.get_untracked(), 11);
    assert_eq!(first_thirty.get_untracked(), 33);
    assert_eq!(second_ten.get_untracked(), 111);
    assert_eq!(first.at_key(40).value().get_untracked(), 4);
    assert_eq!(second.at_key(40).value().get_untracked(), 104);
    tick().await;
    assert!(first_runs.load(Ordering::Relaxed) > first_before);
    assert!(second_runs.load(Ordering::Relaxed) > second_before);
    assert_eq!(first_seen.load(Ordering::Relaxed), 11);
    assert_eq!(second_seen.load(Ordering::Relaxed), 111);

    first.patch(items(&[(10, 12), (30, 34), (40, 4)]));
    first_ten.patch(13);
    assert_eq!(first_ten.get_untracked(), 13);
    assert_eq!(first_thirty.get_untracked(), 34);
    assert_eq!(second_ten.get_untracked(), 111);
    tick().await;
    assert_eq!(first_seen.load(Ordering::Relaxed), 13);
    assert_eq!(second_seen.load(Ordering::Relaxed), 111);
}

#[test]
fn variants_with_different_key_types_share_local_paths_and_refresh_on_return() {
    use reactive_stores::StoreField;

    let store = Store::new(named(&[(10, 1), (20, 2)]));
    let named_path: StorePath =
        store.named_items().unwrap().path().into_iter().collect();
    assert_eq!(
        store
            .named_items()
            .unwrap()
            .at_key(10)
            .value()
            .get_untracked(),
        1
    );
    store.patch(State::Text {
        marker: 7,
        items: vec![TextItem {
            id: "a".into(),
            value: 3,
        }],
    });
    assert!(store.named_items().is_none());
    let text = store.text_items().unwrap();
    let text_path: StorePath = text.path().into_iter().collect();
    assert_eq!(named_path, text_path);
    assert_eq!(text.at_key("a".into()).value().get_untracked(), 3);
    store.patch(tuple(&[(50, 5)]));
    let tuple = store.tuple_1().unwrap();
    let tuple_path: StorePath = tuple.path().into_iter().collect();
    assert_eq!(tuple_path, named_path);
    assert_eq!(tuple_path, text_path);
    assert_eq!(tuple.at_key(50).value().get_untracked(), 5);
    store.patch(named(&[(30, 33), (10, 11)]));
    let named = store.named_items().unwrap();
    assert_eq!(named.at_key(30).value().get_untracked(), 33);
    assert_eq!(named.at_key(10).value().get_untracked(), 11);
    store.patch(State::Text {
        marker: 7,
        items: vec![TextItem {
            id: "b".into(),
            value: 4,
        }],
    });
    assert_eq!(
        store
            .text_items()
            .unwrap()
            .at_key("b".into())
            .value()
            .get_untracked(),
        4
    );
}

#[derive(Store, Patch)]
struct Outer {
    unrelated: i32,
    state: State,
}

#[tokio::test]
async fn nested_enum_collection_uses_its_full_path() {
    _ = any_spawner::Executor::init_tokio();
    let store = Store::new(Outer {
        unrelated: 9,
        state: named(&[(10, 1), (20, 2)]),
    });
    // The first patch must initialize keys at the nested collection's path.
    store.patch(Outer {
        unrelated: 9,
        state: named(&[(20, 22), (10, 11)]),
    });
    let collection = store.state().named_items().unwrap();
    let ten = collection.at_key(10);
    let twenty = collection.at_key(20);
    assert_eq!(ten.value().get_untracked(), 11);
    assert_eq!(twenty.value().get_untracked(), 22);
    let ten_runs = observe(move || ten.value().get());
    let twenty_runs = observe(move || twenty.value().get());
    let unrelated_runs = observe(move || store.unrelated().get());
    tick().await;
    store.patch(Outer {
        unrelated: 9,
        state: named(&[(20, 22), (10, 111)]),
    });
    tick().await;
    assert_eq!(ten_runs.load(Ordering::Relaxed), 2);
    assert_eq!(twenty_runs.load(Ordering::Relaxed), 1);
    assert_eq!(unrelated_runs.load(Ordering::Relaxed), 1);
    assert_eq!(ten.value().get_untracked(), 111);
}

#[derive(Debug, Clone, PartialEq, Store, Patch)]
enum Entry {
    Named { id: usize, value: i32 },
    Tuple(usize, i32),
}

impl Entry {
    fn key(&self) -> usize {
        match self {
            Self::Named { id, .. } | Self::Tuple(id, _) => *id,
        }
    }
}

#[derive(Store, Patch)]
enum Entries {
    Ready(
        i32,
        #[store(key: usize = |entry: &Entry| entry.key())] Vec<Entry>,
    ),
}

#[test]
fn keyed_enum_elements_can_patch_payloads_and_switch_variants() {
    let store = Store::new(Entries::Ready(
        7,
        vec![Entry::Named { id: 10, value: 1 }, Entry::Tuple(20, 2)],
    ));
    let collection = store.ready_1().unwrap();
    let ten = collection.at_key(10);
    let twenty = collection.at_key(20);
    store.patch(Entries::Ready(
        7,
        vec![
            Entry::Tuple(20, 22),
            Entry::Tuple(10, 11),
            Entry::Named { id: 30, value: 3 },
        ],
    ));
    assert!(ten.named_value().is_none());
    assert_eq!(ten.tuple_1().unwrap().get_untracked(), 11);
    assert_eq!(twenty.tuple_1().unwrap().get_untracked(), 22);
    assert_eq!(
        collection.at_key(30).named_value().unwrap().get_untracked(),
        3
    );
    collection.patch(vec![Entry::Named { id: 10, value: 111 }]);
    assert_eq!(ten.named_value().unwrap().get_untracked(), 111);
}
