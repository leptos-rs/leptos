//! Regression tests for `AsyncTransition`.
//!
//! A transition must wait for the async resources created during it, and two
//! transitions that overlap in time must not observe one another's
//! registration slot.

use any_spawner::Executor;
use reactive_graph::{
    computed::{ArcAsyncDerived, ArcMemo},
    owner::Owner,
    signal::ArcRwSignal,
    traits::{Get, GetUntracked, Set},
    transition::AsyncTransition,
};
use std::{
    pin::pin,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio::sync::{Barrier, Notify};

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

            // The resource is created but not awaited inside the action, so
            // only the transition's own registration can make `run` wait.
            let derived = AsyncTransition::run(move || async move {
                // Force the two transitions to be "open" simultaneously.
                barrier.wait().await;
                ArcAsyncDerived::new(move || {
                    let flag = flag.clone();
                    async move {
                        Executor::tick().await;
                        Executor::tick().await;
                        flag.store(true, Ordering::SeqCst);
                        7_u32
                    }
                })
            })
            .await;

            assert!(
                resolved.load(Ordering::SeqCst),
                "transition returned before its own resource resolved"
            );
            assert_eq!(derived.await, 7);
        })
    }

    let barrier = Arc::new(Barrier::new(2));
    let t1 = run_one(barrier.clone());
    let t2 = run_one(barrier.clone());
    t1.await.unwrap();
    t2.await.unwrap();
}

/// `track` must wait for a resource that reloads because a signal it depends
/// on was set inside the tracked action, even though the reload itself only
/// starts after the action has returned.
#[tokio::test]
async fn track_waits_for_resource_reloaded_by_action() {
    _ = Executor::init_tokio();
    let owner = Owner::new();
    owner.set();

    let source = ArcRwSignal::new(1_u32);
    let reloaded = Arc::new(AtomicBool::new(false));
    let derived = ArcAsyncDerived::new({
        let source = source.clone();
        let reloaded = reloaded.clone();
        move || {
            let value = source.get();
            let reloaded = reloaded.clone();
            async move {
                Executor::tick().await;
                Executor::tick().await;
                if value > 1 {
                    reloaded.store(true, Ordering::SeqCst);
                }
                value * 10
            }
        }
    });
    assert_eq!(derived.clone().await, 10);

    let ((), pending) = AsyncTransition::track(|| source.set(2));
    assert!(!reloaded.load(Ordering::SeqCst));
    pending.await;
    assert!(
        reloaded.load(Ordering::SeqCst),
        "track returned before the reload it caused had finished"
    );
    assert_eq!(derived.get_untracked(), Some(20));
}

/// Two tracked updates to the same resource before its worker has run must
/// both wait for the reload they cause.
#[tokio::test]
async fn two_tracked_updates_both_wait_for_the_reload() {
    _ = Executor::init_tokio();
    let owner = Owner::new();
    owner.set();

    let source = ArcRwSignal::new(1_u32);
    let reloaded = Arc::new(AtomicBool::new(false));
    let derived = ArcAsyncDerived::new({
        let source = source.clone();
        let reloaded = reloaded.clone();
        move || {
            let value = source.get();
            let reloaded = reloaded.clone();
            async move {
                Executor::tick().await;
                Executor::tick().await;
                if value > 1 {
                    reloaded.store(true, Ordering::SeqCst);
                }
                value
            }
        }
    });
    assert_eq!(derived.clone().await, 1);

    let ((), first) = AsyncTransition::track(|| source.set(2));
    let ((), second) = AsyncTransition::track(|| source.set(3));
    // dropping one waiter must not affect the other
    drop(first);
    second.await;
    assert!(
        reloaded.load(Ordering::SeqCst),
        "the second transition returned before the reload it caused finished"
    );
    assert_eq!(derived.get_untracked(), Some(3));
}

/// A resource created outside any transition, then notified by a tracked
/// update before its worker has run for the first time, must make `track`
/// wait for the load that update causes.
#[tokio::test]
async fn tracked_update_before_first_run_waits_for_the_reload() {
    _ = Executor::init_tokio();
    let owner = Owner::new();
    owner.set();

    let source = ArcRwSignal::new(1_u32);
    let gate = Arc::new(Notify::new());
    let derived = ArcAsyncDerived::new({
        let source = source.clone();
        let gate = gate.clone();
        move || {
            let value = source.get();
            let gate = gate.clone();
            async move {
                // not ready synchronously, so the first run is a real load
                Executor::tick().await;
                if value > 1 {
                    gate.notified().await;
                }
                value
            }
        }
    });
    // no yield: the derived's worker has not run yet
    let ((), pending) = AsyncTransition::track(|| source.set(2));
    let mut pending = pin!(pending);
    // give the worker time to start the reload, which then blocks on the gate
    tokio::select! {
        _ = &mut pending => panic!("track completed before the reload it caused had finished"),
        _ = tokio::time::sleep(Duration::from_millis(100)) => {}
    }
    gate.notify_one();
    pending.await;
    assert_eq!(derived.get_untracked(), Some(2));
}

/// A node with no worker to handle notifications (a server-side mock) must
/// not register loads with a transition, which would otherwise wait forever.
#[tokio::test]
async fn track_does_not_wait_for_a_mock_resource() {
    _ = Executor::init_tokio();
    let owner = Owner::new();
    owner.set();

    let source = ArcRwSignal::new(1_u32);
    let _derived = ArcAsyncDerived::new_mock({
        let source = source.clone();
        move || {
            let value = source.get();
            async move { value }
        }
    });
    let ((), pending) = AsyncTransition::track(|| source.set(2));
    tokio::time::timeout(Duration::from_secs(1), pending)
        .await
        .expect("track waited on a resource that has no worker");
}

/// A signal update that notifies a resource without changing what it reads
/// (here, through a memo whose value stays the same) must not leave `track`
/// waiting.
#[tokio::test]
async fn track_does_not_wait_for_a_notification_without_reload() {
    _ = Executor::init_tokio();
    let owner = Owner::new();
    owner.set();

    let source = ArcRwSignal::new(10_u32);
    let tens = ArcMemo::new({
        let source = source.clone();
        move |_| source.get() / 10
    });
    let runs = Arc::new(AtomicUsize::new(0));
    let derived = ArcAsyncDerived::new({
        let tens = tens.clone();
        let runs = runs.clone();
        move || {
            let value = tens.get();
            runs.fetch_add(1, Ordering::SeqCst);
            async move { value }
        }
    });
    assert_eq!(derived.clone().await, 1);
    assert_eq!(runs.load(Ordering::SeqCst), 1);

    // the memo is notified but recomputes to the same value, so the derived
    // checks its sources and does not reload
    let ((), pending) = AsyncTransition::track(|| source.set(15));
    tokio::time::timeout(Duration::from_secs(1), pending)
        .await
        .expect("track waited for a reload that never started");
    assert_eq!(runs.load(Ordering::SeqCst), 1);
    assert_eq!(derived.await, 1);
}
