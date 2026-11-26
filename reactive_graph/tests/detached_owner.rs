//! An effect whose owner was detached by its parent's cleanup must not run again.
//!
//! A view that outlives the owner it was built in — the outgoing view a pending
//! `Suspend` keeps mounted — still holds its render effects, and they are still
//! subscribed to their sources. If a source that outlives that owner changes, the
//! effect would run and read the values the cleanup disposed.

#[cfg(feature = "effects")]
pub mod imports {
    pub use any_spawner::Executor;
    pub use reactive_graph::{
        effect::{Effect, ImmediateEffect, RenderEffect},
        owner::{Owner, on_cleanup},
        prelude::*,
        signal::RwSignal,
        wrappers::read::Signal,
    };
    pub use std::sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    };
    pub use tokio::task;
}

/// The arena slots of the branch die with its cleanup; an effect built in the
/// branch and subscribed to a longer-lived signal must stop with them.
#[cfg(feature = "effects")]
#[tokio::test]
async fn render_effect_of_a_cleaned_up_branch_does_not_rerun() {
    use imports::*;

    _ = Executor::init_tokio();
    let root = Owner::new();
    root.set();
    task::LocalSet::new()
        .run_until(async {
            let outlives_branch = RwSignal::new(0);
            let runs = Arc::new(AtomicUsize::new(0));
            let reads = Arc::new(Mutex::new(Vec::new()));

            let branch = root.child();
            let effect = branch.with(|| {
                // What `#[prop(into)] Signal<T>` does with a static value at the
                // call site: an arena slot in the branch.
                let prop = Signal::stored("icon");
                let runs = Arc::clone(&runs);
                let reads = Arc::clone(&reads);
                RenderEffect::new(move |_| {
                    outlives_branch.track();
                    runs.fetch_add(1, Ordering::SeqCst);
                    reads.lock().unwrap().push(prop.try_get());
                })
            });
            Executor::tick().await;
            assert_eq!(runs.load(Ordering::SeqCst), 1);

            branch.cleanup();
            outlives_branch.set(1);
            Executor::tick().await;

            assert_eq!(
                *reads.lock().unwrap(),
                vec![Some("icon")],
                "the effect read a value its branch had already disposed"
            );
            assert_eq!(runs.load(Ordering::SeqCst), 1);
            drop(effect);
        })
        .await;
}

/// The same holds for an `Effect`, which the owner keeps alive itself.
#[cfg(feature = "effects")]
#[tokio::test]
async fn effect_of_a_cleaned_up_branch_does_not_rerun() {
    use imports::*;

    _ = Executor::init_tokio();
    let root = Owner::new();
    root.set();
    task::LocalSet::new()
        .run_until(async {
            let outlives_branch = RwSignal::new(0);
            let runs = Arc::new(AtomicUsize::new(0));

            let branch = root.child();
            branch.with(|| {
                let runs = Arc::clone(&runs);
                Effect::new(move || {
                    outlives_branch.track();
                    runs.fetch_add(1, Ordering::SeqCst);
                });
            });
            Executor::tick().await;
            assert_eq!(runs.load(Ordering::SeqCst), 1);

            branch.cleanup();
            outlives_branch.set(1);
            Executor::tick().await;
            assert_eq!(runs.load(Ordering::SeqCst), 1);
        })
        .await;
}

/// `with_cleanup` cleans the effect's own owner before every rerun. That owner
/// stays live: only the children it detaches are frozen, and the children the
/// rerun creates run normally.
#[cfg(feature = "effects")]
#[tokio::test]
async fn an_effect_keeps_rerunning_and_its_new_children_run() {
    use imports::*;

    _ = Executor::init_tokio();
    let root = Owner::new();
    root.set();
    task::LocalSet::new()
        .run_until(async {
            let outer_source = RwSignal::new(0);
            let inner_source = RwSignal::new(0);
            let outer_runs = Arc::new(AtomicUsize::new(0));
            let inner_runs = Arc::new(Mutex::new(Vec::new()));

            let outer = RenderEffect::new({
                let outer_runs = Arc::clone(&outer_runs);
                let inner_runs = Arc::clone(&inner_runs);
                move |_| {
                    let generation = outer_source.get();
                    outer_runs.fetch_add(1, Ordering::SeqCst);
                    let inner_runs = Arc::clone(&inner_runs);
                    // A nested binding, rebuilt on every outer run.
                    RenderEffect::new(move |_| {
                        inner_source.track();
                        inner_runs.lock().unwrap().push(generation);
                    })
                }
            });
            Executor::tick().await;

            outer_source.set(1);
            Executor::tick().await;
            outer_source.set(2);
            Executor::tick().await;
            assert_eq!(outer_runs.load(Ordering::SeqCst), 3);

            inner_source.set(1);
            Executor::tick().await;
            assert_eq!(
                *inner_runs.lock().unwrap(),
                vec![0, 1, 2, 2],
                "only the current nested binding may react"
            );
            drop(outer);
        })
        .await;
}

