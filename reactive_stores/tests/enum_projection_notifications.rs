use reactive_graph::{
    effect::{Effect, ImmediateEffect},
    traits::{Get, IsDisposed, Set, Track, UntrackableGuard, Write},
};
use reactive_stores::{
    ArcField, ArcStore, DerefField, Field, Store, StoreField,
};
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, PartialEq, Store)]
enum State {
    Idle,
    Ready { count: i32, other: i32 },
}

fn observe_variants(
    field: impl StoreField<Value = State> + Clone + Send + Sync + 'static,
) -> (ImmediateEffect, Arc<Mutex<Vec<(bool, bool)>>>) {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let effect = ImmediateEffect::new({
        let seen = Arc::clone(&seen);
        move || {
            let idle = field.clone().idle();
            let has_count = field.clone().ready_count().is_some();
            seen.lock().unwrap().push((idle, has_count));
        }
    });
    (effect, seen)
}

fn assert_projection_notifications(
    root: impl StoreField<Value = State> + Clone + Send + Sync + 'static,
    projection: impl StoreField<Value = State>
    + Set<Value = State>
    + Clone
    + Send
    + Sync
    + 'static,
) {
    let (_root_effect, root_seen) = observe_variants(root);
    let (_projection_effect, projection_seen) =
        observe_variants(projection.clone());
    assert_eq!(*root_seen.lock().unwrap(), [(true, false)]);
    assert_eq!(*projection_seen.lock().unwrap(), [(true, false)]);

    projection.set(State::Ready { count: 1, other: 2 });
    assert_eq!(*root_seen.lock().unwrap(), [(true, false), (false, true)]);
    assert_eq!(
        *projection_seen.lock().unwrap(),
        [(true, false), (false, true)]
    );

    projection.set(State::Idle);
    assert_eq!(
        *root_seen.lock().unwrap(),
        [(true, false), (false, true), (true, false)]
    );
    assert_eq!(
        *projection_seen.lock().unwrap(),
        [(true, false), (false, true), (true, false)]
    );
}

#[test]
fn root_arc_field_notifies_enum_accessors() {
    let store = Store::new(State::Idle);
    let field: ArcField<State> = store.into();
    assert_projection_notifications(store, field);
}

#[test]
fn arc_store_root_arc_field_notifies_enum_accessors() {
    let store = ArcStore::new(State::Idle);
    let field: ArcField<State> = store.clone().into();
    assert_projection_notifications(store, field);
}

#[test]
fn root_field_notifies_enum_accessors() {
    let store = Store::new(State::Idle);
    let field: Field<State> = store.into();
    assert_projection_notifications(store, field);
}

#[test]
fn arc_store_root_field_notifies_enum_accessors() {
    let store = ArcStore::new(State::Idle);
    let field: Field<State> = store.clone().into();
    assert_projection_notifications(store, field);
}

#[test]
fn root_box_deref_notifies_enum_accessors() {
    let store = Store::new(Box::new(State::Idle));
    assert_projection_notifications(store.deref_field(), store.deref_field());
}

#[test]
fn arc_store_root_box_deref_notifies_enum_accessors() {
    let store = ArcStore::new(Box::new(State::Idle));
    assert_projection_notifications(
        store.clone().deref_field(),
        store.deref_field(),
    );
}

#[test]
fn root_store_write_notifies_enum_accessors_once() {
    let store = Store::new(State::Idle);
    assert_projection_notifications(store, store);
}

#[test]
fn root_arc_store_write_notifies_enum_accessors_once() {
    let store = ArcStore::new(State::Idle);
    assert_projection_notifications(store.clone(), store);
}

fn assert_untracked_writes_are_silent(
    field: impl StoreField<Value = State>
    + Write<Value = State>
    + Clone
    + Send
    + Sync
    + 'static,
) {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let _effect = ImmediateEffect::new({
        let field = field.clone();
        let seen = Arc::clone(&seen);
        move || {
            field.track_field();
            seen.lock().unwrap().push(field.reader().unwrap().clone());
        }
    });
    assert_eq!(*seen.lock().unwrap(), [State::Idle]);

    *field.write_untracked() = State::Ready { count: 1, other: 2 };
    assert_eq!(
        *field.reader().unwrap(),
        State::Ready { count: 1, other: 2 }
    );
    assert_eq!(*seen.lock().unwrap(), [State::Idle]);

    {
        let mut guard = field.write();
        guard.untrack();
        *guard = State::Idle;
    }
    assert_eq!(*field.reader().unwrap(), State::Idle);
    assert_eq!(*seen.lock().unwrap(), [State::Idle]);
}

