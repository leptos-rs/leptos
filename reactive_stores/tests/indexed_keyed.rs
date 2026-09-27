use reactive_graph::{
    effect::Effect,
    traits::{Get, GetUntracked, IsDisposed, ReadUntracked, Set, Write},
};
use reactive_stores::{
    ArcField, DerefField, Field, OptionStoreExt, Patch, Store, StoreField,
    StoreFieldIterator,
};
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, PartialEq, Store, Patch)]
struct Item {
    id: usize,
    value: i32,
}

#[derive(Clone, Debug, PartialEq, Store, Patch)]
struct Numbers {
    #[store(key: usize = |item| item.id)]
    items: Vec<Item>,
}

#[derive(Clone, Debug, PartialEq, Store, Patch)]
struct Text {
    #[store(key: String = |item| item.clone())]
    items: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Store, Patch)]
enum State {
    Numbers(Numbers),
    Text(Text),
    Empty,
}

#[derive(Clone, Debug, PartialEq, Store, Patch)]
struct Row {
    id: usize,
    state: State,
}

#[derive(Clone, Debug, PartialEq, Store, Patch)]
struct Rows {
    #[store(key: usize = |row| row.id)]
    rows: Vec<Row>,
}

fn rows() -> Rows {
    Rows {
        rows: vec![
            Row {
                id: 1,
                state: State::Numbers(Numbers {
                    items: vec![Item { id: 10, value: 1 }],
                }),
            },
            Row {
                id: 2,
                state: State::Text(Text {
                    items: vec!["x".into()],
                }),
            },
        ],
    }
}

#[tokio::test]
async fn ordinary_indexed_siblings_stay_independent_through_erasure() {
    _ = any_spawner::Executor::init_tokio();
    let store =
        Store::new(vec![Item { id: 1, value: 1 }, Item { id: 2, value: 2 }]);
    let arc: ArcField<Vec<Item>> = store.into();
    let field: Field<Vec<Item>> = store.into();
    let first = observe(move || arc.clone().at_unkeyed(0).value().get());
    let second = observe(move || field.at_unkeyed(1).value().get());
    tick().await;
    assert_eq!(*first.lock().unwrap(), [1]);
    assert_eq!(*second.lock().unwrap(), [2]);

    store.at_unkeyed(0).value().set(3);
    tick().await;
    assert_eq!(*first.lock().unwrap(), [1, 3]);
    assert_eq!(*second.lock().unwrap(), [2]);

    store.set(vec![Item { id: 2, value: 2 }, Item { id: 1, value: 3 }]);
    tick().await;
    assert_eq!(*first.lock().unwrap(), [1, 3, 2]);
    assert_eq!(*second.lock().unwrap(), [2, 3]);
}

#[test]
fn indexed_and_keyed_enum_registries_agree_after_reorder() {
    let store = Store::new(rows());
    let indexed_row = store.rows().at_unkeyed(0);
    let keyed_row = store.rows().at_key(1);
    assert_eq!(
        keyed_row
            .state()
            .numbers_0()
            .unwrap()
            .items()
            .at_key(10)
            .value()
            .get_untracked(),
        1,
    );
    assert_eq!(
        store
            .rows()
            .at_key(2)
            .state()
            .text_0()
            .unwrap()
            .items()
            .at_key("x".into())
            .get_untracked(),
        "x",
    );

    let mut replacement = rows().rows;
    replacement.reverse();
    store.rows().set(replacement);
    assert_eq!(indexed_row.id().get_untracked(), 2);
    assert_eq!(keyed_row.id().get_untracked(), 1);
    let indexed = indexed_row.state().text_0().unwrap().items();
    assert_eq!(indexed.at_key("x".into()).get_untracked(), "x");
    assert_eq!(
        keyed_row
            .state()
            .numbers_0()
            .unwrap()
            .items()
            .at_key(10)
            .value()
            .get_untracked(),
        1,
    );
    assert_eq!(indexed.at_key("x".into()).get_untracked(), "x");
}

