use any_spawner::{Executor, ExecutorError};

#[test]
fn test_already_set_error() {
    struct SimpleExecutor;

    impl any_spawner::CustomExecutor for SimpleExecutor {
        fn spawn(&self, _fut: any_spawner::PinnedFuture<()>) {}
        fn spawn_local(&self, _fut: any_spawner::PinnedLocalFuture<()>) {}
        fn poll_local(&self) {}
    }

    // First initialization should succeed
    Executor::init_custom_executor(SimpleExecutor)
        .expect("First initialization failed");

    // Second initialization should fail with AlreadySet error
    let result = Executor::init_custom_executor(SimpleExecutor);
    assert!(matches!(result, Err(ExecutorError::AlreadySet)));

    // A thread-local override is independent of the global executor, so the
    // first local initialization on this thread succeeds even though a global
    // executor has already been set.
    let result = Executor::init_local_custom_executor(SimpleExecutor);
    assert!(result.is_ok());

    // A second local initialization on the same thread fails, since a local
    // executor has already been set for this thread.
    let result = Executor::init_local_custom_executor(SimpleExecutor);
    assert!(matches!(result, Err(ExecutorError::AlreadySet)));
}
