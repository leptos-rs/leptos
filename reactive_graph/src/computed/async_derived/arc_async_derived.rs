use super::{
    inner::ArcAsyncDerivedInner, AsyncDerivedReadyFuture, ScopedFuture,
};
#[cfg(feature = "sandboxed-arenas")]
use crate::owner::Sandboxed;
use crate::{
    channel::channel,
    computed::suspense::SuspenseContext,
    diagnostics::SpecialNonReactiveFuture,
    graph::{
        AnySource, AnySubscriber, ReactiveNode, Source, SourceSet, Subscriber,
        SubscriberSet, ToAnySource, ToAnySubscriber,
    },
    owner::{use_context, Owner},
    signal::guards::{AsyncPlain, ReadGuard},
    traits::{DefinedAt, ReadUntracked},
    transition::AsyncTransition,
};
use any_spawner::Executor;
use async_lock::RwLock as AsyncRwLock;
use core::fmt::Debug;
use futures::{channel::oneshot, FutureExt, StreamExt};
use or_poisoned::OrPoisoned;
use std::{
    future::Future,
    mem,
    panic::Location,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, RwLock, Weak,
    },
    task::Waker,
};

/// A reactive value that is derived by running an asynchronous computation in response to changes
/// in its sources.
///
/// When one of its dependencies changes, this will re-run its async computation, then notify other
/// values that depend on it that it has changed.
///
/// ## Examples
/// ```rust
/// # use reactive_graph::computed::*;
/// # use reactive_graph::signal::*;
/// # use reactive_graph::prelude::*;
/// # tokio_test::block_on(async move {
/// # any_spawner::Executor::init_tokio();
/// # let _guard = reactive_graph::diagnostics::SpecialNonReactiveZone::enter();
///
/// let signal1 = RwSignal::new(0);
/// let signal2 = RwSignal::new(0);
/// let derived = ArcAsyncDerived::new(move || async move {
///   // reactive values can be tracked anywhere in the `async` block
///   let value1 = signal1.get();
///   tokio::time::sleep(std::time::Duration::from_millis(25)).await;
///   let value2 = signal2.get();
///
///   value1 + value2
/// });
///
/// // the value can be accessed synchronously as `Option<T>`
/// assert_eq!(derived.get(), None);
/// // we can also .await the value, i.e., convert it into a Future
/// assert_eq!(derived.clone().await, 0);
/// assert_eq!(derived.get(), Some(0));
///
/// signal1.set(1);
/// // while the new value is still pending, the signal holds the old value
/// tokio::time::sleep(std::time::Duration::from_millis(5)).await;
/// assert_eq!(derived.get(), Some(0));
///
/// // setting multiple dependencies will hold until the latest change is ready
/// signal2.set(1);
/// assert_eq!(derived.clone().await, 2);
/// # });
/// ```
pub struct ArcAsyncDerived<T> {
    #[cfg(debug_assertions)]
    pub(crate) defined_at: &'static Location<'static>,
    // the current state of this signal
    pub(crate) value: Arc<AsyncRwLock<Option<T>>>,
    // holds wakers generated when you .await this
    pub(crate) wakers: Arc<RwLock<Vec<Waker>>>,
    pub(crate) inner: Arc<RwLock<ArcAsyncDerivedInner>>,
    pub(crate) loading: Arc<AtomicBool>,
}

pub(crate) trait BlockingLock<T> {
    fn blocking_read_arc(self: &Arc<Self>)
        -> async_lock::RwLockReadGuardArc<T>;

    fn blocking_read(&self) -> async_lock::RwLockReadGuard<'_, T>;

    fn blocking_write(&self) -> async_lock::RwLockWriteGuard<'_, T>;
}

impl<T> BlockingLock<T> for AsyncRwLock<T> {
    fn blocking_read_arc(
        self: &Arc<Self>,
    ) -> async_lock::RwLockReadGuardArc<T> {
        #[cfg(not(target_family = "wasm"))]
        {
            self.read_arc_blocking()
        }
        #[cfg(target_family = "wasm")]
        {
            self.read_arc().now_or_never().unwrap()
        }
    }

    fn blocking_read(&self) -> async_lock::RwLockReadGuard<'_, T> {
        #[cfg(not(target_family = "wasm"))]
        {
            self.read_blocking()
        }
        #[cfg(target_family = "wasm")]
        {
            self.read().now_or_never().unwrap()
        }
    }

    fn blocking_write(&self) -> async_lock::RwLockWriteGuard<'_, T> {
        #[cfg(not(target_family = "wasm"))]
        {
            self.write_blocking()
        }
        #[cfg(target_family = "wasm")]
        {
            self.write().now_or_never().unwrap()
        }
    }
}

