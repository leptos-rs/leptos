use any_spawner::Executor;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

#[test]
fn test_custom_executor() {
    // Define a simple custom executor
    struct TestExecutor {
        spawn_called: Arc<AtomicBool>,
        spawn_local_called: Arc<AtomicBool>,
        poll_local_called: Arc<AtomicBool>,
    }

    impl any_spawner::CustomExecutor for TestExecutor {
        fn spawn(&self, fut: any_spawner::PinnedFuture<()>) {
            self.spawn_called.store(true, Ordering::SeqCst);
            // Execute the future immediately (this works for simple test futures)
            futures::executor::block_on(fut);
        }

        fn spawn_local(&self, fut: any_spawner::PinnedLocalFuture<()>) {
            self.spawn_local_called.store(true, Ordering::SeqCst);
            // Execute the future immediately
            futures::executor::block_on(fut);
        }

        fn poll_local(&self) {
            self.poll_local_called.store(true, Ordering::SeqCst);
        }
    }

    let spawn_called = Arc::new(AtomicBool::new(false));
    let spawn_local_called = Arc::new(AtomicBool::new(false));
    let poll_local_called = Arc::new(AtomicBool::new(false));

    let executor = TestExecutor {
        spawn_called: spawn_called.clone(),
        spawn_local_called: spawn_local_called.clone(),
        poll_local_called: poll_local_called.clone(),
    };

    // Initialize with our custom executor
    Executor::init_custom_executor(executor)
        .expect("Failed to initialize custom executor");

    // Test spawn
    Executor::spawn(async {
        // Simple task
    });
    assert!(spawn_called.load(Ordering::SeqCst));

    // Test spawn_local
    Executor::spawn_local(async {
        // Simple local task
    });
    assert!(spawn_local_called.load(Ordering::SeqCst));

    // Test poll_local
    Executor::poll_local();
    assert!(poll_local_called.load(Ordering::SeqCst));
}
