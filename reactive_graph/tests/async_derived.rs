use any_spawner::Executor;
use futures::{FutureExt, channel::oneshot, future::BoxFuture};
use reactive_graph::{
    computed::{ArcAsyncDerived, AsyncDerived},
    owner::{Owner, on_cleanup},
    signal::RwSignal,
    traits::{Get, GetUntracked, Read, Set, With, WithUntracked},
};
#[cfg(feature = "sandboxed-arenas")]
use reactive_graph::{
    owner::StoredValue, signal::ArcRwSignal, traits::GetValue,
};
#[cfg(feature = "sandboxed-arenas")]
use std::sync::atomic::AtomicU32;
use std::{
    future::{pending, ready},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering::SeqCst},
    },
    time::Duration,
};
use tokio::time::timeout;

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

    /// A pending async subscriber must preserve its source's pending reload.
    #[tokio::test]
    async fn pending_async_subscriber_preserves_pending_reload() {
        _ = Executor::init_tokio();
        let (_owner, s, m, b) = shared_source();
        let t = RwSignal::new(0);
        let (release, gate) = oneshot::channel::<()>();
        let gate = gate.map(|_| ()).shared();
        let a = ArcAsyncDerived::new({
            let b = b.clone();
            move || {
                let value = (b.get(), m.get());
                let wait = (t.get() > 0).then(|| gate.clone());
                async move {
                    if let Some(wait) = wait {
                        wait.await;
                    }
                    value
                }
            }
        });
        settle().await;
        // `a` loads again and waits for `release` while `s` changes
        t.set(1);
        settle().await;
        s.set(1);
        settle().await;
        assert_eq!(b.get_untracked(), Some(10));
        assert!(release.send(()).is_ok());
        settle().await;
        assert_eq!(a.get_untracked(), Some((Some(10), 1)));
    }
}

/// Yields to the executor until `done` holds, at most a fixed number of times.
async fn yield_until(done: impl Fn() -> bool) -> bool {
    for _ in 0..64 {
        if done() {
            return true;
        }
        tokio::task::yield_now().await;
    }
    done()
}

/// Creates a derived whose run `n` reads `source`, awaits `wait_for(n)` and
/// returns the value read. Returns once run 0 is pending in the spawned task.
async fn derived_waiting_on(
    wait_for: impl Fn(usize) -> BoxFuture<'static, ()> + Send + Sync + 'static,
) -> (RwSignal<u32>, ArcAsyncDerived<u32>, Arc<AtomicUsize>) {
    _ = Executor::init_tokio();
    let source = RwSignal::new(0);
    let runs = Arc::new(AtomicUsize::new(0));
    let started = Arc::new(AtomicBool::new(false));
    let derived = ArcAsyncDerived::new({
        let (runs, started) = (Arc::clone(&runs), Arc::clone(&started));
        move || {
            let value = source.get();
            let wait = wait_for(runs.fetch_add(1, SeqCst));
            let started = Arc::clone(&started);
            async move {
                // `new` polls once in place; continue in the spawned task
                Executor::tick().await;
                started.store(true, SeqCst);
                wait.await;
                value
            }
        }
    });
    assert!(yield_until(|| started.load(SeqCst)).await);
    (source, derived, runs)
}

#[tokio::test]
async fn async_derived_restarts_when_source_changes_while_loading() {
    let owner = Owner::new();
    owner.set();

    let (source, derived, runs) = derived_waiting_on(|run| match run {
        0 => pending().boxed(),
        _ => ready(()).boxed(),
    })
    .await;
    assert_eq!(runs.load(SeqCst), 1);

    source.set(42);
    assert_eq!(
        timeout(Duration::from_secs(5), derived.clone()).await,
        Ok(42),
        "the change did not restart the pending load; calls: {}",
        runs.load(SeqCst)
    );
    assert_eq!(runs.load(SeqCst), 2);
}