/// An owner that is cleaned up directly, not through its parent, can be used again.
#[cfg(feature = "effects")]
#[tokio::test]
async fn an_owner_cleaned_up_directly_is_not_paused() {
    use imports::*;

    _ = Executor::init_tokio();
    let root = Owner::new();
    root.set();
    task::LocalSet::new()
        .run_until(async {
            let source = RwSignal::new(0);
            let runs = Arc::new(AtomicUsize::new(0));

            let branch = root.child();
            branch.cleanup();
            assert!(!branch.paused());

            branch.with(|| {
                let runs = Arc::clone(&runs);
                Effect::new(move || {
                    source.track();
                    runs.fetch_add(1, Ordering::SeqCst);
                });
            });
            Executor::tick().await;
            source.set(1);
            Executor::tick().await;
            assert_eq!(runs.load(Ordering::SeqCst), 2);
        })
        .await;
}

/// A cleanup can write a signal, and an `ImmediateEffect` runs as soon as it is
/// notified. Every owner a cleanup detaches is paused before any cleanup runs, so
/// a sibling's cleanup cannot wake a detached effect that is not torn down yet.
#[cfg(feature = "effects")]
#[tokio::test]
async fn a_sibling_cleanup_does_not_wake_a_detached_immediate_effect() {
    use imports::*;

    _ = Executor::init_tokio();
    let root = Owner::new();
    root.set();
    task::LocalSet::new()
        .run_until(async {
            let source = RwSignal::new(0);
            let runs = Arc::new(AtomicUsize::new(0));
            let scope_cleaned_up = Arc::new(AtomicUsize::new(0));

            let branch = root.child();
            let (scope, effect) = branch.with(|| {
                // A component scope that writes `source` when it is torn down,
                // kept alive as a mounted view keeps its scope.
                let scope = Owner::new();
                let scope_cleaned_up = Arc::clone(&scope_cleaned_up);
                scope.with(|| {
                    on_cleanup(move || {
                        scope_cleaned_up.fetch_add(1, Ordering::SeqCst);
                        source.set(1);
                    })
                });
                let runs = Arc::clone(&runs);
                let effect = ImmediateEffect::new(move || {
                    source.track();
                    runs.fetch_add(1, Ordering::SeqCst);
                });
                (scope, effect)
            });
            assert_eq!(scope_cleaned_up.load(Ordering::SeqCst), 0);
            assert_eq!(runs.load(Ordering::SeqCst), 1);

            root.cleanup();
            assert_eq!(
                runs.load(Ordering::SeqCst),
                1,
                "a detached effect ran during the teardown"
            );
            assert_eq!(
                scope_cleaned_up.load(Ordering::SeqCst),
                1,
                "the sibling's cleanup ran"
            );
            drop((scope, effect));
        })
        .await;
}

/// The same for a cleanup registered by the effect itself.
#[cfg(feature = "effects")]
#[tokio::test]
async fn an_immediate_effect_does_not_rerun_from_its_own_cleanup() {
    use imports::*;

    _ = Executor::init_tokio();
    let root = Owner::new();
    root.set();
    task::LocalSet::new()
        .run_until(async {
            let source = RwSignal::new(0);
            let runs = Arc::new(AtomicUsize::new(0));

            let branch = root.child();
            let effect = branch.with(|| {
                let runs = Arc::clone(&runs);
                ImmediateEffect::new(move || {
                    source.track();
                    runs.fetch_add(1, Ordering::SeqCst);
                    on_cleanup(move || source.set(1));
                })
            });
            assert_eq!(runs.load(Ordering::SeqCst), 1);

            root.cleanup();
            assert_eq!(
                runs.load(Ordering::SeqCst),
                1,
                "a detached effect ran during its own teardown"
            );
            drop(effect);
        })
        .await;
}

/// An owner that is dropped instead of cleaned up lets go of its children too:
/// the arena slots it held are removed, and an effect still held elsewhere must
/// not run against them.
#[cfg(feature = "effects")]
#[tokio::test]
async fn a_render_effect_under_a_dropped_owner_does_not_rerun() {
    use imports::*;

    _ = Executor::init_tokio();
    let root = Owner::new();
    root.set();
    task::LocalSet::new()
        .run_until(async {
            let outlives_branch = RwSignal::new(0);
            let reads = Arc::new(Mutex::new(Vec::new()));

            let branch = root.child();
            let effect = branch.with(|| {
                let prop = Signal::stored("icon");
                let reads = Arc::clone(&reads);
                RenderEffect::new(move |_| {
                    outlives_branch.track();
                    reads.lock().unwrap().push(prop.try_get());
                })
            });
            Executor::tick().await;

            drop(branch);
            outlives_branch.set(1);
            Executor::tick().await;

            assert_eq!(
                *reads.lock().unwrap(),
                vec![Some("icon")],
                "the effect read a value its dropped owner had disposed"
            );
            drop(effect);
        })
        .await;
}
