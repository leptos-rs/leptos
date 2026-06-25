use any_spawner::Executor;
use reactive_graph::traits::IsDisposed;
use reactive_graph::{
    computed::{ArcAsyncDerived, AsyncDerived},
    graph::{AnySource, AnySubscriber, ReactiveNode, Source, Subscriber},
    owner::Owner,
    signal::ArcRwSignal,
    signal::RwSignal,
    traits::{Get, Read, Set, With, WithUntracked},
};
use std::{
    future::pending,
    sync::{Arc, Weak},
};

struct SignalSetOnDirty {
    signal: ArcRwSignal<i32>,
    value: i32,
    fired: std::sync::atomic::AtomicBool,
}

impl IsDisposed for SignalSetOnDirty {
    fn is_disposed(&self) -> bool {
        false
    }
}

impl ReactiveNode for SignalSetOnDirty {
    fn mark_dirty(&self) {
        if !self.fired.swap(true, std::sync::atomic::Ordering::Relaxed) {
            self.signal.set(self.value);
        }
    }
    fn mark_check(&self) {}
    fn mark_subscribers_check(&self) {}
    fn update_if_necessary(&self) -> bool {
        false
    }
}

impl Subscriber for SignalSetOnDirty {
    fn add_source(&self, _: AnySource) {}
    fn clear_sources(&self, _: &AnySubscriber) {}
}

fn make_any_subscriber(sub: Arc<SignalSetOnDirty>) -> AnySubscriber {
    AnySubscriber(
        Arc::as_ptr(&sub) as usize,
        Arc::downgrade(&sub) as Weak<dyn Subscriber + Send + Sync>,
    )
}

#[tokio::test]
async fn arc_async_derived_calculates_eagerly() {
    _ = Executor::init_tokio();
    let owner = Owner::new();
    owner.set();

    let value = ArcAsyncDerived::new(|| async {
        Executor::tick().await;
        42
    });

    assert_eq!(value.clone().await, 42);
}

#[tokio::test]
async fn arc_async_derived_tracks_signal_change() {
    _ = Executor::init_tokio();
    let owner = Owner::new();
    owner.set();

    let signal = RwSignal::new(10);
    let value = ArcAsyncDerived::new(move || async move {
        Executor::tick().await;
        signal.get()
    });

    assert_eq!(value.clone().await, 10);
    signal.set(30);
    Executor::tick().await;
    assert_eq!(value.clone().await, 30);
    signal.set(50);
    Executor::tick().await;
    assert_eq!(value.clone().await, 50);
}

#[tokio::test]
async fn async_derived_calculates_eagerly() {
    _ = Executor::init_tokio();
    let owner = Owner::new();
    owner.set();

    let value = AsyncDerived::new(|| async {
        Executor::tick().await;
        42
    });

    assert_eq!(value.await, 42);
}

#[tokio::test]
async fn async_derived_tracks_signal_change() {
    _ = Executor::init_tokio();
    let owner = Owner::new();
    owner.set();

    let signal = RwSignal::new(10);
    let value = AsyncDerived::new(move || async move {
        Executor::tick().await;
        signal.get()
    });

    assert_eq!(value.await, 10);
    signal.set(30);
    Executor::tick().await;
    assert_eq!(value.await, 30);
    signal.set(50);
    Executor::tick().await;
    assert_eq!(value.await, 50);
}

#[tokio::test]
async fn read_signal_traits_on_arc() {
    _ = Executor::init_tokio();
    let owner = Owner::new();
    owner.set();

    let value = ArcAsyncDerived::new(pending::<()>);
    assert_eq!(value.read(), None);
    assert_eq!(value.with_untracked(|n| *n), None);
    assert_eq!(value.with(|n| *n), None);
    assert_eq!(value.get(), None);
}

#[tokio::test]
async fn read_signal_traits_on_arena() {
    _ = Executor::init_tokio();
    let owner = Owner::new();
    owner.set();

    let value = AsyncDerived::new(pending::<()>);
    println!("{:?}", value.read());
    assert_eq!(value.read(), None);
    assert_eq!(value.with_untracked(|n| *n), None);
    assert_eq!(value.with(|n| *n), None);
    assert_eq!(value.get(), None);
}

#[tokio::test]
async fn async_derived_with_initial() {
    _ = Executor::init_tokio();
    let owner = Owner::new();
    owner.set();

    let signal1 = RwSignal::new(0);
    let signal2 = RwSignal::new(0);
    let derived =
        ArcAsyncDerived::new_with_initial(Some(5), move || async move {
            // reactive values can be tracked anywhere in the `async` block
            let value1 = signal1.get();
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
            let value2 = signal2.get();

            value1 + value2
        });

    // the value can be accessed synchronously as `Option<T>`
    assert_eq!(derived.get(), Some(5));
    // we can also .await the value, i.e., convert it into a Future
    assert_eq!(derived.clone().await, 0);
    assert_eq!(derived.get(), Some(0));

    signal1.set(1);
    // while the new value is still pending, the signal holds the old value
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    assert_eq!(derived.get(), Some(0));

    // setting multiple dependencies will hold until the latest change is ready
    signal2.set(1);
    assert_eq!(derived.await, 2);
}

/// Demonstrates that a source change arriving during the notify_subs window
/// (while ArcAsyncDerived temporarily holds state=Notifying) is not silently
/// dropped.  Before the fix, mark_dirty skipped both setting Dirty and calling
/// the channel notifier when it observed Notifying, and notify_subs then
/// restored the previous Clean state, leaving the derived permanently stale.
#[tokio::test]
async fn arc_async_derived_notify_window_not_dropped() {
    _ = Executor::init_tokio();
    let owner = Owner::new();
    owner.set();

    let signal = ArcRwSignal::new(1_i32);

    // derived just reads the signal
    let signal_c = signal.clone();
    let derived: ArcAsyncDerived<i32> = ArcAsyncDerived::new(move || {
        let s = signal_c.clone();
        async move { s.get() }
    });

    // wait for the initial computation (signal == 1)
    assert_eq!(derived.clone().await, 1);

    // Register a downstream subscriber that, when notified, fires signal.set(3).
    // This simulates a source change arriving during the notify_subs window:
    // notify_subs sets state=Notifying, calls sub.mark_dirty() on our probe,
    // the probe calls signal.set(3) which calls mark_dirty on the derived while
    // state is still Notifying.  Without the fix that mark_dirty is a no-op and
    // the derived stays stale at 2 forever.
    let probe = Arc::new(SignalSetOnDirty {
        signal: signal.clone(),
        value: 3,
        fired: std::sync::atomic::AtomicBool::new(false),
    });
    derived.add_subscriber(make_any_subscriber(probe.clone()));

    // Trigger a recomputation: signal -> 2.
    signal.set(2);
    Executor::tick().await;
    Executor::tick().await;

    // The derived must have re-run for signal==3.  Before the fix it returned 2.
    assert_eq!(
        derived.clone().await,
        3,
        "derived did not re-run after a source changed during the notify window"
    );
}
