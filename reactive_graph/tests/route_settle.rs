//! Lifecycle of `RouteSettleContext`: registration, settlement, and closing.

use reactive_graph::computed::suspense::RouteSettleContext;
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    task::{Wake, Waker},
};

struct CountingWaker(AtomicUsize);

impl Wake for CountingWaker {
    fn wake(self: Arc<Self>) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

fn waker() -> (Waker, Arc<CountingWaker>) {
    let inner = Arc::new(CountingWaker(AtomicUsize::new(0)));
    (Waker::from(Arc::clone(&inner)), inner)
}

#[test]
fn settles_immediately_without_tasks_and_closes() {
    let ctx = RouteSettleContext::new();
    let (waker, _) = waker();
    assert!(ctx.poll_settled(&waker));
    assert!(ctx.task().is_none(), "settling must close the context");
}

#[test]
fn releasing_the_last_task_wakes_and_settles() {
    let ctx = RouteSettleContext::new();
    let (waker, wakes) = waker();
    let a = ctx.task().expect("open context accepts tasks");
    let b = ctx.task().expect("open context accepts tasks");
    assert!(!ctx.poll_settled(&waker));

    drop(a);
    assert_eq!(wakes.0.load(Ordering::SeqCst), 0);
    assert!(!ctx.poll_settled(&waker));

    drop(b);
    assert_eq!(wakes.0.load(Ordering::SeqCst), 1);
    assert!(ctx.poll_settled(&waker));
    assert!(ctx.task().is_none());
}

#[test]
fn closing_wakes_a_pending_waiter_and_rejects_new_tasks() {
    let ctx = RouteSettleContext::new();
    let (waker, wakes) = waker();
    let held = ctx.task().expect("open context accepts tasks");
    assert!(!ctx.poll_settled(&waker));

    ctx.close();
    assert_eq!(wakes.0.load(Ordering::SeqCst), 1);
    assert!(
        ctx.poll_settled(&waker),
        "a closed context counts as settled"
    );
    assert!(ctx.task().is_none());

    // releasing a task that outlived the close is harmless
    drop(held);
    assert!(ctx.poll_settled(&waker));
}

#[test]
fn waker_is_registered_once() {
    let ctx = RouteSettleContext::new();
    let (waker, wakes) = waker();
    let held = ctx.task().expect("open context accepts tasks");
    assert!(!ctx.poll_settled(&waker));
    assert!(!ctx.poll_settled(&waker));
    drop(held);
    assert_eq!(wakes.0.load(Ordering::SeqCst), 1);
}
