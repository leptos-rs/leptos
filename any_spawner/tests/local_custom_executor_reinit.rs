use any_spawner::{
    CustomExecutor, Executor, ExecutorError, PinnedFuture, PinnedLocalFuture,
};
use std::{cell::Cell, rc::Rc};

// Tracks whether the executor it is embedded in has been dropped.
struct DropProbe(Rc<Cell<bool>>);

impl Drop for DropProbe {
    fn drop(&mut self) {
        self.0.set(true);
    }
}

struct ProbeExecutor(#[allow(dead_code)] DropProbe);

impl CustomExecutor for ProbeExecutor {
    fn spawn(&self, _: PinnedFuture<()>) {}
    fn spawn_local(&self, _: PinnedLocalFuture<()>) {}
    fn poll_local(&self) {}
}

// A second `init_local_custom_executor` on the same thread must be rejected
// with `AlreadySet`, and the rejected executor must be dropped rather than
// pinned in thread-local state (which would leak it and leave the API
// permanently unusable on the thread).
#[test]
fn reinitializing_local_executor_is_rejected_without_residue() {
    struct Noop;
    impl CustomExecutor for Noop {
        fn spawn(&self, _: PinnedFuture<()>) {}
        fn spawn_local(&self, _: PinnedLocalFuture<()>) {}
        fn poll_local(&self) {}
    }

    Executor::init_local_custom_executor(Noop)
        .expect("first local initialization should succeed");

    let dropped = Rc::new(Cell::new(false));
    let result = Executor::init_local_custom_executor(ProbeExecutor(
        DropProbe(dropped.clone()),
    ));

    assert!(matches!(result, Err(ExecutorError::AlreadySet)));
    assert!(
        dropped.get(),
        "the rejected executor must be dropped, not pinned in thread-local \
         state"
    );
}
