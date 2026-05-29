//! Regression guard for the poll/wake handshake in `OnceResourceFuture`.
//!
//! The loader task publishes the value, clears the `loading` flag and then
//! drains the registered wakers. The poll side has to register its waker and
//! check `loading` atomically with respect to that drain; otherwise a waker
//! registered just after the (empty) drain is never woken and the future hangs
//! forever. This lost-wake interleaving only exists with genuine multi-thread
//! scheduling, hence the multi-thread runtime with a fixed worker count (so it
//! behaves the same regardless of the host's core count).
//!
//! On correct code every resource resolves, so the test is deterministic: it
//! never depends on timing, and the timeout is only ever reached if a wake-up
//! is genuinely lost.

use any_spawner::Executor;
use leptos_server::ArcOnceResource;
use reactive_graph::owner::Owner;
use std::time::Duration;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn once_resource_resolves_across_threads() {
    _ = Executor::init_tokio();
    let owner = Owner::new();
    owner.set();

    for i in 0..256u32 {
        // The loader runs as its own task (spawned by `ArcOnceResource::new`),
        // so on a multi-thread runtime it races against this awaiter on a
        // separate worker, exercising the check-then-register handshake.
        let resource = ArcOnceResource::<u32>::new(async move {
            // Yield once so the awaiter typically reaches its `loading` check
            // before the loader publishes its value.
            Executor::tick().await;
            i
        });

        let value = tokio::time::timeout(Duration::from_secs(5), async move {
            resource.await
        })
        .await
        .expect("OnceResource await hung (lost wake-up)");

        assert_eq!(value, i);
    }
}
