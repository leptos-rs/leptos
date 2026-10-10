#![cfg(feature = "tracing")]

use any_spawner::Executor;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tracing::{
    Event, Level, Metadata, Subscriber,
    span::{Attributes, Id, Record},
};

// Minimal subscriber that counts ERROR-level events emitted with the
// `any_spawner` target.
struct CountingSubscriber(Arc<AtomicUsize>);

impl Subscriber for CountingSubscriber {
    fn enabled(&self, _: &Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, _: &Attributes<'_>) -> Id {
        Id::from_u64(1)
    }

    fn record(&self, _: &Id, _: &Record<'_>) {}

    fn record_follows_from(&self, _: &Id, _: &Id) {}

    fn event(&self, event: &Event<'_>) {
        let meta = event.metadata();
        if meta.target() == "any_spawner" && *meta.level() == Level::ERROR {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn enter(&self, _: &Id) {}

    fn exit(&self, _: &Id) {}
}

// With the `tracing` feature enabled, spawning before any executor is
// initialized must emit an ERROR diagnostic and drop the task instead of
// panicking. Crucially this must hold in release builds, where the diagnostic
// was previously cfg-gated behind `debug_assertions` and never emitted.
#[test]
fn uninitialized_spawn_emits_tracing_diagnostic() {
    let count = Arc::new(AtomicUsize::new(0));
    let subscriber = CountingSubscriber(count.clone());

    tracing::subscriber::with_default(subscriber, || {
        // No executor is initialized in this test binary.
        Executor::spawn(async {});
        Executor::spawn_local(async {});
    });

    assert_eq!(
        count.load(Ordering::SeqCst),
        2,
        "spawn and spawn_local must each emit an ERROR diagnostic when no \
         executor is initialized"
    );
}
