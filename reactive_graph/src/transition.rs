//! Utilities to wait for asynchronous primitives to resolve.

use futures::{channel::oneshot, future::join_all};
use pin_project_lite::pin_project;
use std::{
    cell::RefCell,
    future::Future,
    iter,
    pin::Pin,
    sync::mpsc,
    task::{Context, Poll},
};

thread_local! {
    // The transition that is *currently being polled* on this thread. It is
    // installed for the duration of each poll of the action future and removed
    // again when that poll returns, so overlapping transitions (whether on the
    // same thread or on different threads of a multi-threaded executor) never
    // observe one another's slot.
    static TRANSITION: RefCell<Option<TransitionInner>> =
        const { RefCell::new(None) };
}

#[derive(Debug, Clone)]
struct TransitionInner {
    tx: mpsc::Sender<oneshot::Receiver<()>>,
    // whether a resource that a source notifies while this transition is
    // active registers the load that follows with it (see `track`)
    capture_notified: bool,
}

/// Transitions allow you to wait for all asynchronous resources created during them to resolve.
#[derive(Debug)]
pub struct AsyncTransition;

impl AsyncTransition {
    /// Calls the `action` function, and returns a `Future` that resolves when any
    /// [`AsyncDerived`](crate::computed::AsyncDerived) or
    /// or [`ArcAsyncDerived`](crate::computed::ArcAsyncDerived) that is read during the action
    /// has resolved.
    ///
    /// This allows for an inversion of control: the caller does not need to know when all the
    /// resources created inside the `action` will resolve, but can wait for them to notify it.
    pub async fn run<T, U>(action: impl FnOnce() -> T) -> U
    where
        T: Future<Output = U>,
    {
        let (tx, rx) = mpsc::channel();
        let inner = TransitionInner {
            tx,
            capture_notified: false,
        };

        // While the action is being run and its future polled, install `inner`
        // as the current transition. The guard inside `ScopedTransition::poll`
        // restores the previous value on every poll exit, so this is safe to
        // run concurrently with other transitions. `action` itself is invoked
        // inside that scope (on the first poll) so resources created
        // synchronously by it are registered too.
        let value = ScopedTransition {
            inner,
            action: Some(action),
            future: None,
        }
        .await;

        let mut pending = Vec::new();
        while let Ok(rx) = rx.try_recv() {
            pending.push(rx);
        }
        join_all(pending).await;
        value
    }

    /// Runs `action` synchronously with a transition active, and returns its
    /// value together with a future that resolves once every async resource
    /// that was created during `action`, or that a signal update during
    /// `action` notified (for example, a resource that depends on route
    /// params when the params change), has finished the load that follows.
    ///
    /// A notification that turns out not to require a reload is not waited
    /// for, and neither is work that effects start later because of the
    /// updates. Dropping the returned future stops waiting, but does not
    /// cancel the loads.
    #[must_use = "await the returned future to wait for the loads"]
    pub fn track<T>(
        action: impl FnOnce() -> T,
    ) -> (T, impl Future<Output = ()> + Send) {
        struct Restore(Option<TransitionInner>);
        impl Drop for Restore {
            fn drop(&mut self) {
                TRANSITION.with_borrow_mut(|slot| *slot = self.0.take());
            }
        }

        let (tx, rx) = mpsc::channel();
        let value = {
            let _restore = TRANSITION.with_borrow_mut(|slot| {
                Restore(slot.replace(TransitionInner {
                    tx,
                    capture_notified: true,
                }))
            });
            action()
        };
        let pending = iter::from_fn(|| rx.try_recv().ok()).collect::<Vec<_>>();
        (value, async move {
            join_all(pending).await;
        })
    }

    /// Whether a transition started with [`run`](Self::run) or
    /// [`track`](Self::track) is currently active on this thread, so that an
    /// async resource created now is waited for by it.
    pub fn is_active() -> bool {
        TRANSITION.with_borrow(Option::is_some)
    }

    /// If a transition started with [`track`](Self::track) is active,
    /// registers a load with it and returns the sender that completes it.
    pub(crate) fn register_notified() -> Option<oneshot::Sender<()>> {
        TRANSITION.with_borrow(|current| {
            current.as_ref().filter(|inner| inner.capture_notified).map(
                |inner| {
                    let (tx, rx) = oneshot::channel();
                    _ = inner.tx.send(rx);
                    tx
                },
            )
        })
    }

    pub(crate) fn register(rx: oneshot::Receiver<()>) {
        TRANSITION.with_borrow(|current| {
            if let Some(inner) = current.as_ref() {
                // if it's an Err, that just means the Receiver was dropped
                // i.e., the transition is no longer listening, in which case it
                // doesn't matter if we successfully register with it or not
                _ = inner.tx.send(rx);
            }
        });
    }
}

pin_project! {
    /// Runs `action` and polls the future it produces with `inner` installed as
    /// the current transition for the duration of each poll, restoring the
    /// previous transition afterwards. The future is built lazily on the first
    /// poll so that `action` runs inside the transition scope.
    struct ScopedTransition<F, Fut> {
        inner: TransitionInner,
        action: Option<F>,
        #[pin]
        future: Option<Fut>,
    }
}

impl<F, Fut> Future for ScopedTransition<F, Fut>
where
    F: FnOnce() -> Fut,
    Fut: Future,
{
    type Output = Fut::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // RAII guard: restore the previous transition no matter how `poll`
        // exits (return, `?`, or a panic in the polled future).
        struct Restore(Option<TransitionInner>);
        impl Drop for Restore {
            fn drop(&mut self) {
                TRANSITION.with_borrow_mut(|slot| *slot = self.0.take());
            }
        }

        let mut this = self.project();
        let _restore = TRANSITION
            .with_borrow_mut(|slot| Restore(slot.replace(this.inner.clone())));
        if let Some(action) = this.action.take() {
            this.future.set(Some(action()));
        }
        this.future
            .as_pin_mut()
            .expect("ScopedTransition polled after completion")
            .poll(cx)
    }
}
