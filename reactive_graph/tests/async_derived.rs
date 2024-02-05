use reactive_graph::{
    computed::{ArcAsyncDerived, AsyncDerived, AsyncState},
    executor::Executor,
    signal::RwSignal,
    traits::{Get, Readable, Set, With, WithUntracked},
};

#[cfg(feature = "tokio")]
#[tokio::test]
async fn async_derived_calculates_eagerly() {
    use std::time::Duration;
    use tokio::time::sleep;

    _ = Executor::init_tokio();

    let value = ArcAsyncDerived::new(|| async {
        sleep(Duration::from_millis(25)).await;
        42
    });

    assert_eq!(value.clone().await, 42);
    std::mem::forget(value);
}

#[cfg(feature = "tokio")]
#[tokio::test]
async fn async_derived_tracks_signal_change() {
    use std::time::Duration;
    use tokio::time::sleep;

    _ = Executor::init_tokio();

    let signal = RwSignal::new(10);
    let value = ArcAsyncDerived::new(move || async move {
        sleep(Duration::from_millis(25)).await;
        signal.get()
    });

    assert_eq!(value.clone().await, 10);
    signal.set(30);
    sleep(Duration::from_millis(5)).await;
    assert_eq!(value.clone().await, 30);
    signal.set(50);
    sleep(Duration::from_millis(5)).await;
    assert_eq!(value.clone().await, 50);
    std::mem::forget(value);
}

#[test]
fn read_signal_traits_on_arc() {
    let value = ArcAsyncDerived::new(move || async {});
    assert_eq!(*value.read(), AsyncState::Loading);
    assert_eq!(value.with_untracked(|n| *n), AsyncState::Loading);
    assert_eq!(value.with(|n| *n), AsyncState::Loading);
    assert_eq!(value.get(), AsyncState::Loading);
}

#[test]
fn read_signal_traits_on_arena() {
    let value = AsyncDerived::new(move || async {});
    assert_eq!(value.with_untracked(|n| *n), AsyncState::Loading);
    assert_eq!(value.with(|n| *n), AsyncState::Loading);
    assert_eq!(value.get(), AsyncState::Loading);
}
