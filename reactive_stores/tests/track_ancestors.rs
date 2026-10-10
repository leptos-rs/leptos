//! Every projection of a store field must subscribe to the same triggers when
//! tracked as the plain subfield it wraps: its own `this` and `children`, plus
//! `this` for each ancestor. Replacing a containing value (including the whole
//! store) then re-runs the observer, while writes to siblings do not.

use reactive_graph::{
    effect::ImmediateEffect,
    owner::Owner,
    traits::{Set, Track, Write},
};
use reactive_stores::{DerefField, Field, Store, StoreFieldIterator};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Clone, Debug, Default, Store)]
struct State {
    boxed: Box<Inner>,
    items: Vec<i32>,
    sibling: i32,
}

#[derive(Clone, Debug, Default, Store)]
struct Inner {
    value: i32,
}

fn state(value: i32) -> State {
    State {
        boxed: Box::new(Inner { value }),
        items: vec![value],
        sibling: value,
    }
}

/// Runs `track` inside an immediate effect and returns the run count after:
/// the initial run, a root replacement, a sibling write, and a write to the
/// tracked field itself.
fn observe(
    track: impl Fn(Store<State>) + Send + Sync + 'static,
    write_own: impl Fn(Store<State>),
) -> [usize; 4] {
    let owner = Owner::new();
    owner.with(|| {
        let store = Store::new(state(0));
        let runs = Arc::new(AtomicUsize::new(0));
        let _effect = ImmediateEffect::new({
            let runs = Arc::clone(&runs);
            move || {
                track(store);
                runs.fetch_add(1, Ordering::Relaxed);
            }
        });
        let initial = runs.load(Ordering::Relaxed);
        store.set(state(1));
        let after_root = runs.load(Ordering::Relaxed);
        store.sibling().set(2);
        let after_sibling = runs.load(Ordering::Relaxed);
        write_own(store);
        let after_own = runs.load(Ordering::Relaxed);
        [initial, after_root, after_sibling, after_own]
    })
}

/// The plain subfield is the reference every other projection must match.
fn boxed_subfield() -> [usize; 4] {
    observe(|s| s.boxed().track(), |s| s.boxed().write().value += 1)
}

#[test]
fn subfield_tracks_root_replacement_but_not_siblings() {
    let [initial, after_root, after_sibling, after_own] = boxed_subfield();
    assert_eq!(initial, 1);
    assert!(after_root > initial, "root replacement must re-run");
    assert_eq!(after_sibling, after_root, "sibling write must not re-run");
    assert!(after_own > after_sibling, "own write must re-run");
}

#[test]
fn derefed_field_tracks_like_its_inner_field() {
    let derefed = observe(
        |s| s.boxed().deref_field().track(),
        |s| s.boxed().deref_field().write().value += 1,
    );
    assert_eq!(derefed, boxed_subfield());
}

#[test]
fn erased_derefed_field_tracks_like_its_inner_field() {
    let erased = observe(
        |s| Field::from(s.boxed().deref_field()).track(),
        |s| s.boxed().deref_field().write().value += 1,
    );
    assert_eq!(erased, boxed_subfield());
}

#[test]
fn unkeyed_iteration_tracks_like_the_collection_field() {
    let collection =
        observe(|s| s.items().track(), |s| s.items().write().push(3));
    let iterated = observe(
        |s| {
            let _ = s.items().iter_unkeyed().count();
        },
        |s| s.items().write().push(3),
    );
    assert_eq!(iterated, collection);
}
