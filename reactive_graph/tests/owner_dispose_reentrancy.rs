// Disposing an owner must not drop its arena values while the arena is locked.
//
// Dropping such a value can run code that accesses the arena again. Here an
// `Owner` is kept alive only by a `StoredValue` of a sibling owner, so disposing
// the sibling drops it, which runs its cleanups and removes its own nodes. While
// the arena was locked this deadlocked natively and aborted the process on
// single-threaded wasm.
//
// Each test runs on its own thread so that a regression fails the test instead
// of hanging the test binary.

use reactive_graph::{
    owner::{Owner, StoredValue, on_cleanup},
    signal::signal,
    traits::{GetValue, Set},
};
use std::{sync::mpsc, thread, time::Duration};

fn finishes(f: impl FnOnce() + Send + 'static) -> bool {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        f();
        _ = tx.send(());
    });
    rx.recv_timeout(Duration::from_secs(5)).is_ok()
}

/// Runs `setup` under an owner `inner` that is kept alive only by a
/// `StoredValue` of its sibling `outer`, then disposes `outer` with `dispose`.
///
/// Both are children of the same root so they share an arena, also with
/// `sandboxed-arenas`.
fn dispose_through_arena(
    root: &Owner,
    setup: impl FnOnce(),
    dispose: impl FnOnce(Owner),
) {
    let (inner, outer) = root.with(|| (Owner::new(), Owner::new()));

    inner.with(setup);
    outer.with(|| {
        _ = StoredValue::new(inner);
    });

    dispose(outer);
}

#[test]
fn dropping_owner_held_in_arena_removes_its_nodes() {
    assert!(finishes(|| {
        let root = Owner::new();
        dispose_through_arena(
            &root,
            || {
                _ = signal(0);
            },
            drop,
        );
    }));
}

#[test]
fn dropping_owner_held_in_arena_runs_cleanups_accessing_arena() {
    assert!(finishes(|| {
        let root = Owner::new();
        let value = root.with(|| StoredValue::new(0));

        dispose_through_arena(
            &root,
            move || {
                let (_, set_count) = signal(0);
                on_cleanup(move || {
                    _ = value.get_value();
                    set_count.set(1);
                });
            },
            drop,
        );
    }));
}

#[test]
fn cleaning_up_owner_holding_owner_in_arena() {
    assert!(finishes(|| {
        let root = Owner::new();
        dispose_through_arena(
            &root,
            || {
                _ = signal(0);
            },
            |outer| outer.cleanup(),
        );
    }));
}
