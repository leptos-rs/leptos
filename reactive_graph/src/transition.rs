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
        let inner = TransitionInner { tx };

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
    /// that started loading, or was notified that it may need to reload,
    /// while `action` ran has finished loading.
    ///
    /// This is the synchronous counterpart of [`run`](Self::run): `action`
    /// runs to completion before this returns, and only what happens inside
    /// it is captured. A resource is captured if it is created during
    /// `action`, or if a signal update inside `action` notifies it (for
    /// example, a resource keyed on route params when the params change);
    /// a notification that turns out not to need a reload is not waited for.
    /// Work started later by effects that run because of those updates is
    /// not captured. Dropping the returned future stops waiting, but does not
    /// cancel the loads.
    ///
    /// A transition started inside another one is separate from it: what it
    /// captures is not added to the outer transition.
    #[must_use = "await the returned future to wait for the captured loads"]
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
                Restore(slot.replace(TransitionInner { tx }))
            });
            action()
        };
        let pending = iter::from_fn(|| rx.try_recv().ok()).collect::<Vec<_>>();
        (value, async move {
            join_all(pending).await;
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

    /// If a transition is active, registers a new pending load with it and
    /// returns the sender that completes it; otherwise returns `None`.
    pub(crate) fn register_pending() -> Option<oneshot::Sender<()>> {
        TRANSITION.with_borrow(|current| {
            current.as_ref().map(|inner| {
                let (tx, rx) = oneshot::channel();
                _ = inner.tx.send(rx);
                tx
            })
        })
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
