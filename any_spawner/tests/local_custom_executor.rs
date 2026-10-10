use any_spawner::{CustomExecutor, Executor, PinnedFuture, PinnedLocalFuture};
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    thread,
};

// A custom executor that counts how many futures it is handed and runs them to
// completion. The same type is used both as the global executor and as a
// thread-local override so we can observe which one a spawn was routed to.
struct CountingExecutor {
    count: Arc<AtomicUsize>,
}

impl CustomExecutor for CountingExecutor {
    fn spawn(&self, fut: PinnedFuture<()>) {
        self.count.fetch_add(1, Ordering::SeqCst);
        futures::executor::block_on(fut);
    }

    fn spawn_local(&self, fut: PinnedLocalFuture<()>) {
        self.count.fetch_add(1, Ordering::SeqCst);
        futures::executor::block_on(fut);
    }

    fn poll_local(&self) {}
}

// Exercises the documented contract of `init_local_custom_executor`: it
// overrides the executor for spawns made *on the current thread only*, without
// affecting other threads or the global executor.
#[test]
fn local_executor_overrides_global_only_on_its_thread() {
    let global_count = Arc::new(AtomicUsize::new(0));
    let local_count = Arc::new(AtomicUsize::new(0));

    // Global executor used by every thread that has no local override.
    Executor::init_custom_executor(CountingExecutor {
        count: global_count.clone(),
    })
    .expect("global executor should initialize");

    // Thread-local override for the current thread only. This must succeed even
    // though a global executor is already set, because the two are independent.
    Executor::init_local_custom_executor(CountingExecutor {
        count: local_count.clone(),
    })
    .expect("local override should initialize");

    // On this thread, both spawn and spawn_local route to the LOCAL executor.
    Executor::spawn(async {});
    Executor::spawn_local(async {});
    assert_eq!(local_count.load(Ordering::SeqCst), 2);
    assert_eq!(global_count.load(Ordering::SeqCst), 0);

    // On another thread (no local override) a spawn must fall through to the
    // GLOBAL executor and must NOT panic. Before the thread-local dispatch fix,
    // the global table pointed at an unset thread-local instance and this
    // panicked with `Option::unwrap()` on `None`.
    let global_count_thread = global_count.clone();
    thread::spawn(move || {
        Executor::spawn(async {});
        assert_eq!(global_count_thread.load(Ordering::SeqCst), 1);
    })
    .join()
    .expect("spawn on a different thread must not panic");

    // The other thread never touched the local executor.
    assert_eq!(local_count.load(Ordering::SeqCst), 2);
}