impl<T> Clone for ArcAsyncDerived<T> {
    fn clone(&self) -> Self {
        Self {
            #[cfg(debug_assertions)]
            defined_at: self.defined_at,
            value: Arc::clone(&self.value),
            wakers: Arc::clone(&self.wakers),
            inner: Arc::clone(&self.inner),
            loading: Arc::clone(&self.loading),
        }
    }
}

impl<T> Debug for ArcAsyncDerived<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut f = f.debug_struct("ArcAsyncDerived");
        #[cfg(debug_assertions)]
        f.field("defined_at", &self.defined_at);
        f.finish_non_exhaustive()
    }
}

impl<T> DefinedAt for ArcAsyncDerived<T> {
    #[inline(always)]
    fn defined_at(&self) -> Option<&'static Location<'static>> {
        #[cfg(debug_assertions)]
        {
            Some(self.defined_at)
        }
        #[cfg(not(debug_assertions))]
        {
            None
        }
    }
}

// This helps create a derived async signal.
// It needs to be implemented as a macro because it needs to be flexible over
// whether `fun` returns a `Future` that is `Send`. Doing it as a function would,
// as far as I can tell, require repeating most of the function body.
macro_rules! spawn_derived {
    ($spawner:expr, $initial:ident, $fun:ident) => {{
        let (notifier, mut rx) = channel();

        let is_ready = $initial.is_some();

        let owner = Owner::new();
        let inner = Arc::new(RwLock::new(ArcAsyncDerivedInner {
            owner: owner.clone(),
            notifier,
            sources: SourceSet::new(),
            subscribers: SubscriberSet::new(),
            dirty: false
        }));
        let value = Arc::new(AsyncRwLock::new($initial));
        let wakers = Arc::new(RwLock::new(Vec::new()));

        let this = ArcAsyncDerived {
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
            value: Arc::clone(&value),
            wakers,
            inner: Arc::clone(&inner),
            loading: Arc::new(AtomicBool::new(!is_ready)),
        };
        let any_subscriber = this.to_any_subscriber();

        let was_ready = {
            if is_ready {
                true
            } else {
                // if we don't already know that it's ready, we need to poll once, initially
                // so that the correct value is set synchronously
                let fut = owner.with_cleanup(|| {
                    any_subscriber
                        .with_observer(|| ScopedFuture::new($fun()))
                });
                #[cfg(feature = "sandboxed-arenas")]
                let fut = Sandboxed::new(fut);
                let mut fut = Box::pin(fut);
                let initial = fut.as_mut().now_or_never();
                match initial {
                    None => false,
                    Some(orig_value) => {
                        let mut guard = this.inner.write().or_poisoned();

                        guard.dirty = false;
                        *value.blocking_write() = Some(orig_value);
                        this.loading.store(false, Ordering::Relaxed);
                        true
                    }
                }
            }
        };

        let mut first_run = {
            let (ready_tx, ready_rx) = oneshot::channel();
            AsyncTransition::register(ready_rx);
            Some(ready_tx)
        };

        // begin loading eagerly but asynchronously, if not already loaded
        if !was_ready {
            any_subscriber.mark_dirty();
        }

        $spawner({
            let value = Arc::downgrade(&this.value);
            let inner = Arc::downgrade(&this.inner);
            let wakers = Arc::downgrade(&this.wakers);
            let loading = Arc::downgrade(&this.loading);
            let fut = async move {
                while rx.next().await.is_some() {
                    if first_run.is_some() || any_subscriber.update_if_necessary() {
                        match (value.upgrade(), inner.upgrade(), wakers.upgrade(), loading.upgrade()) {
                            (Some(value), Some(inner), Some(wakers), Some(loading)) => {
                                // generate new Future
                                let owner = inner.read().or_poisoned().owner.clone();
                                let fut = owner.with_cleanup(|| {
                                    any_subscriber
                                        .with_observer(|| ScopedFuture::new($fun()))
                                });
                                #[cfg(feature = "sandboxed-arenas")]
                                let fut = Sandboxed::new(fut);

                                // register with global transition listener, if any
                                let ready_tx = first_run.take().unwrap_or_else(|| {
                                    let (ready_tx, ready_rx) = oneshot::channel();
                                    AsyncTransition::register(ready_rx);
                                    ready_tx
                                });


                                // notify reactive subscribers that we're now loading
                                loading.store(true, Ordering::Relaxed);
                                inner.write().or_poisoned().dirty = true;
                                for sub in (&inner.read().or_poisoned().subscribers).into_iter() {
                                    sub.mark_check();
                                }

                                // generate and assign new value
                                let new_value = fut.await;
                                loading.store(false, Ordering::Relaxed);
                                *value.write().await = Some(new_value);
                                inner.write().or_poisoned().dirty = true;

                                // if it's an Err, that just means the Receiver was dropped
                                // we don't particularly care about that: the point is to notify if
                                // it still exists, but we don't need to know if Suspense is no
                                // longer listening
                                _ = ready_tx.send(());

                                // notify reactive subscribers that we're not loading any more
                                for sub in (&inner.read().or_poisoned().subscribers).into_iter() {
                                    sub.mark_check();
                                }

                                // notify async .awaiters
                                for waker in mem::take(&mut *wakers.write().or_poisoned()) {
                                    waker.wake();
                                }
                            }
                            _ => break,
                        }
                    }
                }
            };

            #[cfg(feature = "sandboxed-arenas")]
            let fut = Sandboxed::new(fut);

            fut
        });

        (this, is_ready)
    }};
}

