//! Regression tests for arena restoration.
//!
//! Polling a `Sandboxed` future, running `Owner::with`, and cleaning up or
//! dropping an owner each install another arena for the duration of the call,
//! including cleanup callbacks and value destructors. They must restore the
//! caller's arena afterwards, also when the call panics; otherwise that arena
//! bleeds into whatever runs next on the same thread, breaking the per-request
//! isolation the `sandboxed-arenas` feature exists to provide.
#![cfg(feature = "sandboxed-arenas")]

use futures::executor::block_on;
use reactive_graph::{
    owner::{ArenaItem, Owner, StoredValue, on_cleanup, provide_context},
    traits::{Dispose, GetValue, ReadValue},
};
use std::panic::{AssertUnwindSafe, catch_unwind};

#[test]
fn sandboxed_poll_restores_callers_arena() {
    // Two independent owners, each with its own arena. `owner_b` is created
    // before `owner_a` is set as current, so it does not inherit A's arena.
    let owner_a = Owner::new();
    let owner_b = Owner::new();

    // Build a future bound to arena B.
    owner_b.set();
    let sandboxed = reactive_graph::owner::Sandboxed::new(async { 42_u32 });

    // Switch to arena A and create a value there.
    owner_a.set();
    let a_value = StoredValue::new(1_u32);
    assert_eq!(a_value.try_read_value().as_deref(), Some(&1));

    // Polling the sandboxed future installs arena B during the poll. After it
    // returns, arena A must be active again.
    let out = block_on(sandboxed);
    assert_eq!(out, 42);

    assert_eq!(
        a_value.try_read_value().as_deref(),
        Some(&1),
        "Sandboxed::poll leaked arena B into the caller's context"
    );
}

#[test]
fn owner_with_restores_callers_owner_and_arena() {
    // Both owners exist before `owner_a` is set, so each has its own arena.
    let owner_a = Owner::new();
    let owner_b = Owner::new();
    owner_a.set();
    let a_value = StoredValue::new(11_u32);

    owner_b.with(|| {
        _ = StoredValue::new(22_u32);
    });

    assert_eq!(
        Owner::current(),
        Some(owner_a.clone()),
        "Owner::with did not restore the previous owner"
    );
    assert_eq!(
        a_value.try_get_value(),
        Some(11),
        "Owner::with leaked arena B into the caller's context"
    );

    // A value created now is registered with A, so it must live in A's arena.
    let after = StoredValue::new(33_u32);
    owner_a.set();
    assert_eq!(
        after.try_get_value(),
        Some(33),
        "value created after Owner::with was not stored in A's arena"
    );
}

#[test]
fn owner_with_restores_callers_arena_after_panic() {
    let owner_a = Owner::new();
    let owner_b = Owner::new();
    owner_a.set();
    let a_value = StoredValue::new(11_u32);

    let result = catch_unwind(AssertUnwindSafe(|| {
        owner_b.with(|| {
            _ = StoredValue::new(22_u32);
            panic!("user callback");
        })
    }));

    assert!(result.is_err());
    assert_eq!(
        a_value.try_get_value(),
        Some(11),
        "a panic in Owner::with leaked arena B into the caller's context"
    );
    assert_eq!(
        Owner::current(),
        Some(owner_a),
        "a panic in Owner::with left its owner as the current owner"
    );
}

#[test]
fn owner_with_restores_absent_owner_and_arena() {
    // With no current owner, no arena is active either.
    let owner_b = Owner::new();

    let b_value = owner_b.with(|| StoredValue::new(22_u32));

    assert_eq!(Owner::current(), None);
    assert_eq!(
        b_value.try_get_value(),
        None,
        "Owner::with left arena B active after returning to a context without \
         an arena"
    );
}

#[test]
fn owner_cleanup_restores_callers_arena() {
    let owner_a = Owner::new();
    let owner_b = Owner::new();
    owner_a.set();
    let a_value = StoredValue::new(10_u32);
    owner_b.set();
    on_cleanup(|| {});
    owner_a.set();

    owner_b.cleanup();

    assert_eq!(
        a_value.try_get_value(),
        Some(10),
        "a cleanup callback of B leaked arena B into the caller's context"
    );

    // A value created now is registered with A, so it must live in A's arena.
    let after = StoredValue::new(30_u32);
    owner_a.set();
    assert_eq!(
        after.try_get_value(),
        Some(30),
        "value created after B's cleanup was not stored in A's arena"
    );
}

#[test]
fn owner_drop_restores_callers_arena() {
    let owner_a = Owner::new();
    let owner_b = Owner::new();
    owner_a.set();
    let a_value = StoredValue::new(10_u32);
    owner_b.set();
    on_cleanup(|| {});
    owner_a.set();

    // This is B's last handle, so dropping it runs B's cleanup callbacks.
    drop(owner_b);

    assert_eq!(
        a_value.try_get_value(),
        Some(10),
        "a cleanup callback of dropped B leaked arena B into the caller's \
         context"
    );

    let after = StoredValue::new(30_u32);
    owner_a.set();
    assert_eq!(
        after.try_get_value(),
        Some(30),
        "value created after dropping B was not stored in A's arena"
    );
}

