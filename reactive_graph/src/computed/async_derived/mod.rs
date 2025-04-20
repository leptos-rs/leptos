mod arc_async_derived;
pub use arc_async_derived::*;
#[allow(clippy::module_inception)] // not a pub mod, who cares?
mod async_derived;
mod future_impls;
mod inner;
use crate::{
    graph::{AnySubscriber, Observer, WithObserver},
    owner::Owner,
};
pub use async_derived::*;
pub use future_impls::*;
use futures::Future;
use pin_project_lite::pin_project;
use std::{
    pin::Pin,
    task::{Context, Poll},
};

pin_project! {
    /// A [`Future`] wrapper that sets the [`Owner`] and [`Observer`] before polling the inner
    /// `Future`.
    #[derive(Clone)]
    #[allow(missing_docs)]
    pub struct ScopedFuture<Fut> {
        pub owner: Owner,
        pub observer: Option<AnySubscriber>,
        #[pin]
        pub fut: Fut,
    }
}

impl<Fut> ScopedFuture<Fut> {
    /// Wraps the given `Future` by taking the current [`Owner`] and [`Observer`] and re-setting
    /// them as the active owner and observer every time the inner `Future` is polled.
    pub fn new(fut: Fut) -> Self {
        let owner = Owner::current().unwrap_or_default();
        let observer = Observer::get();
        Self {
            owner,
            observer,
            fut,
        }
    }
}

impl<Fut: Future> Future for ScopedFuture<Fut> {
    type Output = Fut::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        this.owner
            .with(|| this.observer.with_observer(|| this.fut.poll(cx)))
    }
}

/// Utilities used to track whether asynchronous computeds are currently loading.
pub mod suspense {
    use crate::{
        signal::ArcRwSignal,
        traits::{Update, Write},
    };
    use futures::channel::oneshot::Sender;
    use or_poisoned::OrPoisoned;
    use slotmap::{DefaultKey, SlotMap};
    use std::sync::{Arc, Mutex};

    /// Sends a one-time notification that the resource being read from is "local only," i.e.,
    /// that it will only run on the client, not the server.
    #[derive(Clone, Debug)]
    pub struct LocalResourceNotifier(Arc<Mutex<Option<Sender<()>>>>);

    impl LocalResourceNotifier {
        /// Send the notification. If the inner channel has already been used, this does nothing.
        pub fn notify(&mut self) {
            if let Some(tx) = self.0.lock().or_poisoned().take() {
                tx.send(()).unwrap();
            }
        }
    }

    impl From<Sender<()>> for LocalResourceNotifier {
        fn from(value: Sender<()>) -> Self {
            Self(Arc::new(Mutex::new(Some(value))))
        }
    }

    /// Tracks the collection of active async tasks.
    #[derive(Clone, Debug)]
    pub struct SuspenseContext {
        /// The set of active tasks.
        pub tasks: ArcRwSignal<SlotMap<DefaultKey, ()>>,
    }

    impl SuspenseContext {
        /// Generates a unique task ID.
        pub fn task_id(&self) -> TaskHandle {
            let key = self.tasks.write().insert(());
            TaskHandle {
                tasks: self.tasks.clone(),
                key,
            }
        }
    }

    /// A unique identifier that removes itself from the set of tasks when it is dropped.
    #[derive(Debug)]
    pub struct TaskHandle {
        tasks: ArcRwSignal<SlotMap<DefaultKey, ()>>,
        key: DefaultKey,
    }

    impl Drop for TaskHandle {
        fn drop(&mut self) {
            self.tasks.update(|tasks| {
                tasks.remove(self.key);
            });
        }
    }
}
