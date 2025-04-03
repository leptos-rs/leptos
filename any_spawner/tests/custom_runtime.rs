#![cfg(feature = "futures-executor")]

use any_spawner::{CustomExecutor, Executor, PinnedFuture, PinnedLocalFuture};

#[test]
fn can_create_custom_executor() {
    use futures::{
        executor::{LocalPool, LocalSpawner},
        task::LocalSpawnExt,
    };
    use std::{
        cell::RefCell,
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        },
    };

    thread_local! {
        static LOCAL_POOL: RefCell<LocalPool> = RefCell::new(LocalPool::new());
        static SPAWNER: LocalSpawner = LOCAL_POOL.with(|pool| pool.borrow().spawner());
    }

    struct CustomFutureExecutor;
    impl CustomExecutor for CustomFutureExecutor {
        fn spawn(&self, _fut: PinnedFuture<()>) {
            panic!("not supported in this test");
        }

        fn spawn_local(&self, fut: PinnedLocalFuture<()>) {
            SPAWNER.with(|spawner| {
                spawner.spawn_local(fut).expect("failed to spawn future");
            });
        }

        fn poll_local(&self) {
            LOCAL_POOL.with(|pool| {
                if let Ok(mut pool) = pool.try_borrow_mut() {
                    pool.run_until_stalled();
                }
                // If we couldn't borrow_mut, we're in a nested call to poll, so we don't need to do anything.
            });
        }
    }

    Executor::init_custom_executor(CustomFutureExecutor)
        .expect("couldn't set executor");

    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = Arc::clone(&counter);
    Executor::spawn_local(async move {
        counter_clone.store(1, Ordering::Release);
    });
    Executor::poll_local();
    assert_eq!(counter.load(Ordering::Acquire), 1);
}
