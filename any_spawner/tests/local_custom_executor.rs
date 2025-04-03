use any_spawner::Executor;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

#[test]
fn test_local_custom_executor() {
    // Define a thread-local custom executor
    struct LocalTestExecutor {
        spawn_called: Arc<AtomicBool>,
        spawn_local_called: Arc<AtomicBool>,
    }

    impl any_spawner::CustomExecutor for LocalTestExecutor {
        fn spawn(&self, fut: any_spawner::PinnedFuture<()>) {
            self.spawn_called.store(true, Ordering::SeqCst);
            futures::executor::block_on(fut);
        }

        fn spawn_local(&self, fut: any_spawner::PinnedLocalFuture<()>) {
            self.spawn_local_called.store(true, Ordering::SeqCst);
            futures::executor::block_on(fut);
        }

        fn poll_local(&self) {
            // No-op for this test
        }
    }

    let local_spawn_called = Arc::new(AtomicBool::new(false));
    let local_spawn_local_called = Arc::new(AtomicBool::new(false));

    let local_executor = LocalTestExecutor {
        spawn_called: local_spawn_called.clone(),
        spawn_local_called: local_spawn_local_called.clone(),
    };

    // Initialize a thread-local executor
    Executor::init_local_custom_executor(local_executor)
        .expect("Failed to initialize local custom executor");

    // Test spawn - should use the thread-local executor
    Executor::spawn(async {
        // Simple task
    });
    assert!(local_spawn_called.load(Ordering::SeqCst));

    // Test spawn_local - should use the thread-local executor
    Executor::spawn_local(async {
        // Simple local task
    });
    assert!(local_spawn_local_called.load(Ordering::SeqCst));
}
