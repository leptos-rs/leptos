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

    /// Tracks the `<Suspense>`/`<Transition>` boundaries built for a route
    /// during a router navigation, so `<Router set_is_routing>` can keep
    /// `is_routing` set until they have all settled.
    ///
    /// This is router plumbing shared between the router and the boundary
    /// components, not an application-facing API. A boundary built for the
    /// navigation registers a task at construction and releases it once it
    /// has built its children (so nested boundaries register before their
    /// parent releases) and has no pending tasks of its own; registering does
    /// **not** affect what the boundary displays. The router polls the context
    /// with [`poll_settled`](Self::poll_settled), which reports the route as
    /// settled once every task has been released.
    ///
    /// The context is *closed* once the view has settled, or once the router
    /// has replaced the view with another one: [`task`](Self::task) then
    /// returns `None`, so boundaries created later (for example by user
    /// interaction inside the route, which can still find the context via
    /// `use_context`) do not register.
    #[doc(hidden)]
    #[derive(Clone, Debug)]
    pub struct RouteSettleContext {
        state: Arc<Mutex<RouteSettleState>>,
    }

    #[derive(Debug)]
    struct RouteSettleState {
        open: bool,
        pending: usize,
        wakers: Vec<Waker>,
    }

    impl Default for RouteSettleContext {
        fn default() -> Self {
            Self::new()
        }
    }

    impl RouteSettleContext {
        /// Creates a new, open context with no tasks.
        pub fn new() -> Self {
            Self {
                state: Arc::new(Mutex::new(RouteSettleState {
                    open: true,
                    pending: 0,
                    wakers: Vec::new(),
                })),
            }
        }

        /// Registers a task that keeps the route "unsettled" until the returned
        /// handle is dropped, or `None` if the context has been closed.
        pub fn task(&self) -> Option<RouteSettleTask> {
            let mut state = self.state.lock().or_poisoned();
            if !state.open {
                return None;
            }
            state.pending += 1;
            Some(RouteSettleTask {
                state: Arc::clone(&self.state),
            })
        }

        /// Closes the context, so that [`task`](Self::task) no longer registers
        /// anything, and wakes anyone waiting on
        /// [`poll_settled`](Self::poll_settled). Used by the router when the
        /// view the context belongs to is replaced before it settles; a view
        /// that settles is closed by `poll_settled` itself.
        pub fn close(&self) {
            let wakers = {
                let mut state = self.state.lock().or_poisoned();
                state.open = false;
                mem::take(&mut state.wakers)
            };
            for waker in wakers {
                waker.wake();
            }
        }

        /// Whether every registered task has been released, or the context has
        /// been closed.
        ///
        /// Observing the route as settled and closing the context happen under
        /// the same lock, so no task can register in between. If this returns
        /// `false`, `waker` will be woken when the last [`RouteSettleTask`] is
        /// dropped or the context is closed.
        pub fn poll_settled(&self, waker: &Waker) -> bool {
            let mut state = self.state.lock().or_poisoned();
            if !state.open {
                return true;
            }
            if state.pending == 0 {
                state.open = false;
                state.wakers.clear();
                return true;
            }
            if !state.wakers.iter().any(|w| w.will_wake(waker)) {
                state.wakers.push(waker.clone());
            }
            false
        }
    }

    /// A task registered with a [`RouteSettleContext`]; releases itself when
    /// dropped.
    #[doc(hidden)]
    #[derive(Debug)]
    pub struct RouteSettleTask {
        state: Arc<Mutex<RouteSettleState>>,
    }

    impl Drop for RouteSettleTask {
        fn drop(&mut self) {
            let wakers = {
                let mut state = self.state.lock().or_poisoned();
                debug_assert!(state.pending > 0);
                state.pending = state.pending.saturating_sub(1);
                if state.pending == 0 {
                    mem::take(&mut state.wakers)
                } else {
                    Vec::new()
                }
            };
            for waker in wakers {
                waker.wake();
            }
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
