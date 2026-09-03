//! Regression test for the suppression timing of `OnceResource`.
//!
//! `Resource` evaluates `SuppressResourceLoad` when its fetcher future is
//! polled, while `OnceResource` used to evaluate it once, at construction. A
//! `OnceResource` created inside a suppression scope therefore never loaded —
//! not even after the guard dropped — because the loader task was skipped at
//! construction. The fetch-time check makes the two consistent.

use any_spawner::Executor;
use leptos_server::{ArcOnceResource, SuppressResourceLoad};
use reactive_graph::owner::Owner;
use std::time::Duration;

#[tokio::test]
async fn once_resource_constructed_under_suppression_loads_after_guard_drops() {
    _ = Executor::init_tokio();
    let owner = Owner::new();
    owner.set();

    // Construct the resource while suppression is active, then drop the guard
    // before the spawned loader is ever polled.
    let resource = {
        let _guard = SuppressResourceLoad::new();
        ArcOnceResource::<u32>::new(async { 42 })
    };

    // By the time the loader is first polled the guard is gone, so the value
    // loads. Before the fix this hung forever, because construction-time
    // suppression skipped spawning the loader entirely.
    let value =
        tokio::time::timeout(
            Duration::from_secs(5),
            async move { resource.await },
        )
        .await
        .expect("OnceResource created under suppression should still load");

    assert_eq!(value, 42);
}