#[tokio::test]
async fn async_derived_does_not_publish_superseded_result() {
    let owner = Owner::new();
    owner.set();

    let (release, gate) = oneshot::channel::<()>();
    let gate = gate.map(|_| ()).shared();
    let (source, derived, runs) = derived_waiting_on(move |run| match run {
        0 => gate.clone().boxed(),
        _ => pending().boxed(),
    })
    .await;

    // the first load can finish, but only after its input has changed
    source.set(1);
    assert!(release.send(()).is_ok());
    yield_until(|| runs.load(SeqCst) >= 2).await;

    assert_eq!(runs.load(SeqCst), 2);
    assert_eq!(
        derived.get_untracked(),
        None,
        "the result computed from the old input was published"
    );
}

#[tokio::test]
async fn async_derived_drops_superseded_future_before_cleanup() {
    static DROPPED: AtomicBool = AtomicBool::new(false);
    static CLEANED_AFTER_DROP: AtomicBool = AtomicBool::new(false);
    struct SetOnDrop;
    impl Drop for SetOnDrop {
        fn drop(&mut self) {
            DROPPED.store(true, SeqCst);
        }
    }

    let owner = Owner::new();
    owner.set();

    let (release, gate) = oneshot::channel::<()>();
    let gate = gate.map(|_| ()).shared();
    let (source, derived, runs) = derived_waiting_on(move |run| match run {
        0 => {
            // runs when the replacement load cleans up the owner
            on_cleanup(|| {
                CLEANED_AFTER_DROP.store(DROPPED.load(SeqCst), SeqCst)
            });
            async {
                let _guard = SetOnDrop;
                pending().await
            }
            .boxed()
        }
        _ => gate.clone().boxed(),
    })
    .await;
    assert!(!DROPPED.load(SeqCst));

    source.set(1);
    yield_until(|| runs.load(SeqCst) >= 2).await;

    assert_eq!(runs.load(SeqCst), 2);
    assert!(DROPPED.load(SeqCst));
    assert!(
        CLEANED_AFTER_DROP.load(SeqCst),
        "the owner was cleaned up before the superseded future was dropped"
    );
    assert_eq!(derived.get_untracked(), None);

    assert!(release.send(()).is_ok());
    let value = timeout(Duration::from_secs(5), derived.clone()).await;
    assert_eq!(value, Ok(1));
    assert_eq!(runs.load(SeqCst), 2);
}

#[tokio::test]
async fn async_derived_manual_set_wins_over_pending_load() {
    static FINISHED: AtomicBool = AtomicBool::new(false);
    let owner = Owner::new();
    owner.set();

    let (release, gate) = oneshot::channel::<()>();
    let gate = gate.map(|_| FINISHED.store(true, SeqCst)).shared();
    let (_, derived, _) =
        derived_waiting_on(move |_| gate.clone().boxed()).await;

    derived.set(Some(5));
    assert!(release.send(()).is_ok());
    assert!(yield_until(|| FINISHED.load(SeqCst)).await);
    assert_eq!(derived.get_untracked(), Some(5));
}

#[tokio::test]
async fn async_derived_paused_owner_does_not_restart_pending_load() {
    let owner = Owner::new();
    owner.set();

    let (release, gate) = oneshot::channel::<()>();
    let gate = gate.map(|_| ()).shared();
    let (source, derived, runs) =
        derived_waiting_on(move |_| gate.clone().boxed()).await;

    owner.pause();
    source.set(1);
    assert!(release.send(()).is_ok());

    let value = timeout(Duration::from_secs(5), derived.clone()).await;
    assert_eq!(value, Ok(0));
    assert_eq!(runs.load(SeqCst), 1);
}

#[cfg(feature = "sandboxed-arenas")]
fn derived_recording_drop(
    source: ArcRwSignal<u32>,
    started: Arc<AtomicBool>,
    recorded: Arc<AtomicU32>,
) -> ArcAsyncDerived<u32> {
    struct RecordOnDrop(StoredValue<u32>, Arc<AtomicU32>);
    impl Drop for RecordOnDrop {
        fn drop(&mut self) {
            self.1.store(self.0.try_get_value().unwrap_or(0), SeqCst);
        }
    }

    ArcAsyncDerived::new(move || {
        let value = source.get();
        let guard = (value == 0).then(|| {
            RecordOnDrop(StoredValue::new(7_u32), Arc::clone(&recorded))
        });
        let started = Arc::clone(&started);
        async move {
            let _guard = guard;
            Executor::tick().await;
            if value == 0 {
                started.store(true, SeqCst);
                pending::<()>().await;
            }
            value
        }
    })
}