fn observe<T: Send + 'static>(
    read: impl Fn() -> T + Send + Sync + 'static,
) -> Arc<Mutex<Vec<T>>> {
    let seen = Arc::new(Mutex::new(Vec::new()));
    Effect::new_sync({
        let seen = Arc::clone(&seen);
        move |_| seen.lock().unwrap().push(read())
    });
    seen
}

async fn tick() {
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
}

#[derive(Clone, Debug, PartialEq, Store, Patch)]
enum PredicateState {
    Ready { value: i32 },
    Empty,
}

#[derive(Clone, Debug, PartialEq, Store, Patch)]
struct PredicateRow {
    id: usize,
    state: PredicateState,
    other: i32,
}

#[derive(Clone, Debug, PartialEq, Store, Patch)]
struct PredicateRows {
    #[store(key: usize = |row| row.id)]
    rows: Vec<PredicateRow>,
}

#[tokio::test]
async fn indexed_enum_predicates_ignore_payload_writes_but_follow_reorders() {
    _ = any_spawner::Executor::init_tokio();
    let store = Store::new(PredicateRows {
        rows: vec![
            PredicateRow {
                id: 1,
                state: PredicateState::Ready { value: 1 },
                other: 0,
            },
            PredicateRow {
                id: 2,
                state: PredicateState::Empty,
                other: 0,
            },
        ],
    });
    let ready = observe(move || store.rows().at_unkeyed(0).state().ready());
    let erased: Field<PredicateRow> = store.rows().at_unkeyed(0).into();
    let has_value = observe(move || erased.state().ready_value().is_some());
    tick().await;
    assert_eq!(*ready.lock().unwrap(), [true]);
    assert_eq!(*has_value.lock().unwrap(), [true]);

    store.rows().at_key(2).other().set(1);
    tick().await;
    assert_eq!(*ready.lock().unwrap(), [true]);
    assert_eq!(*has_value.lock().unwrap(), [true]);

    store.rows().at_key(1).other().set(1);
    store.rows().at_key(1).state().ready_value().unwrap().set(2);
    tick().await;
    assert_eq!(*ready.lock().unwrap(), [true]);
    assert_eq!(*has_value.lock().unwrap(), [true]);

    let mut replacement = store.rows().get_untracked();
    replacement[0].state = PredicateState::Ready { value: 3 };
    replacement[1].other = 2;
    store.rows().patch(replacement.clone());
    tick().await;
    assert_eq!(*ready.lock().unwrap(), [true]);
    assert_eq!(*has_value.lock().unwrap(), [true]);

    replacement.reverse();
    store.rows().patch(replacement);
    tick().await;
    assert_eq!(*ready.lock().unwrap(), [true, false]);
    assert_eq!(*has_value.lock().unwrap(), [true, false]);

    store.rows().at_key(1).other().set(2);
    tick().await;
    assert_eq!(*ready.lock().unwrap(), [true, false]);
    assert_eq!(*has_value.lock().unwrap(), [true, false]);

    store
        .rows()
        .at_key(2)
        .state()
        .set(PredicateState::Ready { value: 4 });
    tick().await;
    assert_eq!(*ready.lock().unwrap(), [true, false, true]);
    assert_eq!(*has_value.lock().unwrap(), [true, false, true]);
}

#[tokio::test]
async fn pure_reorder_wakes_index_and_descendants_but_not_keyed_items() {
    _ = any_spawner::Executor::init_tokio();
    let store = Store::new(rows());
    let indexed = store.rows().at_unkeyed(0);
    let whole = observe(move || indexed.get().id);
    let descendant = observe(move || indexed.id().get());
    let variant = observe(move || indexed.state().text_0().is_some());
    let keyed = observe(move || store.rows().at_key(1).id().get());
    tick().await;
    assert_eq!(*whole.lock().unwrap(), [1]);
    assert_eq!(*descendant.lock().unwrap(), [1]);
    assert_eq!(*variant.lock().unwrap(), [false]);
    assert_eq!(*keyed.lock().unwrap(), [1]);

    let mut replacement = rows().rows;
    replacement.reverse();
    store.rows().patch(replacement);
    tick().await;
    assert_eq!(*whole.lock().unwrap(), [1, 2]);
    assert_eq!(*descendant.lock().unwrap(), [1, 2]);
    assert_eq!(*variant.lock().unwrap(), [false, true]);
    assert_eq!(*keyed.lock().unwrap(), [1]);

    store.rows().patch(rows().rows);
    tick().await;
    assert_eq!(*whole.lock().unwrap(), [1, 2, 1]);
    assert_eq!(*descendant.lock().unwrap(), [1, 2, 1]);
    assert_eq!(*variant.lock().unwrap(), [false, true, false]);
    assert_eq!(*keyed.lock().unwrap(), [1]);
}

