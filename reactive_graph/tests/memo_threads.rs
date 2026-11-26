//! Regression tests to ensure that if one of a memo's sources changes on one thread
//! while the memo is recomputing on another thread, the recomputation doesn't mark
//! the whole memo clean.
//!
//! See https://github.com/leptos-rs/leptos/issues/5022

use reactive_graph::{
    computed::{ArcAsyncDerived, ArcMemo},
    owner::Owner,
    prelude::*,
    signal::ArcRwSignal,
    wrappers::read::ArcSignal,
};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex, mpsc},
    thread,
    time::Duration,
};

#[derive(Clone, Default)]
#[allow(clippy::type_complexity)]
struct Pause(Arc<Mutex<VecDeque<(mpsc::Sender<()>, mpsc::Receiver<()>)>>>);

struct Paused {
    reached: mpsc::Receiver<()>,
    resume: mpsc::Sender<()>,
}

impl Pause {
    /// The next run that calls [`Pause::point`] will pause there.
    fn arm(&self) -> Paused {
        let (reached_tx, reached) = mpsc::channel();
        let (resume, resume_rx) = mpsc::channel();
        self.0.lock().unwrap().push_back((reached_tx, resume_rx));
        Paused { reached, resume }
    }

    fn point(&self) {
        let next = self.0.lock().unwrap().pop_front();
        if let Some((reached, resume)) = next {
            reached.send(()).unwrap();
            resume.recv().unwrap();
        }
    }
}

impl Paused {
    fn wait(&self) {
        self.reached.recv().unwrap();
    }

    fn resume(&self) {
        self.resume.send(()).unwrap();
    }
}

fn owner() -> Owner {
    let owner = Owner::new();
    owner.set();
    owner
}

/// The case from the issue: a signal changes while the memo is recomputing.
#[test]
fn signal_changes_while_memo_computes() {
    let _owner = owner();
    let source = ArcRwSignal::new(0);
    let pause = Pause::default();
    let memo = ArcMemo::new({
        let (source, pause) = (source.clone(), pause.clone());
        move |_| {
            let value = source.get();
            pause.point();
            value
        }
    });

    let paused = pause.arm();
    let computing = thread::spawn({
        let memo = memo.clone();
        move || memo.get()
    });
    paused.wait();
    source.set(1);
    paused.resume();

    assert_eq!(computing.join().unwrap(), 0);
    assert_eq!(memo.get(), 1);
}

#[test]
fn upstream_memo_changes_while_memo_computes() {
    let _owner = owner();
    let source = ArcRwSignal::new(0);
    let upstream = ArcMemo::new({
        let source = source.clone();
        move |_| source.get()
    });
    let pause = Pause::default();
    let memo = ArcMemo::new({
        let (upstream, pause) = (upstream.clone(), pause.clone());
        move |_| {
            let value = upstream.get();
            pause.point();
            value
        }
    });

    let paused = pause.arm();
    let computing = thread::spawn({
        let memo = memo.clone();
        move || memo.get()
    });
    paused.wait();
    source.set(1);
    paused.resume();

    assert_eq!(computing.join().unwrap(), 0);
    assert_eq!(memo.get(), 1);
}