/// Cancelled futures must drop their live locals inside the derived's owner.
#[cfg(feature = "sandboxed-arenas")]
#[tokio::test]
async fn async_derived_cancellation_drops_future_in_own_arena() {
    _ = Executor::init_tokio();
    assert_eq!(Owner::current(), None);
    let source = ArcRwSignal::new(0);
    let started = Arc::new(AtomicBool::new(false));
    let recorded = Arc::new(AtomicU32::new(u32::MAX));
    let derived = derived_recording_drop(
        source.clone(),
        Arc::clone(&started),
        Arc::clone(&recorded),
    );
    assert!(yield_until(|| started.load(SeqCst)).await);
    source.set(1);
    assert_eq!(timeout(Duration::from_secs(5), derived).await, Ok(1));
    assert_eq!(recorded.load(SeqCst), 7);
}

/// Stale initial futures must drop their live locals inside the derived's owner.
#[cfg(feature = "sandboxed-arenas")]
#[tokio::test]
async fn async_derived_initial_discard_drops_future_in_own_arena() {
    _ = Executor::init_tokio();
    assert_eq!(Owner::current(), None);
    let source = ArcRwSignal::new(0);
    let recorded = Arc::new(AtomicU32::new(u32::MAX));
    let derived = derived_recording_drop(
        source.clone(),
        Arc::new(AtomicBool::new(false)),
        Arc::clone(&recorded),
    );
    source.set(1);
    assert_eq!(timeout(Duration::from_secs(5), derived).await, Ok(1));
    assert_eq!(recorded.load(SeqCst), 7);
}

#[cfg(feature = "effects")]
mod stale_dependencies {
    use super::*;
    use reactive_graph::traits::GetUntracked;
    use std::{
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering::SeqCst},
        },
        time::Duration,
    };

    async fn settle() {
        for _ in 0..64 {
            tokio::task::yield_now().await;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
        for _ in 0..64 {
            tokio::task::yield_now().await;
        }
    }

    /// A signal stops invalidating a derived once a later run stops reading it.
    #[tokio::test]
    async fn stops_tracking_unused_signal() {
        _ = Executor::init_tokio();
        let owner = Owner::new();
        owner.set();
        let mode = RwSignal::new(0);
        let ticker = RwSignal::new(0);
        let runs = Arc::new(AtomicUsize::new(0));
        let derived = ArcAsyncDerived::new({
            let runs = Arc::clone(&runs);
            move || {
                runs.fetch_add(1, SeqCst);
                let value = if mode.get() == 0 { ticker.get() } else { 42 };
                async move { value }
            }
        });
        settle().await;
        mode.set(1);
        settle().await;
        assert_eq!(runs.load(SeqCst), 2);
        for value in 1..=2 {
            ticker.set(value);
            settle().await;
        }
        assert_eq!(derived.get_untracked(), Some(42));
        assert_eq!(runs.load(SeqCst), 2);
    }

    /// A signal that the pending run did not read cannot restart its load.
    #[tokio::test]
    async fn unused_signal_does_not_restart_pending_load() {
        _ = Executor::init_tokio();
        let owner = Owner::new();
        owner.set();
        let mode = RwSignal::new(0);
        let ticker = RwSignal::new(0);
        let (release, gate) = oneshot::channel::<()>();
        let gate = gate.map(|_| ()).shared();
        let runs = Arc::new(AtomicUsize::new(0));
        let derived = ArcAsyncDerived::new({
            let runs = Arc::clone(&runs);
            move || {
                runs.fetch_add(1, SeqCst);
                let mode = mode.get();
                let value = if mode == 0 { ticker.get() } else { 42 };
                let gate = gate.clone();
                async move {
                    if mode != 0 {
                        gate.await;
                    }
                    value
                }
            }
        });
        settle().await;
        // the new run waits for `release` and does not read `ticker`
        mode.set(1);
        settle().await;
        for value in 1..=3 {
            ticker.set(value);
            settle().await;
        }
        assert_eq!(runs.load(SeqCst), 2);
        assert!(release.send(()).is_ok());
        settle().await;
        assert_eq!(derived.get_untracked(), Some(42));
    }

    /// Dependencies first read after an await in a later run remain tracked.
    #[tokio::test]
    async fn tracks_dependency_first_read_after_await() {
        _ = Executor::init_tokio();
        let owner = Owner::new();
        owner.set();
        let mode = RwSignal::new(false);
        let late = RwSignal::new(0);
        let runs = Arc::new(AtomicUsize::new(0));
        let derived = ArcAsyncDerived::new({
            let runs = Arc::clone(&runs);
            move || {
                runs.fetch_add(1, SeqCst);
                let read_late = mode.get();
                async move {
                    tokio::task::yield_now().await;
                    if read_late { late.get() } else { 0 }
                }
            }
        });
        settle().await;
        mode.set(true);
        settle().await;
        assert_eq!(runs.load(SeqCst), 2);
        late.set(7);
        settle().await;
        assert_eq!(derived.get_untracked(), Some(7));
        assert_eq!(runs.load(SeqCst), 3);
    }
}

