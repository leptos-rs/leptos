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

/// A transition is active only while its future is being polled.
#[tokio::test]
async fn transition_is_active_only_while_it_is_polled() {
    _ = Executor::init_tokio();
    let owner = Owner::new();
    owner.set();

    assert!(!AsyncTransition::is_active());
    let during = AsyncTransition::run(|| async {
        let before_await = AsyncTransition::is_active();
        Executor::tick().await;
        before_await && AsyncTransition::is_active()
    })
    .await;
    assert!(during);
    assert!(!AsyncTransition::is_active());
}

/// A resource keyed on `source`, which waits for `gate` before loading any
/// value but the first.
fn gated_resource(
    source: &ArcRwSignal<u32>,
    gate: &Arc<Notify>,
) -> ArcAsyncDerived<u32> {
    let source = source.clone();
    let gate = gate.clone();
    ArcAsyncDerived::new(move || {
        let value = source.get();
        let gate = gate.clone();
        async move {
            if value > 1 {
                gate.notified().await;
            }
            value
        }
    })
}

/// `track` waits for a resource that reloads because a signal it depends on
/// was set during the tracked action, although the reload itself only starts
/// once the action has returned.
#[tokio::test]
async fn track_waits_for_a_reload_the_action_causes() {
    _ = Executor::init_tokio();
    let owner = Owner::new();
    owner.set();

    let source = ArcRwSignal::new(1);
    let gate = Arc::new(Notify::new());
    let derived = gated_resource(&source, &gate);
    assert_eq!(derived.clone().await, 1);

    let ((), reloaded) = AsyncTransition::track(|| source.set(2));
    let mut reloaded = pin!(reloaded);
    tokio::select! {
        _ = &mut reloaded => panic!("track resolved before the reload finished"),
        _ = tokio::time::sleep(Duration::from_millis(50)) => {}
    }
    gate.notify_one();
    reloaded.await;
    assert_eq!(derived.get_untracked(), Some(2));
}

/// Two tracked updates before the resource's worker has handled the first one
/// both wait for the reload they cause.
#[tokio::test]
async fn two_tracked_updates_both_wait_for_the_reload() {
    _ = Executor::init_tokio();
    let owner = Owner::new();
    owner.set();

    let source = ArcRwSignal::new(1);
    let gate = Arc::new(Notify::new());
    let derived = gated_resource(&source, &gate);
    assert_eq!(derived.clone().await, 1);

    let ((), first) = AsyncTransition::track(|| source.set(2));
    let ((), second) = AsyncTransition::track(|| source.set(3));
    // dropping one of them must not affect the other
    drop(first);
    let mut second = pin!(second);
    tokio::select! {
        _ = &mut second => panic!("track resolved before the reload finished"),
        _ = tokio::time::sleep(Duration::from_millis(50)) => {}
    }
    gate.notify_one();
    second.await;
    assert_eq!(derived.get_untracked(), Some(3));
}

/// A tracked update that notifies a resource before its worker has run for
/// the first time waits for the load that follows.
#[tokio::test]
async fn track_waits_for_a_resource_that_has_not_loaded_yet() {
    _ = Executor::init_tokio();
    let owner = Owner::new();
    owner.set();

    let source = ArcRwSignal::new(1);
    let gate = Arc::new(Notify::new());
    let derived = ArcAsyncDerived::new({
        let source = source.clone();
        let gate = gate.clone();
        move || {
            let value = source.get();
            let gate = gate.clone();
            async move {
                // not ready at once, so that the first run is a real load
                Executor::tick().await;
                if value > 1 {
                    gate.notified().await;
                }
                value
            }
        }
    });
    let ((), reloaded) = AsyncTransition::track(|| source.set(2));
    let mut reloaded = pin!(reloaded);
    tokio::select! {
        _ = &mut reloaded => panic!("track resolved before the load finished"),
        _ = tokio::time::sleep(Duration::from_millis(50)) => {}
    }
    gate.notify_one();
    reloaded.await;
    assert_eq!(derived.get_untracked(), Some(2));
}

/// A node that no worker handles (a server-side mock) is never waited for.
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
    let ((), reloaded) = AsyncTransition::track(|| source.set(2));
    tokio::time::timeout(Duration::from_secs(1), reloaded)
        .await
        .expect("track waited for a resource that has no worker");
}

/// A notification that does not require a reload (here, through a memo whose
/// value does not change) is not waited for.
#[tokio::test]
async fn track_does_not_wait_for_a_notification_without_a_reload() {
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

    let ((), reloaded) = AsyncTransition::track(|| source.set(15));
    tokio::time::timeout(Duration::from_secs(1), reloaded)
        .await
        .expect("track waited for a reload that never started");
    assert_eq!(runs.load(Ordering::SeqCst), 1);
}

/// `run` only waits for the resources created during it: a resource that
/// already exists, and that a signal set during it notifies, is not waited
/// for.
#[tokio::test]
async fn run_does_not_wait_for_a_resource_it_only_notifies() {
    _ = Executor::init_tokio();
    let owner = Owner::new();
    owner.set();

    let source = ArcRwSignal::new(1);
    let gate = Arc::new(Notify::new());
    let derived = gated_resource(&source, &gate);
    assert_eq!(derived.clone().await, 1);

    let transition = AsyncTransition::run(|| {
        let source = source.clone();
        async move { source.set(2) }
    });
    tokio::time::timeout(Duration::from_secs(1), transition)
        .await
        .expect("run waited for a resource it did not create");
    gate.notify_one();
}