#[tokio::test]
async fn erased_index_ancestors_keep_reorder_dependencies() {
    _ = any_spawner::Executor::init_tokio();
    let store = Store::new(rows());
    let arc: ArcField<Row> = store.rows().at_unkeyed(0).into();
    let field: Field<Row> = store.rows().at_unkeyed(0).into();
    let arc_seen = observe(move || arc.clone().state().text());
    let field_seen = observe(move || field.id().get());
    tick().await;
    assert_eq!(*arc_seen.lock().unwrap(), [false]);
    assert_eq!(*field_seen.lock().unwrap(), [1]);

    let mut replacement = rows().rows;
    replacement.reverse();
    store.rows().patch(replacement);
    tick().await;
    assert_eq!(*arc_seen.lock().unwrap(), [false, true]);
    assert_eq!(*field_seen.lock().unwrap(), [1, 2]);
}

#[test]
fn indexed_writes_prepare_cold_and_stale_metadata_under_the_value_lock() {
    for warm in [false, true] {
        let store = Store::new(rows());
        if warm {
            assert_eq!(store.rows().at_key(1).id().get_untracked(), 1);
        }
        store.write_untracked().rows.reverse();
        let indexed: ArcField<Row> = store.rows().at_unkeyed(0).into();
        indexed.clone().state().set(State::Empty);
        assert!(store.rows().at_key(2).state().empty());
        assert!(store.rows().at_key(1).state().numbers());
        indexed.state().set(State::Text(Text {
            items: vec!["y".into()],
        }));
        assert_eq!(
            store
                .rows()
                .at_key(2)
                .state()
                .text_0()
                .unwrap()
                .items()
                .at_key("y".into())
                .get_untracked(),
            "y",
        );
    }
}

#[test]
fn missing_indices_return_none_before_and_after_shrinking() {
    let store = Store::new(rows());
    let missing = store.rows().at_unkeyed(4);
    assert!(missing.try_read_untracked().is_none());
    assert!(missing.try_write().is_none());
    let present = store.rows().at_unkeyed(0);
    assert_eq!(present.id().get_untracked(), 1);
    store.rows().write_untracked().clear();
    assert!(present.try_read_untracked().is_none());
    assert!(present.try_write().is_none());
    assert!(store.rows().get_untracked().is_empty());
}

#[test]
fn indexed_metadata_refreshes_after_each_mutation_route() {
    for mutation in 0..4 {
        for warm in [false, true] {
            let store = Store::new(rows());
            let indexed = store.rows().at_unkeyed(0);
            if warm {
                assert_eq!(indexed.id().get_untracked(), 1);
            }
            let mut replacement = rows();
            replacement.rows.reverse();
            match mutation {
                0 => store.set(replacement),
                1 => *store.write() = replacement,
                2 => *store.write_untracked() = replacement,
                _ => store.rows().patch(replacement.rows),
            }
            let text = indexed.state().text_0().unwrap().items();
            assert_eq!(text.at_key("x".into()).get_untracked(), "x");
            assert_eq!(
                store
                    .rows()
                    .at_key(1)
                    .state()
                    .numbers_0()
                    .unwrap()
                    .items()
                    .at_key(10)
                    .value()
                    .get_untracked(),
                1,
            );
            assert_eq!(text.at_key("x".into()).get_untracked(), "x");
        }
    }
}

#[derive(Clone, Debug, PartialEq, Store, Patch)]
struct Group {
    id: usize,
    data: Rows,
}