#[cfg(feature = "effects")]
mod awaited_derived {
    use super::*;
    use futures::FutureExt;
    use reactive_graph::{
        graph::{
            AnySource, AnySubscriber, ReactiveNode, Subscriber, WithObserver,
        },
        traits::{GetUntracked, Write},
    };
    use std::{
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering::SeqCst},
        },
        time::Duration,
    };

    async fn settle() {
        for _ in 0..64 {
            tokio::task::yield_now().await;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
        for _ in 0..64 {
            tokio::task::yield_now().await;
        }
    }

    /// Awaiting a newly created pending child publishes once without a rerun loop.
    #[tokio::test]
    async fn newly_created_child_does_not_invalidate_parent() {
        _ = Executor::init_tokio();
        let owner = Owner::new();
        owner.set();
        let runs = Arc::new(AtomicUsize::new(0));
        let parent = ArcAsyncDerived::new({
            let runs = Arc::clone(&runs);
            move || {
                runs.fetch_add(1, SeqCst);
                async {
                    let child = ArcAsyncDerived::new(|| async {
                        tokio::task::yield_now().await;
                        42
                    });
                    child.await
                }
            }
        });
        settle().await;
        assert_eq!(parent.get_untracked(), Some(42));
        assert_eq!(runs.load(SeqCst), 1);
    }

    /// Awaiting an existing pending child by reference does not rerun its parent.
    #[tokio::test]
    async fn pending_child_by_ref_does_not_invalidate_parent() {
        _ = Executor::init_tokio();
        let owner = Owner::new();
        owner.set();
        let child = ArcAsyncDerived::new(|| async {
            tokio::task::yield_now().await;
            42
        });
        let runs = Arc::new(AtomicUsize::new(0));
        let parent = ArcAsyncDerived::new({
            let runs = Arc::clone(&runs);
            move || {
                runs.fetch_add(1, SeqCst);
                let child = child.clone();
                async move { *child.by_ref().await }
            }
        });
        settle().await;
        assert_eq!(parent.get_untracked(), Some(42));
        assert_eq!(runs.load(SeqCst), 1);
    }

    /// Readiness awaits track later child loads without invalidating the first wait.
    #[tokio::test]
    async fn readiness_tracks_subsequent_child_loads() {
        _ = Executor::init_tokio();
        let owner = Owner::new();
        owner.set();
        let source = RwSignal::new(0);
        let child = ArcAsyncDerived::new(move || {
            let value = source.get();
            async move {
                tokio::task::yield_now().await;
                value * 10
            }
        });
        let runs = Arc::new(AtomicUsize::new(0));
        let parent = ArcAsyncDerived::new({
            let runs = Arc::clone(&runs);
            move || {
                runs.fetch_add(1, SeqCst);
                let child = child.clone();
                async move {
                    child.ready().await;
                    child.get_untracked()
                }
            }
        });
        settle().await;
        assert_eq!(parent.get_untracked(), Some(Some(0)));
        assert_eq!(runs.load(SeqCst), 1);
        source.set(1);
        settle().await;
        assert_eq!(parent.get_untracked(), Some(Some(10)));
        assert_eq!(runs.load(SeqCst), 2);
    }

    /// Awaiting a pending child still tracks the child's later loads.
    #[tokio::test]
    async fn await_tracks_subsequent_child_loads() {
        _ = Executor::init_tokio();
        let owner = Owner::new();
        owner.set();
        let source = RwSignal::new(0);
        let child = ArcAsyncDerived::new(move || {
            let value = source.get();
            async move {
                tokio::task::yield_now().await;
                value * 10
            }
        });
        let runs = Arc::new(AtomicUsize::new(0));
        let parent = ArcAsyncDerived::new({
            let runs = Arc::clone(&runs);
            move || {
                runs.fetch_add(1, SeqCst);
                let child = child.clone();
                async move { child.await }
            }
        });
        settle().await;
        assert_eq!(parent.get_untracked(), Some(0));
        assert_eq!(runs.load(SeqCst), 1);
        source.set(1);
        settle().await;
        assert_eq!(parent.get_untracked(), Some(10));
        assert_eq!(runs.load(SeqCst), 2);
    }

    async fn writer_locked_child_completes(by_ref: bool) {
        _ = Executor::init_tokio();
        let owner = Owner::new();
        owner.set();
        let child = ArcAsyncDerived::new(|| async { 42 });
        let guard = child.write();
        let parent = ArcAsyncDerived::new({
            let child = child.clone();
            move || {
                let child = child.clone();
                async move {
                    if by_ref {
                        *child.by_ref().await
                    } else {
                        child.await
                    }
                }
            }
        });
        settle().await;
        drop(guard);
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(1), parent).await,
            Ok(42)
        );
    }

    /// A notifying writer wakes a reactive awaiter waiting to clone its value.
    #[tokio::test]
    async fn writer_locked_value_wakes_awaiter() {
        writer_locked_child_completes(false).await;
    }

    /// A notifying writer wakes a reactive awaiter waiting to borrow its value.
    #[tokio::test]
    async fn writer_locked_value_wakes_by_ref_awaiter() {
        writer_locked_child_completes(true).await;
    }

    /// Tracking permits a writer to acquire the source's locks without deadlocking.
    #[test]
    fn tracking_releases_value_and_waker_locks() {
        struct WritingObserver(ArcAsyncDerived<i32>);
        impl ReactiveNode for WritingObserver {
            fn mark_dirty(&self) {}
            fn mark_check(&self) {}
            fn mark_subscribers_check(&self) {}
            fn update_if_necessary(&self) -> bool {
                false
            }
        }
        impl Subscriber for WritingObserver {
            fn add_source(&self, _: AnySource) {
                drop(self.0.write());
            }
            fn clear_sources(&self, _: &AnySubscriber) {}
        }

        // The watchdog must run outside the thread whose poll can block.
        let (tx, rx) = std::sync::mpsc::channel();
        let _worker = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            runtime.block_on(async {
                _ = Executor::init_tokio();
                let owner = Owner::new();
                owner.set();
                let child = ArcAsyncDerived::new(|| async { 42 });
                let subscriber: Arc<dyn Subscriber + Send + Sync> =
                    Arc::new(WritingObserver(child.clone()));
                let observer = AnySubscriber(0, Arc::downgrade(&subscriber));
                let value = observer
                    .with_observer(|| async { child.await }.now_or_never());
                _ = tx.send(value);
            });
        });
        assert_eq!(rx.recv_timeout(Duration::from_secs(2)), Ok(Some(42)));
    }
}