#[test]
fn derived_signal_changes_while_memo_computes() {
    let _owner = owner();
    let source = ArcRwSignal::new(0);
    let derived = ArcSignal::derive({
        let source = source.clone();
        move || source.get() * 10
    });
    let pause = Pause::default();
    let memo = ArcMemo::new({
        let (derived, pause) = (derived.clone(), pause.clone());
        move |_| {
            let value = derived.get();
            pause.point();
            value
        }
    });

    let paused = pause.arm();
    let computing = thread::spawn({
        let memo = memo.clone();
        move || memo.get()
    });
    paused.wait();
    source.set(1);
    paused.resume();

    assert_eq!(computing.join().unwrap(), 0);
    assert_eq!(memo.get(), 10);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn async_derived_resolves_while_memo_computes() {
    _ = any_spawner::Executor::init_tokio();
    let _owner = owner();

    let (resolve, resolved) = tokio::sync::oneshot::channel::<i32>();
    let resolved = Arc::new(Mutex::new(Some(resolved)));
    let async_value = ArcAsyncDerived::new(move || {
        let resolved = resolved.lock().unwrap().take();
        async move {
            match resolved {
                Some(resolved) => resolved.await.unwrap(),
                None => -1,
            }
        }
    });
    let pause = Pause::default();
    let memo = ArcMemo::new({
        let (async_value, pause) = (async_value.clone(), pause.clone());
        move |_| {
            let value = async_value.get();
            pause.point();
            value
        }
    });

    let paused = pause.arm();
    let computing = thread::spawn({
        let memo = memo.clone();
        move || memo.get()
    });
    paused.wait();
    resolve.send(1).unwrap();
    while async_value.get_untracked().is_none() {
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    paused.resume();

    assert_eq!(computing.join().unwrap(), None);
    assert_eq!(memo.get(), Some(1));
}

#[test]
fn source_changes_while_memo_checks_sources() {
    let _owner = owner();
    // `first` is walked first, and changes while `second` is being checked.
    let first_source = ArcRwSignal::new(0);
    let first = ArcMemo::new({
        let first_source = first_source.clone();
        move |_| first_source.get()
    });
    let second_source = ArcRwSignal::new(0);
    let pause = Pause::default();
    let second = ArcMemo::new({
        let (second_source, pause) = (second_source.clone(), pause.clone());
        move |_| {
            let value = second_source.get() / 10;
            pause.point();
            value
        }
    });
    let memo = ArcMemo::new({
        let (first, second) = (first.clone(), second.clone());
        move |_| first.get() + second.get()
    });
    assert_eq!(memo.get(), 0);

    // `second` reruns but doesn't change
    second_source.set(1);
    let paused = pause.arm();
    let checking = thread::spawn({
        let memo = memo.clone();
        move || memo.get()
    });
    paused.wait();
    first_source.set(5);
    paused.resume();

    let raced = checking.join().unwrap();
    assert!(raced == 0 || raced == 5, "{raced}");
    assert_eq!(memo.get(), 5);
}

#[test]
fn overlapping_runs_source_changes_during_both() {
    let _owner = owner();
    let source = ArcRwSignal::new(0);
    let pause = Pause::default();
    let memo = ArcMemo::new({
        let (source, pause) = (source.clone(), pause.clone());
        move |_| {
            let value = source.get();
            pause.point();
            value
        }
    });

    let first = pause.arm();
    let second = pause.arm();
    let a = thread::spawn({
        let memo = memo.clone();
        move || memo.get()
    });
    first.wait();
    let b = thread::spawn({
        let memo = memo.clone();
        move || memo.get()
    });
    second.wait();
    source.set(1);
    first.resume();
    assert_eq!(a.join().unwrap(), 0);
    second.resume();
    assert_eq!(b.join().unwrap(), 0);

    assert_eq!(memo.get(), 1);
}

#[test]
fn overlapping_runs_source_changes_after_first_finishes() {
    let _owner = owner();
    let source = ArcRwSignal::new(0);
    let pause = Pause::default();
    let memo = ArcMemo::new({
        let (source, pause) = (source.clone(), pause.clone());
        move |_| {
            let value = source.get();
            pause.point();
            value
        }
    });

    let first = pause.arm();
    let second = pause.arm();
    let a = thread::spawn({
        let memo = memo.clone();
        move || memo.get()
    });
    first.wait();
    let b = thread::spawn({
        let memo = memo.clone();
        move || memo.get()
    });
    second.wait();
    first.resume();
    assert_eq!(a.join().unwrap(), 0);
    source.set(1);
    second.resume();
    assert_eq!(b.join().unwrap(), 0);

    assert_eq!(memo.get(), 1);
}

#[test]
fn unchanged_upstream_memo_marked_while_memo_computes_does_not_rerun() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    let _owner = owner();
    let source = ArcRwSignal::new(0);
    let upstream = ArcMemo::new({
        let source = source.clone();
        move |_| source.get() / 10
    });
    let runs = Arc::new(AtomicUsize::new(0));
    let pause = Pause::default();
    let memo = ArcMemo::new({
        let (upstream, pause, runs) =
            (upstream.clone(), pause.clone(), runs.clone());
        move |_| {
            runs.fetch_add(1, Ordering::SeqCst);
            let value = upstream.get();
            pause.point();
            value
        }
    });

    let paused = pause.arm();
    let computing = thread::spawn({
        let memo = memo.clone();
        move || memo.get()
    });
    paused.wait();
    source.set(1);
    paused.resume();

    assert_eq!(computing.join().unwrap(), 0);
    assert_eq!(memo.get(), 0);
    assert_eq!(runs.load(Ordering::SeqCst), 1);
}

#[test]
fn dirty_mark_wins_over_check_mark_while_memo_computes() {
    for check_first in [true, false] {
        let _owner = owner();
        let upstream_source = ArcRwSignal::new(0);
        let upstream = ArcMemo::new({
            let upstream_source = upstream_source.clone();
            move |_| upstream_source.get() / 10
        });
        let source = ArcRwSignal::new(0);
        let pause = Pause::default();
        let memo = ArcMemo::new({
            let (upstream, source, pause) =
                (upstream.clone(), source.clone(), pause.clone());
            move |_| {
                let value = upstream.get() + source.get();
                pause.point();
                value
            }
        });

        let paused = pause.arm();
        let computing = thread::spawn({
            let memo = memo.clone();
            move || memo.get()
        });
        paused.wait();
        if check_first {
            upstream_source.set(1);
            source.set(1);
        } else {
            source.set(1);
            upstream_source.set(1);
        }
        paused.resume();

        assert_eq!(computing.join().unwrap(), 0);
        assert_eq!(memo.get(), 1, "check_first = {check_first}");
    }
}