#[derive(Clone, Debug, PartialEq, Store, Patch)]
struct Groups {
    #[store(key: usize = |group| group.id)]
    groups: Vec<Group>,
}

#[test]
fn indexed_segments_do_not_translate_keyed_ancestors_twice() {
    let store = Store::new(Groups {
        groups: vec![
            Group {
                id: 10,
                data: rows(),
            },
            Group {
                id: 20,
                data: rows(),
            },
        ],
    });
    let group = store.groups().at_key(10);
    let indexed = group.data().rows().at_unkeyed(0);
    assert_eq!(indexed.id().get_untracked(), 1);
    assert_eq!(store.groups().at_key(20).id().get_untracked(), 20);
    {
        let mut writer = store.write_untracked();
        writer.groups[0].data.rows.reverse();
        writer.groups.reverse();
    }
    assert_eq!(indexed.id().get_untracked(), 2);
    let text = indexed.state().text_0().unwrap().items();
    assert_eq!(text.at_key("x".into()).get_untracked(), "x");
    assert_eq!(
        group
            .data()
            .rows()
            .at_key(1)
            .state()
            .numbers_0()
            .unwrap()
            .items()
            .at_key(10)
            .value()
            .get_untracked(),
        1
    );
    assert_eq!(text.at_key("x".into()).get_untracked(), "x");
    indexed.state().set(State::Empty);
    assert!(group.data().rows().at_key(2).state().empty());
    assert!(
        store
            .groups()
            .at_key(20)
            .data()
            .rows()
            .at_key(2)
            .state()
            .text()
    );
}

#[derive(Clone, Debug, PartialEq, Store, Patch)]
struct Wrapped {
    id: usize,
    #[patch(|this, new| *this = new)]
    value: Option<Box<Item>>,
    children: Numbers,
}

#[derive(Clone, Debug, PartialEq, Store, Patch)]
struct Wrappers {
    #[store(key: usize = |item| item.id)]
    items: Vec<Wrapped>,
}

#[tokio::test]
async fn indexed_dependencies_survive_option_deref_keyed_and_iterator_projections()
 {
    _ = any_spawner::Executor::init_tokio();
    let store = Store::new(Wrappers {
        items: vec![
            Wrapped {
                id: 1,
                value: Some(Box::new(Item { id: 10, value: 1 })),
                children: Numbers {
                    items: vec![Item { id: 10, value: 1 }],
                },
            },
            Wrapped {
                id: 2,
                value: Some(Box::new(Item { id: 10, value: 2 })),
                children: Numbers {
                    items: vec![
                        Item { id: 10, value: 2 },
                        Item { id: 20, value: 3 },
                    ],
                },
            },
        ],
    });
    let indexed = store.items().at_unkeyed(0);
    let optional =
        observe(move || indexed.value().unwrap().deref_field().value().get());
    let nested_keyed =
        observe(move || indexed.children().items().at_key(10).value().get());
    let iterated =
        observe(move || indexed.children().items().iter_unkeyed().count());
    tick().await;
    assert_eq!(*optional.lock().unwrap(), [1]);
    assert_eq!(*nested_keyed.lock().unwrap(), [1]);
    assert_eq!(*iterated.lock().unwrap(), [1]);
    let mut replacement = store.items().get_untracked();
    replacement.reverse();
    store.items().patch(replacement);
    tick().await;
    assert_eq!(*optional.lock().unwrap(), [1, 2]);
    assert_eq!(*nested_keyed.lock().unwrap(), [1, 2]);
    assert_eq!(*iterated.lock().unwrap(), [1, 2]);
}

#[tokio::test]
async fn missing_index_variant_effect_recovers_when_the_collection_grows() {
    _ = any_spawner::Executor::init_tokio();
    let store = Store::new(Rows { rows: vec![] });
    let indexed = store.rows().at_unkeyed(0);
    let seen = observe(move || indexed.state().empty());
    tick().await;
    assert_eq!(*seen.lock().unwrap(), [false]);
    store.rows().patch(vec![Row {
        id: 1,
        state: State::Empty,
    }]);
    tick().await;
    assert_eq!(*seen.lock().unwrap(), [false, true]);
    store.rows().patch(vec![]);
    tick().await;
    assert_eq!(*seen.lock().unwrap(), [false, true, false]);
}

