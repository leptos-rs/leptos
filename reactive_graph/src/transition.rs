//! Utilities to wait for asynchronous primitives to resolve.

use futures::{channel::oneshot, future::join_all};
use pin_project_lite::pin_project;
use std::{
    cell::RefCell,
    future::Future,
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
