use reactive_graph::traits::{GetUntracked, ReadUntracked, Set, Write};
use reactive_stores::{Patch, Store};

#[derive(Debug, Clone, PartialEq, Store, Patch)]
struct Item<K: reactive_stores::PatchField> {
    id: K,
    value: i32,
}

#[derive(Debug, Clone, PartialEq, Store, Patch)]
enum State {
    Numbers(#[store(key: usize = |item| item.id)] Vec<Item<usize>>),
    Text(#[store(key: String = |item| item.id.clone())] Vec<Item<String>>),
    OtherNumbers(#[store(key: usize = |item| item.id)] Vec<Item<usize>>),
    Plain(Item<usize>),
    Empty,
}

#[derive(Clone, Copy, Debug)]
enum Mutation {
    Patch,
    Set,
    Untracked,
}

const MUTATIONS: [Mutation; 3] =
    [Mutation::Patch, Mutation::Set, Mutation::Untracked];

fn replace(store: Store<State>, value: State, mutation: Mutation) {
    match mutation {
        Mutation::Patch => store.patch(value),
        Mutation::Set => store.set(value),
        Mutation::Untracked => *store.write_untracked() = value,
    }
}

fn numbers(value: i32) -> State {
    State::Numbers(vec![Item { id: 10, value }])
}

fn text(value: i32) -> State {
    State::Text(vec![Item {
        id: "a".into(),
        value,
    }])
}

#[test]
fn fresh_accessors_isolate_different_and_same_key_types_after_all_mutations() {
    for mutation in MUTATIONS {
        let store = Store::new(numbers(1));
        assert_eq!(
            store
                .numbers_0()
                .unwrap()
                .at_key(10)
                .value()
                .get_untracked(),
            1
        );

        replace(store, text(2), mutation);
        // Even a mismatching accessor must observe the actual current variant.
        assert!(store.numbers_0().is_none());
        assert_eq!(
            store
                .text_0()
                .unwrap()
                .at_key("a".into())
                .value()
                .get_untracked(),
            2
        );
        store.text_0().unwrap().at_key("a".into()).value().patch(3);

        replace(
            store,
            State::OtherNumbers(vec![Item { id: 10, value: 4 }]),
            mutation,
        );
        assert_eq!(
            store
                .other_numbers_0()
                .unwrap()
                .at_key(10)
                .value()
                .get_untracked(),
            4
        );
        replace(store, numbers(5), mutation);
        let current = store.numbers_0().unwrap().at_key(10);
        assert_eq!(current.value().get_untracked(), 5);
        current.value().set(6);
        assert_eq!(store.get_untracked(), numbers(6));

        replace(store, State::Empty, mutation);
        assert!(store.empty());
        replace(store, text(7), mutation);
        assert_eq!(
            store
                .text_0()
                .unwrap()
                .at_key("a".into())
                .value()
                .get_untracked(),
            7
        );
    }
}

#[test]
fn same_variant_patch_after_replacement_does_not_require_an_accessor() {
    for mutation in MUTATIONS {
        let store = Store::new(numbers(1));
        assert_eq!(
            store
                .numbers_0()
                .unwrap()
                .at_key(10)
                .value()
                .get_untracked(),
            1
        );
        replace(store, text(2), mutation);
        store.patch(text(3));
        assert_eq!(
            store
                .text_0()
                .unwrap()
                .at_key("a".into())
                .value()
                .get_untracked(),
            3
        );
    }
}

#[test]
fn plain_payload_does_not_inherit_collection_index_mapping() {
    for mutation in MUTATIONS {
        let store = Store::new(State::Numbers(vec![]));
        // Publish an empty mapping: treating a plain struct field as an item
        // index would fail even for index zero.

        assert!(
            store
                .numbers_0()
                .unwrap()
                .at_key(10)
                .try_read_untracked()
                .is_none()
        );
        replace(store, State::Plain(Item { id: 10, value: 1 }), mutation);
        store.patch(State::Plain(Item { id: 10, value: 2 }));
        let current = store.plain_0().unwrap();
        current.value().patch(3);
        assert_eq!(current.value().get_untracked(), 3);
        replace(store, numbers(4), mutation);
        assert_eq!(
            store
                .numbers_0()
                .unwrap()
                .at_key(10)
                .value()
                .get_untracked(),
            4
        );
    }
}

#[derive(Debug, Clone, PartialEq, Store, Patch)]
struct Group {
    id: usize,
    state: State,
}

#[derive(Debug, Clone, PartialEq, Store, Patch)]
struct Groups {
    #[store(key: usize = |group| group.id)]
    groups: Vec<Group>,
}

#[test]
fn direct_keyed_patch_continues_after_replacing_an_enum_variant() {
    for reordered in [false, true] {
        let store = Store::new(Groups {
            groups: vec![
                Group {
                    id: 10,
                    state: numbers(1),
                },
                Group {
                    id: 20,
                    state: numbers(2),
                },
            ],
        });
        assert_eq!(
            store
                .groups()
                .at_key(10)
                .state()
                .numbers_0()
                .unwrap()
                .at_key(10)
                .value()
                .get_untracked(),
            1
        );
        let mut replacement = vec![
            Group {
                id: 10,
                state: text(11),
            },
            Group {
                id: 20,
                state: numbers(22),
            },
        ];
        if reordered {
            replacement.reverse();
        }
        store.groups().patch(replacement.clone());
        assert_eq!(store.get_untracked().groups, replacement);
    }
}

#[test]
fn nested_variant_replacement_uses_old_key_mapping_during_outer_reorder() {
    let store = Store::new(Groups {
        groups: vec![
            Group {
                id: 10,
                state: numbers(1),
            },
            Group {
                id: 20,
                state: numbers(2),
            },
        ],
    });
    let first = store.groups().at_key(10);
    let second = store.groups().at_key(20);
    assert_eq!(
        first
            .state()
            .numbers_0()
            .unwrap()
            .at_key(10)
            .value()
            .get_untracked(),
        1
    );
    let unchanged = second.state().numbers_0().unwrap().at_key(10);
    assert_eq!(unchanged.value().get_untracked(), 2);

    store.patch(Groups {
        groups: vec![
            Group {
                id: 20,
                state: numbers(22),
            },
            Group {
                id: 10,
                state: text(11),
            },
        ],
    });
    assert_eq!(
        first
            .state()
            .text_0()
            .unwrap()
            .at_key("a".into())
            .value()
            .get_untracked(),
        11
    );
    assert_eq!(unchanged.value().get_untracked(), 22);
    unchanged.value().patch(23);
    assert_eq!(
        second
            .state()
            .numbers_0()
            .unwrap()
            .at_key(10)
            .value()
            .get_untracked(),
        23
    );
}
