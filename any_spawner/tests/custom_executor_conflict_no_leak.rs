#![cfg(feature = "tokio")]

use any_spawner::{
    CustomExecutor, Executor, ExecutorError, PinnedFuture, PinnedLocalFuture,
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

// Flips a flag when dropped, so we can detect whether the executor it is
// embedded in was dropped or leaked into static memory.
struct DropProbe(Arc<AtomicBool>);

impl Drop for DropProbe {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

struct ProbeExecutor(#[allow(dead_code)] DropProbe);

impl CustomExecutor for ProbeExecutor {
    fn spawn(&self, _: PinnedFuture<()>) {}
    fn spawn_local(&self, _: PinnedLocalFuture<()>) {}
    fn poll_local(&self) {}
}

// When the global executor slot is already claimed (here by tokio, which does
// not touch the custom-executor instance slot), `init_custom_executor` must
// fail *without* storing the rejected executor in the static instance slot —
// otherwise that executor is leaked for the life of the process.
#[test]
fn rejected_custom_executor_is_not_leaked() {
    Executor::init_tokio().expect("init tokio");

    let dropped = Arc::new(AtomicBool::new(false));
    let result = Executor::init_custom_executor(ProbeExecutor(DropProbe(
        dropped.clone(),
    )));

    assert!(matches!(result, Err(ExecutorError::AlreadySet)));
    assert!(
        dropped.load(Ordering::SeqCst),
        "rejected custom executor must be dropped, not leaked into static \
         memory"
    );
}
