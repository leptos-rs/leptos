//! Regression tests for `SuppressResourceLoad`.
//!
//! `Resource` evaluates `SuppressResourceLoad` when its fetcher future is
//! polled, while `OnceResource` used to evaluate it once, at construction. A
//! `OnceResource` created inside a suppression scope therefore never loaded —
//! not even after the guard dropped — because the loader task was skipped at
//! construction. The fetch-time check makes the two consistent.
//!
//! The guard used to be process-global, so a guard held on one thread (e.g.
//! by `RouteList::generate`) suppressed resources created by an unrelated
//! render on another thread, which then never loaded. It now only applies to
//! resources created on the thread that holds it.

use any_spawner::Executor;
use leptos_server::{ArcOnceResource, ArcResource, SuppressResourceLoad};
use reactive_graph::owner::Owner;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::Duration,
};

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

#[tokio::test]
async fn guard_on_another_thread_does_not_suppress_this_thread() {
    _ = Executor::init_tokio();
    let owner = Owner::new();
    owner.set();

    // Another thread holds a guard for the whole test, as a concurrent
    // `RouteList::generate` would.
    let (held_tx, held_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel::<()>();
    let generating = thread::spawn(move || {
        let _guard = SuppressResourceLoad::new();
        held_tx.send(()).unwrap();
        _ = release_rx.recv();
    });
    held_rx.recv().unwrap();

    let resource = ArcResource::<u32>::new(|| (), |_| async { 42 });
    let once = ArcOnceResource::<u32>::new(async { 7 });

    // With a process-global guard both resources hung forever here.
    let values = tokio::time::timeout(Duration::from_secs(5), async move {
        (resource.await, once.await)
    })
    .await
    .expect("a guard held on another thread suppressed this thread's loads");

    release_tx.send(()).unwrap();
    generating.join().unwrap();
    assert_eq!(values, (42, 7));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn resources_created_under_a_guard_do_not_load_while_it_is_alive() {
    _ = Executor::init_tokio();
    let owner = Owner::new();
    owner.set();

    let resource_loaded = Arc::new(AtomicBool::new(false));
    let once_loaded = Arc::new(AtomicBool::new(false));

    // Like `RouteList::generate`: hold the guard while synchronously creating
    // resources. The `OnceResource` loader is spawned onto a worker thread
    // that does not hold the guard, and must stay suppressed all the same.
    {
        let _guard = SuppressResourceLoad::new();
        let _resource = ArcResource::<u32>::new(|| (), {
            let loaded = Arc::clone(&resource_loaded);
            move |_| {
                let loaded = Arc::clone(&loaded);
                async move {
                    loaded.store(true, Ordering::SeqCst);
                    1
                }
            }
        });
        let _once = ArcOnceResource::<u32>::new({
            let loaded = Arc::clone(&once_loaded);
            async move {
                loaded.store(true, Ordering::SeqCst);
                2
            }
        });
        // give the workers time to poll the loader while the guard is alive
        thread::sleep(Duration::from_millis(100));
    }

    assert!(!resource_loaded.load(Ordering::SeqCst), "Resource loaded");
    assert!(!once_loaded.load(Ordering::SeqCst), "OnceResource loaded");
}