impl<T: 'static> ArcAsyncDerived<T> {
    #[track_caller]
    pub fn new<Fut>(fun: impl Fn() -> Fut + Send + Sync + 'static) -> Self
    where
        T: Send + Sync + 'static,
        Fut: Future<Output = T> + Send + 'static,
    {
        Self::new_with_initial(None, fun)
    }

    #[track_caller]
    pub fn new_with_initial<Fut>(
        initial_value: Option<T>,
        fun: impl Fn() -> Fut + Send + Sync + 'static,
    ) -> Self
    where
        T: Send + Sync + 'static,
        Fut: Future<Output = T> + Send + 'static,
    {
        let (this, _) = spawn_derived!(Executor::spawn, initial_value, fun);
        this
    }

    #[track_caller]
    pub fn new_unsync<Fut>(fun: impl Fn() -> Fut + 'static) -> Self
    where
        T: 'static,
        Fut: Future<Output = T> + 'static,
    {
        Self::new_unsync_with_initial(None, fun)
    }

    #[track_caller]
    pub fn new_unsync_with_initial<Fut>(
        initial_value: Option<T>,
        fun: impl Fn() -> Fut + 'static,
    ) -> Self
    where
        T: 'static,
        Fut: Future<Output = T> + 'static,
    {
        let (this, _) =
            spawn_derived!(Executor::spawn_local, initial_value, fun);
        this
    }

    pub fn ready(&self) -> AsyncDerivedReadyFuture {
        AsyncDerivedReadyFuture {
            source: self.to_any_source(),
            loading: Arc::clone(&self.loading),
            wakers: Arc::clone(&self.wakers),
        }
    }
}

impl<T: Send + Sync + 'static> ReadUntracked for ArcAsyncDerived<T> {
    type Value = ReadGuard<Option<T>, AsyncPlain<Option<T>>>;

    fn try_read_untracked(&self) -> Option<Self::Value> {
        if let Some(suspense_context) = use_context::<SuspenseContext>() {
            if self.value.blocking_read().is_none() {
                let handle = suspense_context.task_id();
                let ready = SpecialNonReactiveFuture::new(self.ready());
                Executor::spawn(async move {
                    ready.await;
                    drop(handle);
                });
            }
        }
        AsyncPlain::try_new(&self.value).map(ReadGuard::new)
    }
}

impl<T: 'static> ToAnySource for ArcAsyncDerived<T> {
    fn to_any_source(&self) -> AnySource {
        AnySource(
            Arc::as_ptr(&self.inner) as usize,
            Arc::downgrade(&self.inner) as Weak<dyn Source + Send + Sync>,
            #[cfg(debug_assertions)]
            self.defined_at,
        )
    }
}

impl<T: 'static> ToAnySubscriber for ArcAsyncDerived<T> {
    fn to_any_subscriber(&self) -> AnySubscriber {
        AnySubscriber(
            Arc::as_ptr(&self.inner) as usize,
            Arc::downgrade(&self.inner) as Weak<dyn Subscriber + Send + Sync>,
        )
    }
}

impl<T> Source for ArcAsyncDerived<T> {
    fn add_subscriber(&self, subscriber: AnySubscriber) {
        self.inner.add_subscriber(subscriber);
    }

    fn remove_subscriber(&self, subscriber: &AnySubscriber) {
        self.inner.remove_subscriber(subscriber);
    }

    fn clear_subscribers(&self) {
        self.inner.clear_subscribers();
    }
}

impl<T> ReactiveNode for ArcAsyncDerived<T> {
    fn mark_dirty(&self) {
        self.inner.mark_dirty();
    }

    fn mark_check(&self) {
        self.inner.mark_check();
    }

    fn mark_subscribers_check(&self) {
        self.inner.mark_subscribers_check();
    }

    fn update_if_necessary(&self) -> bool {
        self.inner.update_if_necessary()
    }
}

impl<T> Subscriber for ArcAsyncDerived<T> {
    fn add_source(&self, source: AnySource) {
        self.inner.add_source(source);
    }

    fn clear_sources(&self, subscriber: &AnySubscriber) {
        self.inner.clear_sources(subscriber);
    }
}
