use any_spawner::Executor;
use reactive_graph::{
    computed::{ArcAsyncDerived, AsyncDerived},
    signal::RwSignal,
    traits::{Get, Read, Set, With, WithUntracked},
};
use std::future::pending;

#[tokio::test]
async fn arc_async_derived_calculates_eagerly() {
    _ = Executor::init_tokio();

    let value = ArcAsyncDerived::new(|| async {
        Executor::tick().await;
        42
    });

    assert_eq!(value.clone().await, 42);
}

#[tokio::test]
async fn arc_async_derived_tracks_signal_change() {
    _ = Executor::init_tokio();

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

    let value = AsyncDerived::new(|| async {
        Executor::tick().await;
        42
    });

    assert_eq!(value.await, 42);
}

#[tokio::test]
async fn async_derived_tracks_signal_change() {
    _ = Executor::init_tokio();

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

    let value = ArcAsyncDerived::new(pending::<()>);
    assert_eq!(value.read(), None);
    assert_eq!(value.with_untracked(|n| *n), None);
    assert_eq!(value.with(|n| *n), None);
    assert_eq!(value.get(), None);
}

#[tokio::test]
async fn read_signal_traits_on_arena() {
    _ = Executor::init_tokio();

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
