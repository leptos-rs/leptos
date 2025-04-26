//! Utilities to wait for asynchronous primitives to resolve.

use futures::{channel::oneshot, future::join_all};
use pin_project_lite::pin_project;
use std::{cell::RefCell, future::Future, sync::mpsc};

thread_local! {
    static TRANSITION: RefCell<Option<TransitionInner>> = RefCell::new(None);
}

/// A Drop guard is needed because drop is called even in case of a panic
struct TransitionGuard<'a>(&'a mut Option<TransitionInner>);
impl<'a> TransitionGuard<'a> {
    fn new(value: &'a mut Option<TransitionInner>) -> Self {
        TRANSITION.with(|transaction| {
            std::mem::swap(&mut *transaction.borrow_mut(), value)
        });
        Self(value)
    }
}
impl Drop for TransitionGuard<'_> {
    fn drop(&mut self) {
        TRANSITION.with(|transaction| {
            std::mem::swap(&mut *transaction.borrow_mut(), self.0)
        });
    }
}

// A future wrapper, to use in async functions
pin_project! {
    struct WithTransition<Fut>{
        transition: Option<TransitionInner>,
        #[pin]
        inner: Fut
    }
}
impl<Fut> Future for WithTransition<Fut>
where
    Fut: Future,
{
    type Output = <Fut as Future>::Output;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = self.project();
        let _guard = TransitionGuard::new(this.transition);
        this.inner.poll(cx)
    }
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
        let transition = Some(TransitionInner { tx });
        let value = WithTransition {
            transition,
            inner: action(),
        }
        .await;

        let mut pending = Vec::new();
        // This should never block since all tx instances have been dropped
        while let Ok(tx) = rx.recv() {
            pending.push(tx);
        }
        join_all(pending).await;
        value
    }

    pub(crate) fn register(rx: oneshot::Receiver<()>) {
        TRANSITION.with_borrow(|transition| {
            if let Some(transition) = transition {
                // if it's an Err, that just means the Receiver was dropped
                // i.e., the transition is no longer listening, in which case it doesn't matter if we
                // successfully register with it or not
                _ = transition.tx.send(rx);
            }
        })
    }
}
