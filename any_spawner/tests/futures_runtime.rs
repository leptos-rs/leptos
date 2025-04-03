#![cfg(feature = "futures-executor")]

use any_spawner::Executor;
// All tests in this file use the same executor.

#[test]
fn can_spawn_local_future() {
    use std::rc::Rc;

    let _ = Executor::init_futures_executor();
    let rc = Rc::new(());
    Executor::spawn_local(async {
        _ = rc;
    });
    Executor::spawn(async {});
}

#[test]
fn can_make_local_progress() {
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    let _ = Executor::init_futures_executor();
    let counter = Arc::new(AtomicUsize::new(0));
    Executor::spawn_local({
        let counter = Arc::clone(&counter);
        async move {
            assert_eq!(counter.fetch_add(1, Ordering::AcqRel), 0);
            Executor::spawn_local(async {
                // Should not crash
            });
        }
    });
    Executor::poll_local();
    assert_eq!(counter.load(Ordering::Acquire), 1);
}
