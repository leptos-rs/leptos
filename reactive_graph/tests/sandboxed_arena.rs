//! Regression test for `Sandboxed` arena restoration.
//!
//! Polling a `Sandboxed` future installs the arena it was created under for the
//! duration of the poll. It must restore the caller's arena afterwards;
//! otherwise the sandboxed arena bleeds into whatever runs next on the same
//! thread, breaking the per-request isolation the `sandboxed-arenas` feature
//! exists to provide.
#![cfg(feature = "sandboxed-arenas")]

use futures::executor::block_on;
use reactive_graph::{
    owner::{Owner, StoredValue},
    traits::ReadValue,
};

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