#[test]
fn owner_cleanup_restores_callers_arena_after_panic() {
    let owner_a = Owner::new();
    let owner_b = Owner::new();
    owner_a.set();
    let a_value = StoredValue::new(10_u32);
    owner_b.set();
    on_cleanup(|| panic!("cleanup callback"));
    owner_a.set();

    let result = catch_unwind(AssertUnwindSafe(|| owner_b.cleanup()));

    assert!(result.is_err());
    assert_eq!(
        a_value.try_get_value(),
        Some(10),
        "a panicking cleanup callback of B leaked arena B into the caller's \
         context"
    );
}

#[test]
fn owner_cleanup_restores_absent_arena() {
    // B shares its parent's arena, and B's cleanup does not dispose the
    // parent's value, so reading it shows whether that arena is active.
    let parent = Owner::new();
    let owner_b = parent.child();
    let parent_value = parent.with(|| StoredValue::new(22_u32));
    owner_b.with(|| on_cleanup(|| {}));
    assert_eq!(
        parent_value.try_get_value(),
        None,
        "precondition: no arena is active"
    );

    owner_b.cleanup();

    assert_eq!(
        parent_value.try_get_value(),
        None,
        "a cleanup callback of B left arena B active in a context without an \
         arena"
    );
}

struct DisposeOnDrop(StoredValue<u32>);

impl Drop for DisposeOnDrop {
    fn drop(&mut self) {
        self.0.dispose();
    }
}

/// Returns owners A and B, whose arenas give their first values the same key,
/// a child of B that owns a value disposing B's first value when dropped, and
/// A's first value.
fn colliding_arenas() -> (Owner, Owner, Owner, StoredValue<u32>) {
    colliding_arenas_with(|value| {
        _ = StoredValue::new(value);
    })
}

fn colliding_arenas_with(
    store: impl FnOnce(DisposeOnDrop),
) -> (Owner, Owner, Owner, StoredValue<u32>) {
    let owner_a = Owner::new();
    let owner_b = Owner::new();
    owner_a.set();
    let value_a = StoredValue::new(10_u32);
    owner_b.set();
    let value_b = StoredValue::new(20_u32);
    assert_eq!(value_a, value_b, "the first arena keys must collide");
    let child = owner_b.child();
    child.with(|| store(DisposeOnDrop(value_b)));
    (owner_a, owner_b, child, value_a)
}

/// After its callbacks, a cleanup runs value destructors in the owner's arena.
#[test]
fn owner_cleanup_with_callback_preserves_callers_value() {
    let (owner_a, _owner_b, child, value_a) = colliding_arenas();
    child.with(|| on_cleanup(|| {}));
    owner_a.set();
    child.cleanup();
    owner_a.set();
    assert_eq!(value_a.try_get_value(), Some(10));
}

/// After its callbacks, dropping an owner runs value destructors in its arena.
#[test]
fn owner_drop_with_callback_preserves_callers_value() {
    let (owner_a, _owner_b, child, value_a) = colliding_arenas();
    child.with(|| on_cleanup(|| {}));
    owner_a.set();
    drop(child);
    owner_a.set();
    assert_eq!(value_a.try_get_value(), Some(10));
}

/// Without callbacks, a cleanup runs value destructors in the owner's arena too.
#[test]
fn owner_cleanup_without_callback_preserves_callers_value() {
    let (owner_a, _owner_b, child, value_a) = colliding_arenas();
    owner_a.set();
    child.cleanup();
    owner_a.set();
    assert_eq!(value_a.try_get_value(), Some(10));
}

/// Dropping an owner's contexts must run their destructors in its own arena.
#[test]
fn owner_drop_context_preserves_callers_value() {
    let (owner_a, _owner_b, child, value_a) =
        colliding_arenas_with(provide_context);
    owner_a.set();
    drop(child);
    owner_a.set();
    assert_eq!(value_a.try_get_value(), Some(10));
}

/// A failed arena switch during a borrow must panic once and preserve its caller.
#[test]
fn owner_with_inside_arena_borrow_panics_without_aborting() {
    let owner_a = Owner::new();
    let other_owner = Owner::new();
    owner_a.set();
    let value_a = StoredValue::new(10_u32);
    let item = ArenaItem::new(());

    let result = catch_unwind(AssertUnwindSafe(|| {
        item.try_with_value(|_| other_owner.with(|| ()))
    }));

    assert!(result.is_err(), "arena switch result: {result:?}");
    assert_eq!(Owner::current(), Some(owner_a.clone()));
    assert_eq!(value_a.try_get_value(), Some(10));
}
