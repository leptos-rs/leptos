//! Regression tests for `AsyncTransition`.
//!
//! A transition must wait for the async resources created during it, and two
//! transitions that overlap in time must not observe one another's
//! registration slot.

use any_spawner::Executor;
use reactive_graph::{
    computed::ArcAsyncDerived, owner::Owner, transition::AsyncTransition,
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio::sync::Barrier;

/// The transition must not return until the resource created inside it has
/// resolved.
#[tokio::test]
async fn transition_waits_for_resource_created_inside() {
    _ = Executor::init_tokio();
    let owner = Owner::new();
    owner.set();

    let resolved = Arc::new(AtomicBool::new(false));
    let flag = resolved.clone();

    // The resource is created but deliberately *not* awaited here; the
    // transition itself is responsible for waiting until it resolves.
    let derived = AsyncTransition::run(move || async move {
        ArcAsyncDerived::new(move || {
            let flag = flag.clone();
            async move {
                Executor::tick().await;
                Executor::tick().await;
                flag.store(true, Ordering::SeqCst);
                42_u32
            }
        })
    })
    .await;

    assert!(
        resolved.load(Ordering::SeqCst),
        "transition returned before its resource resolved"
    );
    assert_eq!(derived.await, 42);
}

/// Two transitions forced to overlap must each independently wait for the
/// resource created within them; with the previous process-global slot, a
/// resource created during one transition could register against the other.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn overlapping_transitions_are_isolated() {
    _ = Executor::init_tokio();

    fn run_one(barrier: Arc<Barrier>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let owner = Owner::new();
            owner.set();
            let resolved = Arc::new(AtomicBool::new(false));
            let flag = resolved.clone();

            AsyncTransition::run(move || async move {
                // Force the two transitions to be "open" simultaneously.
                barrier.wait().await;
                let derived = ArcAsyncDerived::new(move || {
                    let flag = flag.clone();
                    async move {
                        Executor::tick().await;
                        Executor::tick().await;
                        flag.store(true, Ordering::SeqCst);
                        7_u32
                    }
                });
                let v = derived.await;
                assert_eq!(v, 7);
            })
            .await;

            assert!(
                resolved.load(Ordering::SeqCst),
                "transition returned before its own resource resolved"
            );
        })
    }

    let barrier = Arc::new(Barrier::new(2));
    let t1 = run_one(barrier.clone());
    let t2 = run_one(barrier.clone());
    t1.await.unwrap();
    t2.await.unwrap();
}
