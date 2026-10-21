use any_spawner::Executor;
use reactive_graph::{
    computed::{ArcAsyncDerived, AsyncDerived},
    owner::Owner,
    signal::RwSignal,
    traits::{Get, Read, Set, With, WithUntracked},
};
use std::future::pending;

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

#[tokio::test]
async fn async_derived_stops_polling_when_owner_disposed() {
    use futures::channel::oneshot;
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    };

    _ = Executor::init_tokio();
    let owner = Owner::new();
    owner.set();

    let (tx, rx) = oneshot::channel::<()>();
    let rx = Arc::new(Mutex::new(Some(rx)));
    let resumed = Arc::new(AtomicBool::new(false));

    let signal = RwSignal::new(1);
    let _value = AsyncDerived::new({
        let resumed = Arc::clone(&resumed);
        move || {
            let rx = rx.lock().unwrap().take();
            let resumed = Arc::clone(&resumed);
            async move {
                if let Some(rx) = rx {
                    _ = rx.await;
                }
                resumed.store(true, Ordering::Relaxed);
                // would panic, as the signal has been disposed
                signal.get()
            }
        }
    });

    Executor::tick().await;
    owner.cleanup();
    _ = tx.send(());
    Executor::tick().await;
    Executor::tick().await;

    assert!(!resumed.load(Ordering::Relaxed));
}

#[tokio::test]
async fn disposing_a_loading_async_derived_releases_its_suspense_reads() {
    use reactive_graph::{
        computed::suspense::SuspenseContext, owner::provide_context,
        signal::ArcRwSignal, traits::GetUntracked,
    };
    use slotmap::SlotMap;

    _ = Executor::init_tokio();
    let owner = Owner::new();
    owner.set();

    let tasks = ArcRwSignal::new(SlotMap::new());
    provide_context(SuspenseContext::new(tasks.clone()));

    let child = owner.child();
    child.with(|| {
        let value = AsyncDerived::new(pending::<i32>);
        assert_eq!(value.get_untracked(), None);
    });
    Executor::tick().await;
    assert!(
        !tasks.with_untracked(SlotMap::is_empty),
        "reading the loading value registers a suspense task"
    );

    child.cleanup();
    Executor::tick().await;
    Executor::tick().await;

    assert!(
        tasks.with_untracked(SlotMap::is_empty),
        "a value disposed while loading will never load, so the suspense must \
         stop waiting for it"
    );
}

#[tokio::test]
async fn disposing_a_loading_async_derived_finishes_its_ready_future() {
    use futures::FutureExt;

    _ = Executor::init_tokio();
    let owner = Owner::new();
    owner.set();

    let child = owner.child();
    let value = child.with(|| ArcAsyncDerived::new(pending::<i32>));
    let mut ready = Box::pin(value.ready());
    let mut awaited = Box::pin(value.clone().into_future());
    Executor::tick().await;
    assert!(ready.as_mut().now_or_never().is_none());

    child.cleanup();
    Executor::tick().await;

    assert!(
        ready.as_mut().now_or_never().is_some(),
        "the value stops loading once disposed, so whatever waits for it to \
         be ready, such as its serialization for hydration, must go on"
    );
    assert!(
        awaited.as_mut().now_or_never().is_none(),
        "a disposed value never loads, so awaiting it keeps waiting instead \
         of panicking"
    );
}
