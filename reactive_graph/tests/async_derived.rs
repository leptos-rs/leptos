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

#[cfg(feature = "effects")]
mod dependency_checks {
    use super::*;
    use reactive_graph::{
        computed::Memo, effect::Effect, traits::GetUntracked,
    };
    use std::{
        sync::{Arc, Mutex},
        time::Duration,
    };

    fn shared_source() -> (Owner, RwSignal<u32>, Memo<u32>, ArcAsyncDerived<u32>)
    {
        let owner = Owner::new();
        owner.set();
        let s = RwSignal::new(0);
        let m = Memo::new(move |_| s.get());
        // Wake the subscriber through M before waking B's own task.
        assert_eq!(m.get_untracked(), 0);
        let b = ArcAsyncDerived::new(move || {
            let value = s.get();
            async move { value * 10 }
        });
        (owner, s, m, b)
    }

    async fn settle() {
        for _ in 0..64 {
            tokio::task::yield_now().await;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
        for _ in 0..64 {
            tokio::task::yield_now().await;
        }
    }

    /// An idle async subscriber must preserve its source's pending reload.
    #[tokio::test]
    async fn idle_async_subscriber_preserves_pending_reload() {
        _ = Executor::init_tokio();
        let (_owner, s, m, b) = shared_source();
        let a = ArcAsyncDerived::new({
            let b = b.clone();
            move || {
                let value = (b.get(), m.get());
                async move { value }
            }
        });
        settle().await;
        assert_eq!(a.get_untracked(), Some((Some(0), 0)));
        s.set(1);
        settle().await;
        assert_eq!(b.get_untracked(), Some(10));
        assert_eq!(a.get_untracked(), Some((Some(10), 1)));
    }

    /// An effect's dependency check must preserve its source's pending reload.
    #[tokio::test]
    async fn effect_subscriber_preserves_pending_reload() {
        _ = Executor::init_tokio();
        tokio::task::LocalSet::new()
            .run_until(async {
                let (_owner, s, m, b) = shared_source();
                let value = Arc::new(Mutex::new(None));
                let _effect = Effect::new({
                    let (b, value) = (b.clone(), Arc::clone(&value));
                    move || *value.lock().unwrap() = Some((b.get(), m.get()))
                });
                settle().await;
                assert_eq!(*value.lock().unwrap(), Some((Some(0), 0)));
                s.set(1);
                settle().await;
                assert_eq!(b.get_untracked(), Some(10));
                assert_eq!(*value.lock().unwrap(), Some((Some(10), 1)));
            })
            .await;
    }

    /// A memo's dependency check must preserve its source's pending reload.
    #[tokio::test]
    async fn memo_subscriber_preserves_pending_reload() {
        _ = Executor::init_tokio();
        let (_owner, s, m, b) = shared_source();
        let subscriber = Memo::new({
            let b = b.clone();
            move |_| (b.get(), m.get())
        });
        assert_eq!(subscriber.get_untracked(), (Some(0), 0));
        settle().await;
        s.set(1);
        // Check B before its task gets a chance to reload.
        _ = subscriber.get_untracked();
        settle().await;
        assert_eq!(b.get_untracked(), Some(10));
        assert_eq!(subscriber.get_untracked(), (Some(10), 1));
    }
}