#[test]
fn root_arc_field_untracked_writes_are_silent() {
    for field in [
        ArcField::from(Store::new(State::Idle)),
        ArcField::from(ArcStore::new(State::Idle)),
    ] {
        assert_untracked_writes_are_silent(field);
    }
}

#[test]
fn root_field_untracked_writes_are_silent() {
    for field in [
        Field::<State>::from(Store::new(State::Idle)),
        Field::<State>::from(ArcStore::new(State::Idle)),
    ] {
        assert_untracked_writes_are_silent(field);
    }
}

#[test]
fn root_box_deref_untracked_writes_are_silent() {
    assert_untracked_writes_are_silent(
        Store::new(Box::new(State::Idle)).deref_field(),
    );
    assert_untracked_writes_are_silent(
        ArcStore::new(Box::new(State::Idle)).deref_field(),
    );
}

#[test]
fn root_store_untracked_writes_are_silent() {
    assert_untracked_writes_are_silent(Store::new(State::Idle));
}

#[test]
fn root_arc_store_untracked_writes_are_silent() {
    assert_untracked_writes_are_silent(ArcStore::new(State::Idle));
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
    tokio::time::sleep(std::time::Duration::from_millis(1)).await;
}

async fn assert_payload_writes_stay_granular(
    field: impl StoreField<Value = State>
    + Write<Value = State>
    + IsDisposed
    + Track
    + Clone
    + Send
    + Sync
    + 'static,
) {
    let (_variant_effect, variants) = observe_variants(field.clone());
    let count = observe({
        let field = field.clone();
        move || field.clone().ready_count().map(|count| count.get())
    });
    let other = observe({
        let field = field.clone();
        move || field.clone().ready_other().map(|other| other.get())
    });
    let whole = observe({
        let field = field.clone();
        move || {
            field.track_field();
            field.reader().unwrap().clone()
        }
    });
    tick().await;
    assert_eq!(*count.lock().unwrap(), [Some(1)]);
    assert_eq!(*other.lock().unwrap(), [Some(2)]);
    assert_eq!(
        *whole.lock().unwrap(),
        [State::Ready { count: 1, other: 2 }]
    );

    field.clone().ready_other().unwrap().set(3);
    tick().await;
    assert_eq!(*count.lock().unwrap(), [Some(1)]);
    assert_eq!(*other.lock().unwrap(), [Some(2), Some(3)]);
    assert_eq!(*variants.lock().unwrap(), [(false, true)]);

    field.clone().ready_count().unwrap().set(4);
    tick().await;
    assert_eq!(*count.lock().unwrap(), [Some(1), Some(4)]);
    assert_eq!(*other.lock().unwrap(), [Some(2), Some(3)]);
    assert_eq!(*variants.lock().unwrap(), [(false, true)]);

    field.set(State::Idle);
    tick().await;
    assert_eq!(*count.lock().unwrap(), [Some(1), Some(4), None]);
    assert_eq!(*other.lock().unwrap(), [Some(2), Some(3), None]);
    assert_eq!(*variants.lock().unwrap(), [(false, true), (true, false)]);
    assert_eq!(
        *whole.lock().unwrap(),
        [
            State::Ready { count: 1, other: 2 },
            State::Ready { count: 1, other: 3 },
            State::Ready { count: 4, other: 3 },
            State::Idle,
        ]
    );
}

#[tokio::test]
async fn root_arc_field_payload_writes_stay_granular() {
    _ = any_spawner::Executor::init_tokio();
    for field in [
        ArcField::from(Store::new(State::Ready { count: 1, other: 2 })),
        ArcField::from(ArcStore::new(State::Ready { count: 1, other: 2 })),
    ] {
        assert_payload_writes_stay_granular(field).await;
    }
}

#[tokio::test]
async fn root_field_payload_writes_stay_granular() {
    _ = any_spawner::Executor::init_tokio();
    for field in [
        Field::<State>::from(Store::new(State::Ready { count: 1, other: 2 })),
        Field::<State>::from(ArcStore::new(State::Ready {
            count: 1,
            other: 2,
        })),
    ] {
        assert_payload_writes_stay_granular(field).await;
    }
}

#[tokio::test]
async fn root_box_deref_payload_writes_stay_granular() {
    _ = any_spawner::Executor::init_tokio();
    assert_payload_writes_stay_granular(
        Store::new(Box::new(State::Ready { count: 1, other: 2 })).deref_field(),
    )
    .await;
    assert_payload_writes_stay_granular(
        ArcStore::new(Box::new(State::Ready { count: 1, other: 2 }))
            .deref_field(),
    )
    .await;
}