fn assert_erased_collection_write(
    store: Store<Rows>,
    collection: impl StoreField<Value = Vec<Row>> + Clone + IsDisposed,
) {
    // The first operation is a write, so there is no reader available to
    // initialize metadata once the parent writer has acquired the value lock.
    let indexed = collection.at_unkeyed(0);
    indexed.clone().state().set(State::Empty);
    assert!(store.rows().at_key(2).state().empty());
    assert!(store.rows().at_key(1).state().numbers());
    indexed.state().set(State::Text(Text {
        items: vec!["y".into()],
    }));
    assert_eq!(
        store
            .rows()
            .at_key(2)
            .state()
            .text_0()
            .unwrap()
            .items()
            .at_key("y".into())
            .get_untracked(),
        "y",
    );
}

#[test]
fn erased_keyed_collections_resolve_cold_and_stale_indexed_writes() {
    for warm in [false, true] {
        let store = Store::new(rows());
        let collection: ArcField<Vec<Row>> = store.rows().into();
        if warm {
            assert_eq!(store.rows().at_key(1).id().get_untracked(), 1);
        }
        store.write_untracked().rows.reverse();
        assert_erased_collection_write(store, collection);

        let store = Store::new(rows());
        let collection: Field<Vec<Row>> = store.rows().into();
        if warm {
            assert_eq!(store.rows().at_key(1).id().get_untracked(), 1);
        }
        store.write_untracked().rows.reverse();
        assert_erased_collection_write(store, collection);
    }
}

#[tokio::test]
async fn erased_keyed_collections_keep_index_effects_positional() {
    _ = any_spawner::Executor::init_tokio();
    let store = Store::new(rows());
    let arc: ArcField<Vec<Row>> = store.rows().into();
    let field: Field<Vec<Row>> = store.rows().into();
    let arc_seen = observe(move || arc.clone().at_unkeyed(0).id().get());
    let field_seen = observe(move || field.at_unkeyed(0).state().text());
    tick().await;
    assert_eq!(*arc_seen.lock().unwrap(), [1]);
    assert_eq!(*field_seen.lock().unwrap(), [false]);
    let mut replacement = rows().rows;
    replacement.reverse();
    store.rows().patch(replacement);
    tick().await;
    assert_eq!(*arc_seen.lock().unwrap(), [1, 2]);
    assert_eq!(*field_seen.lock().unwrap(), [false, true]);
}

#[tokio::test]
async fn indexed_writes_notify_the_current_key_not_the_previous_occupant() {
    _ = any_spawner::Executor::init_tokio();
    let store = Store::new(rows());
    let first = observe(move || store.rows().at_key(1).state().get());
    let second = observe(move || store.rows().at_key(2).state().get());
    let indexed = observe(move || store.rows().at_unkeyed(0).state().get());
    tick().await;
    let original = rows().rows;
    assert_eq!(*first.lock().unwrap(), [original[0].state.clone()]);
    assert_eq!(*second.lock().unwrap(), [original[1].state.clone()]);

    let mut replacement = original.clone();
    replacement.reverse();
    store.rows().patch(replacement);
    tick().await;
    store.rows().at_unkeyed(0).state().set(State::Empty);
    tick().await;
    assert_eq!(*first.lock().unwrap(), [original[0].state.clone()]);
    assert_eq!(
        *second.lock().unwrap(),
        [original[1].state.clone(), State::Empty]
    );
    assert_eq!(
        *indexed.lock().unwrap(),
        [
            original[0].state.clone(),
            original[1].state.clone(),
            State::Empty
        ],
    );
    store
        .rows()
        .at_key(2)
        .state()
        .set(original[1].state.clone());
    tick().await;
    assert_eq!(
        *indexed.lock().unwrap(),
        [
            original[0].state.clone(),
            original[1].state.clone(),
            State::Empty,
            original[1].state.clone()
        ],
    );
    assert_eq!(*first.lock().unwrap(), [original[0].state.clone()]);
}
