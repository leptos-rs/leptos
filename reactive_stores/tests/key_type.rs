use reactive_graph::traits::GetUntracked;
use reactive_stores::{Patch, Store};

#[test]
fn enum_patch_before_accessor_preserves_declared_key_type() {
    #[derive(Store, Patch)]
    enum State {
        Ready(#[store(key: usize = |_| 0)] Vec<i32>),
    }

    let store = Store::new(State::Ready(vec![1]));
    store.patch(State::Ready(vec![2]));

    assert_eq!(
        store.ready_0().unwrap().at_key(0usize).try_get_untracked(),
        Some(2)
    );
}

type Key = usize;
type Items = Vec<i32>;

#[test]
fn struct_patch_preserves_key_and_container_aliases() {
    #[derive(Store, Patch)]
    struct State {
        #[store(key: Key = |_| 0)]
        items: Items,
    }

    let store = Store::new(State { items: vec![1] });
    store.patch(State { items: vec![2] });

    assert_eq!(store.items().at_key(0usize).try_get_untracked(), Some(2));
}

#[test]
fn struct_custom_patch_preserves_declared_key_type() {
    #[derive(Store, Patch)]
    struct State {
        #[store(key: Key = |_| 0)]
        #[patch(|this, new| *this = new)]
        items: Items,
    }

    let store = Store::new(State { items: vec![1] });
    store.patch(State { items: vec![2] });

    assert_eq!(store.items().at_key(0usize).try_get_untracked(), Some(2));
}

#[test]
fn enum_custom_patch_preserves_declared_key_type() {
    #[derive(Store, Patch)]
    enum State {
        Ready {
            #[store(key: Key = |_| 0)]
            #[patch(|this, new| *this = new)]
            items: Items,
        },
    }

    let store = Store::new(State::Ready { items: vec![1] });
    store.patch(State::Ready { items: vec![2] });

    assert_eq!(
        store
            .ready_items()
            .unwrap()
            .at_key(0usize)
            .try_get_untracked(),
        Some(2)
    );
}

#[test]
fn enum_replacement_before_accessor_preserves_declared_key_type() {
    #[derive(Store, Patch)]
    enum State {
        Idle,
        Ready(#[store(key: Key = |_| 0)] Items),
    }

    let store = Store::new(State::Idle);
    store.patch(State::Ready(vec![2]));

    assert_eq!(
        store.ready_0().unwrap().at_key(0usize).try_get_untracked(),
        Some(2)
    );
}
