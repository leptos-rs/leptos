//! Regression tests for arena restoration.
//!
//! Polling a `Sandboxed` future and running `Owner::with` both install another
//! arena for the duration of the call. They must restore the caller's arena
//! afterwards, also when the call panics; otherwise that arena bleeds into
//! whatever runs next on the same thread, breaking the per-request isolation
//! the `sandboxed-arenas` feature exists to provide.
#![cfg(feature = "sandboxed-arenas")]

use futures::executor::block_on;
use reactive_graph::{
    owner::{ArenaItem, Owner, StoredValue},
    traits::{GetValue, ReadValue},
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
