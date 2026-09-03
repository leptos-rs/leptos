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
        owner: Owner,
        observer: Option<AnySubscriber>,
        diagnostics: bool,
        #[pin]
        fut: Fut,
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
            diagnostics: true,
            fut,
        }
    }

    /// Wraps the given `Future` by taking the current [`Owner`] re-setting it as the
    /// active owner every time the inner `Future` is polled. Always untracks, i.e., clears
    /// the active [`Observer`] when polled.
    pub fn new_untracked(fut: Fut) -> Self {
        let owner = Owner::current().unwrap_or_default();
        Self {
            owner,
            observer: None,
            diagnostics: false,
            fut,
        }
    }

    #[doc(hidden)]
    #[track_caller]
    pub fn new_untracked_with_diagnostics(fut: Fut) -> Self {
        let owner = Owner::current().unwrap_or_default();
        Self {
            owner,
            observer: None,
            diagnostics: true,
            fut,
        }
    }
}

impl<Fut: Future> Future for ScopedFuture<Fut> {
    type Output = Fut::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        this.owner.with(|| {
            this.observer.with_observer(|| {
                #[cfg(debug_assertions)]
                let _maybe_guard = if *this.diagnostics {
                    None
                } else {
                    Some(crate::diagnostics::SpecialNonReactiveZone::enter())
                };
                this.fut.poll(cx)
            })
        })
    }
}

/// Utilities used to track whether asynchronous computeds are currently loading.
pub mod suspense {
    use crate::{
        signal::ArcRwSignal,
        traits::{ReadUntracked, Update, Write},
    };
    use futures::channel::oneshot::Sender;
    use or_poisoned::OrPoisoned;
    use slotmap::{DefaultKey, SlotMap};
    use std::{
        mem,
        sync::{Arc, Mutex},
        task::Waker,
    };

    /// Sends a one-time notification that the resource being read from is "local only," i.e.,
    /// that it will only run on the client, not the server.
    #[derive(Clone, Debug)]
    pub struct LocalResourceNotifier(Arc<Mutex<Option<Sender<()>>>>);

    impl LocalResourceNotifier {
        /// Send the notification. If the inner channel has already been used, this does nothing.
        pub fn notify(&mut self) {
            if let Some(tx) = self.0.lock().or_poisoned().take()
                && tx.send(()).is_err()
            {
                crate::log_warning(format_args!(
                    "A local-resource notification could not be delivered \
                     because its listener was already dropped."
                ));
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
        empty_wakers: Arc<Mutex<Vec<Waker>>>,
    }

    impl SuspenseContext {
        /// Creates a context that tracks the given set of active tasks.
        pub fn new(tasks: ArcRwSignal<SlotMap<DefaultKey, ()>>) -> Self {
            Self {
                tasks,
                empty_wakers: Default::default(),
            }
        }

        /// Generates a unique task ID.
        pub fn task_id(&self) -> TaskHandle {
            let key = self.tasks.write().insert(());
            TaskHandle {
                tasks: self.tasks.clone(),
                empty_wakers: Arc::clone(&self.empty_wakers),
                key,
            }
        }

        /// Whether the set of active tasks is currently empty.
        ///
        /// If not, `waker` will be woken when the last [`TaskHandle`] is dropped.
        pub fn poll_empty(&self, waker: &Waker) -> bool {
            let mut wakers = self.empty_wakers.lock().or_poisoned();
            let empty = self
                .tasks
                .try_read_untracked()
                .map(|tasks| tasks.is_empty())
                .unwrap_or(false);
            if empty {
                wakers.clear();
            } else if !wakers.iter().any(|w| w.will_wake(waker)) {
                wakers.push(waker.clone());
            }
            empty
        }
    }

    /// A unique identifier that removes itself from the set of tasks when it is dropped.
    #[derive(Debug)]
    pub struct TaskHandle {
        tasks: ArcRwSignal<SlotMap<DefaultKey, ()>>,
        empty_wakers: Arc<Mutex<Vec<Waker>>>,
        key: DefaultKey,
    }

    impl Drop for TaskHandle {
        fn drop(&mut self) {
            let mut now_empty = false;
            self.tasks.update(|tasks| {
                tasks.remove(self.key);
                now_empty = tasks.is_empty();
            });
            if now_empty {
                for waker in
                    mem::take(&mut *self.empty_wakers.lock().or_poisoned())
                {
                    waker.wake();
                }
            }
        }
    }

    /// A [`TaskHandle`] that can be released from one of multiple places.
    #[derive(Clone, Debug)]
    pub struct SharedTaskHandle(Arc<Mutex<Option<TaskHandle>>>);

    impl SharedTaskHandle {
        /// Wraps a handle so that it can be released from more than one place.
        pub fn new(handle: TaskHandle) -> Self {
            Self(Arc::new(Mutex::new(Some(handle))))
        }

        /// Drops the inner handle, if it has not been dropped already.
        pub fn release(&self) {
            drop(self.0.lock().or_poisoned().take());
        }
    }
}
