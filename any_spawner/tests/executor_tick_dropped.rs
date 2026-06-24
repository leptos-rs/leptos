use any_spawner::{CustomExecutor, Executor, PinnedFuture, PinnedLocalFuture};

// An executor that drops every future it is handed instead of running it.
struct DropAll;

impl CustomExecutor for DropAll {
    fn spawn(&self, _fut: PinnedFuture<()>) {}
    fn spawn_local(&self, _fut: PinnedLocalFuture<()>) {}
    fn poll_local(&self) {}
}

// If the executor drops spawned futures, the tick-synchronization task never
// runs and `tick()` can never observe an executor cycle. It must surface this
// loudly instead of returning as though a tick had elapsed.
#[test]
#[should_panic(expected = "could not synchronize with the executor")]
fn tick_panics_when_sync_task_is_dropped() {
    Executor::init_custom_executor(DropAll).expect("init");
    futures::executor::block_on(Executor::tick());
}
