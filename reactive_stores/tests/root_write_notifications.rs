//! Writing to the store root must notify its observers exactly like writing
//! to any other field does: the root writer carries the root's own triggers,
//! and `Write` returns it as is instead of notifying a second time.

use reactive_graph::{
    effect::ImmediateEffect,
    owner::Owner,
    traits::{Set, Track, Update, Write},
};
use reactive_stores::{ArcStore, Store};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Clone, Debug, Default, Store)]
struct State {
    value: i32,
}

fn store() -> Store<State> {
    Store::new(State::default())
}

fn arc_store() -> ArcStore<State> {
    ArcStore::new(State::default())
}

/// Creates a store, tracks it in an immediate effect and returns how many
/// times the effect re-runs because of `write`.
fn reruns<S: Clone + Send + Sync + 'static>(
    make: impl FnOnce() -> S,
    track: impl Fn(S) + Send + Sync + 'static,
    write: impl FnOnce(S),
) -> usize {
    let owner = Owner::new();
    owner.with(|| {
        let store = make();
        let runs = Arc::new(AtomicUsize::new(0));
        let _effect = ImmediateEffect::new({
            let store = store.clone();
            let runs = Arc::clone(&runs);
            move || {
                track(store.clone());
                runs.fetch_add(1, Ordering::Relaxed);
            }
        });
        let before = runs.load(Ordering::Relaxed);
        write(store);
        runs.load(Ordering::Relaxed) - before
    })
}

#[test]
fn root_write_notifies_root_observers_like_a_field_write() {
    let field_write =
        reruns(store, |s| s.value().track(), |s| s.value().set(1));
    assert!(field_write > 0);

    let root_set = reruns(store, |s| s.track(), |s| s.set(State { value: 1 }));
    let root_update =
        reruns(store, |s| s.track(), |s| s.update(|v| v.value = 1));
    let root_write =
        reruns(store, |s| s.track(), |s| *s.write() = State { value: 1 });
    let arc_root_set =
        reruns(arc_store, |s| s.track(), |s| s.set(State { value: 1 }));
    assert_eq!(
        [root_set, root_update, root_write, arc_root_set],
        [field_write; 4]
    );
}

#[test]
fn root_write_notifies_field_observers_once() {
    assert_eq!(
        reruns(store, |s| s.value().track(), |s| s.set(State { value: 1 })),
        1
    );
}

#[test]
fn untracked_root_write_is_silent() {
    assert_eq!(
        reruns(
            store,
            |s| s.track(),
            |s| *s.write_untracked() = State { value: 1 }
        ),
        0
    );
    assert_eq!(
        reruns(
            arc_store,
            |s| s.track(),
            |s| *s.write_untracked() = State { value: 1 }
        ),
        0
    );
}
