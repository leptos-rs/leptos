use super::{
    inner::{ArcAsyncDerivedInner, AsyncDerivedState},
    AsyncDerivedReadyFuture, ScopedFuture,
};
#[cfg(feature = "sandboxed-arenas")]
use crate::owner::Sandboxed;
use crate::{
    channel::channel,
    computed::suspense::SuspenseContext,
    diagnostics::SpecialNonReactiveFuture,
    graph::{
        AnySource, AnySubscriber, ReactiveNode, Source, SourceSet, Subscriber,
        SubscriberSet, ToAnySource, ToAnySubscriber, WithObserver,
    },
    owner::{use_context, Owner},
    signal::{
        guards::{AsyncPlain, ReadGuard, WriteGuard},
        ArcTrigger,
    },
    traits::{
        DefinedAt, IsDisposed, Notify, ReadUntracked, Track, UntrackableGuard,
        Write,
    },
    transition::AsyncTransition,
};
use any_spawner::Executor;
use async_lock::RwLock as AsyncRwLock;
use core::fmt::Debug;
use futures::{channel::oneshot, FutureExt, StreamExt};
use or_poisoned::OrPoisoned;
use send_wrapper::SendWrapper;
use std::{
    future::Future,
    mem,
    ops::DerefMut,
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
/// This is a reference-counted type, which is `Clone` but not `Copy`.
/// For arena-allocated `Copy` memos, use [`AsyncDerived`](super::AsyncDerived).
///
/// ## Examples
/// ```rust
/// # use reactive_graph::computed::*;
/// # use reactive_graph::signal::*; let owner = reactive_graph::owner::Owner::new(); owner.set();
/// # use reactive_graph::prelude::*;
/// # tokio_test::block_on(async move {
/// # any_spawner::Executor::init_tokio(); let owner = reactive_graph::owner::Owner::new(); owner.set();
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
/// assert_eq!(derived.await, 2);
/// # });
/// ```
///
/// ## Core Trait Implementations
/// - [`.get()`](crate::traits::Get) clones the current value as an `Option<T>`.
///   If you call it within an effect, it will cause that effect to subscribe
///   to the memo, and to re-run whenever the value of the memo changes.
///   - [`.get_untracked()`](crate::traits::GetUntracked) clones the value of
///     without reactively tracking it.
/// - [`.read()`](crate::traits::Read) returns a guard that allows accessing the
///   value by reference. If you call it within an effect, it will
///   cause that effect to subscribe to the memo, and to re-run whenever the
///   value changes.
///   - [`.read_untracked()`](crate::traits::ReadUntracked) gives access to the
///     current value without reactively tracking it.
/// - [`.with()`](crate::traits::With) allows you to reactively access the
///   value without cloning by applying a callback function.
///   - [`.with_untracked()`](crate::traits::WithUntracked) allows you to access
///     the value by applying a callback function without reactively
///     tracking it.
/// - [`IntoFuture`](std::future::Future) allows you to create a [`Future`] that resolves
///   when this resource is done loading.
pub struct ArcAsyncDerived<T> {
    #[cfg(any(debug_assertions, leptos_debuginfo))]
    pub(crate) defined_at: &'static Location<'static>,
    // the current state of this signal
    pub(crate) value: Arc<AsyncRwLock<Option<T>>>,
    // holds wakers generated when you .await this
    pub(crate) wakers: Arc<RwLock<Vec<Waker>>>,
    pub(crate) inner: Arc<RwLock<ArcAsyncDerivedInner>>,
    pub(crate) loading: Arc<AtomicBool>,
}

#[allow(dead_code)]
pub(crate) trait BlockingLock<T> {
    fn blocking_read_arc(self: &Arc<Self>)
        -> async_lock::RwLockReadGuardArc<T>;

    fn blocking_write_arc(
        self: &Arc<Self>,
    ) -> async_lock::RwLockWriteGuardArc<T>;

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

    fn blocking_write_arc(
        self: &Arc<Self>,
    ) -> async_lock::RwLockWriteGuardArc<T> {
        #[cfg(not(target_family = "wasm"))]
        {
            self.write_arc_blocking()
        }
        #[cfg(target_family = "wasm")]
        {
            self.write_arc().now_or_never().unwrap()
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
            #[cfg(any(debug_assertions, leptos_debuginfo))]
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
        #[cfg(any(debug_assertions, leptos_debuginfo))]
        f.field("defined_at", &self.defined_at);
        f.finish_non_exhaustive()
    }
}

impl<T> DefinedAt for ArcAsyncDerived<T> {
    #[inline(always)]
    fn defined_at(&self) -> Option<&'static Location<'static>> {
        #[cfg(any(debug_assertions, leptos_debuginfo))]
        {
            Some(self.defined_at)
        }
        #[cfg(not(any(debug_assertions, leptos_debuginfo)))]
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
    ($spawner:expr, $initial:ident, $fun:ident, $should_spawn:literal, $force_spawn:literal, $should_track:literal, $source:expr) => {{
        let (notifier, mut rx) = channel();

        let is_ready = $initial.is_some() && !$force_spawn;

        let owner = Owner::new();
        let inner = Arc::new(RwLock::new(ArcAsyncDerivedInner {
            owner: owner.clone(),
            notifier,
            sources: SourceSet::new(),
            subscribers: SubscriberSet::new(),
            state: AsyncDerivedState::Clean,
            version: 0,
            suspenses: Vec::new()
        }));
        let value = Arc::new(AsyncRwLock::new($initial));
        let wakers = Arc::new(RwLock::new(Vec::new()));

        let this = ArcAsyncDerived {
            #[cfg(any(debug_assertions, leptos_debuginfo))]
            defined_at: Location::caller(),
            value: Arc::clone(&value),
            wakers,
            inner: Arc::clone(&inner),
            loading: Arc::new(AtomicBool::new(!is_ready)),
        };
        let any_subscriber = this.to_any_subscriber();
        let initial_fut = if $should_track {
            owner.with_cleanup(|| {
                any_subscriber
                    .with_observer(|| ScopedFuture::new($fun()))
            })
        } else {
            owner.with_cleanup(|| {
                any_subscriber
                    .with_observer_untracked(|| ScopedFuture::new($fun()))
            })
        };
        #[cfg(feature = "sandboxed-arenas")]
        let initial_fut = Sandboxed::new(initial_fut);
        let mut initial_fut = Box::pin(initial_fut);

        let (was_ready, mut initial_fut) = {
            if is_ready {
                (true, None)
            } else {
                // if we don't already know that it's ready, we need to poll once, initially
                // so that the correct value is set synchronously
                let initial = initial_fut.as_mut().now_or_never();
                match initial {
                    None => {
                        inner.write().or_poisoned().notifier.notify();
                        (false, Some(initial_fut))
                    }
                    Some(orig_value) => {
                        let mut guard = this.inner.write().or_poisoned();

                        guard.state = AsyncDerivedState::Clean;
                        *value.blocking_write() = Some(orig_value);
                        this.loading.store(false, Ordering::Relaxed);
                        (true, None)
                    }
                }
            }
        };

        let mut first_run = {
            let (ready_tx, ready_rx) = oneshot::channel();
            if !was_ready {
                AsyncTransition::register(ready_rx);
            }
            Some(ready_tx)
        };

        if was_ready {
            first_run.take();
        }

        if let Some(source) = $source {
            any_subscriber.with_observer(|| source.track());
        }

        if $should_spawn {
            $spawner({
                let value = Arc::downgrade(&this.value);
                let inner = Arc::downgrade(&this.inner);
                let wakers = Arc::downgrade(&this.wakers);
                let loading = Arc::downgrade(&this.loading);
                let fut = async move {
                    // if the AsyncDerived has *already* been marked dirty (i.e., one of its
                    // sources has changed after creation), we should throw out the Future
                    // we already created, because its values might be stale
                    let already_dirty = inner.upgrade()
                        .as_ref()
                        .and_then(|inner| inner.read().ok())
                        .map(|inner| inner.state == AsyncDerivedState::Dirty)
                        .unwrap_or(false);
                    if already_dirty {
                        initial_fut.take();
                    }

                    while rx.next().await.is_some() {
                        let update_if_necessary = !owner.paused() && if $should_track {
                            any_subscriber
                                .with_observer(|| any_subscriber.update_if_necessary())
                        } else {
                            any_subscriber
                                .with_observer_untracked(|| any_subscriber.update_if_necessary())
                        };
                        if update_if_necessary || first_run.is_some() {
                            match (value.upgrade(), inner.upgrade(), wakers.upgrade(), loading.upgrade()) {
                                (Some(value), Some(inner), Some(wakers), Some(loading)) => {
                                    // generate new Future
                                    let owner = inner.read().or_poisoned().owner.clone();
                                    let fut = initial_fut.take().unwrap_or_else(|| {
                                        let fut = if $should_track {
                                            owner.with_cleanup(|| {
                                                any_subscriber
                                                    .with_observer(|| ScopedFuture::new($fun()))
                                            })
                                        } else {
                                            owner.with_cleanup(|| {
                                                any_subscriber
                                                    .with_observer_untracked(|| ScopedFuture::new($fun()))
                                            })
                                        };
                                        #[cfg(feature = "sandboxed-arenas")]
                                        let fut = Sandboxed::new(fut);
                                        Box::pin(fut)
                                    });

                                    // register with global transition listener, if any
                                    let ready_tx = first_run.take().unwrap_or_else(|| {
                                        let (ready_tx, ready_rx) = oneshot::channel();
                                        if !was_ready {
                                            AsyncTransition::register(ready_rx);
                                        }
                                        ready_tx
                                    });

                                    // generate and assign new value
                                    loading.store(true, Ordering::Relaxed);

                                    let (this_version, suspense_ids) = {
                                        let mut guard = inner.write().or_poisoned();
                                        guard.version += 1;
                                        let version = guard.version;
                                        let suspense_ids = mem::take(&mut guard.suspenses)
                                            .into_iter()
                                            .map(|sc| sc.task_id())
                                            .collect::<Vec<_>>();
                                        (version, suspense_ids)
                                    };

                                    let new_value = fut.await;

                                    drop(suspense_ids);

                                    let latest_version = inner.read().or_poisoned().version;

                                    if latest_version == this_version {
                                        Self::set_inner_value(new_value, value, wakers, inner, loading, Some(ready_tx)).await;
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
        }

        (this, is_ready)
    }};
}

impl<T: 'static> ArcAsyncDerived<T> {
    async fn set_inner_value(
        new_value: T,
        value: Arc<AsyncRwLock<Option<T>>>,
        wakers: Arc<RwLock<Vec<Waker>>>,
        inner: Arc<RwLock<ArcAsyncDerivedInner>>,
        loading: Arc<AtomicBool>,
        ready_tx: Option<oneshot::Sender<()>>,
    ) {
        *value.write().await = Some(new_value);
        Self::notify_subs(&wakers, &inner, &loading, ready_tx);
    }

    fn notify_subs(
        wakers: &Arc<RwLock<Vec<Waker>>>,
        inner: &Arc<RwLock<ArcAsyncDerivedInner>>,
        loading: &Arc<AtomicBool>,
        ready_tx: Option<oneshot::Sender<()>>,
    ) {
        loading.store(false, Ordering::Relaxed);

        let prev_state = mem::replace(
            &mut inner.write().or_poisoned().state,
            AsyncDerivedState::Notifying,
        );

        if let Some(ready_tx) = ready_tx {
            // if it's an Err, that just means the Receiver was dropped
            // we don't particularly care about that: the point is to notify if
            // it still exists, but we don't need to know if Suspense is no
            // longer listening
            _ = ready_tx.send(());
        }

        // notify reactive subscribers that we're not loading any more
        for sub in (&inner.read().or_poisoned().subscribers).into_iter() {
            sub.mark_dirty();
        }

        // notify async .awaiters
        for waker in mem::take(&mut *wakers.write().or_poisoned()) {
            waker.wake();
        }

        // if this was marked dirty before notifications began, this means it
        // had been notified while loading; marking it clean will cause it not to
        // run on the next tick of the async loop, so here it should be left dirty
        inner.write().or_poisoned().state = prev_state;
    }
}

impl<T: 'static> ArcAsyncDerived<T> {
    /// Creates a new async derived computation.
    ///
    /// This runs eagerly: i.e., calls `fun` once when created and immediately spawns the `Future`
    /// as a new task.
    #[track_caller]
    pub fn new<Fut>(fun: impl Fn() -> Fut + Send + Sync + 'static) -> Self
    where
        T: Send + Sync + 'static,
        Fut: Future<Output = T> + Send + 'static,
    {
        Self::new_with_initial(None, fun)
    }

    /// Creates a new async derived computation with an initial value as a fallback, and begins running the
    /// `Future` eagerly to get the actual first value.
    #[track_caller]
    pub fn new_with_initial<Fut>(
        initial_value: Option<T>,
        fun: impl Fn() -> Fut + Send + Sync + 'static,
    ) -> Self
    where
        T: Send + Sync + 'static,
        Fut: Future<Output = T> + Send + 'static,
    {
        let (this, _) = spawn_derived!(
            Executor::spawn,
            initial_value,
            fun,
            true,
            true,
            true,
            None::<ArcTrigger>
        );
        this
    }

    /// Creates a new async derived computation with an initial value, and does not spawn a task
    /// initially.
    ///
    /// This is mostly used with manual dependency tracking, for primitives built on top of this
    /// where you do not want to run the run the `Future` unnecessarily.
    #[doc(hidden)]
    #[track_caller]
    pub fn new_with_manual_dependencies<Fut, S>(
        initial_value: Option<T>,
        fun: impl Fn() -> Fut + Send + Sync + 'static,
        source: &S,
    ) -> Self
    where
        T: Send + Sync + 'static,
        Fut: Future<Output = T> + Send + 'static,
        S: Track,
    {
        let (this, _) = spawn_derived!(
            Executor::spawn,
            initial_value,
            fun,
            true,
            false,
            false,
            Some(source)
        );
        this
    }

    /// Creates a new async derived computation that will be guaranteed to run on the current
    /// thread.
    ///
    /// This runs eagerly: i.e., calls `fun` once when created and immediately spawns the `Future`
    /// as a new task.
    #[track_caller]
    pub fn new_unsync<Fut>(fun: impl Fn() -> Fut + 'static) -> Self
    where
        T: 'static,
        Fut: Future<Output = T> + 'static,
    {
        Self::new_unsync_with_initial(None, fun)
    }

    /// Creates a new async derived computation with an initial value as a fallback, and begins running the
    /// `Future` eagerly to get the actual first value.
    #[track_caller]
    pub fn new_unsync_with_initial<Fut>(
        initial_value: Option<T>,
        fun: impl Fn() -> Fut + 'static,
    ) -> Self
    where
        T: 'static,
        Fut: Future<Output = T> + 'static,
    {
        let (this, _) = spawn_derived!(
            Executor::spawn_local,
            initial_value,
            fun,
            true,
            true,
            true,
            None::<ArcTrigger>
        );
        this
    }

    /// Returns a `Future` that is ready when this resource has next finished loading.
    pub fn ready(&self) -> AsyncDerivedReadyFuture {
        AsyncDerivedReadyFuture::new(
            self.to_any_source(),
            &self.loading,
            &self.wakers,
        )
    }
}

impl<T: 'static> ArcAsyncDerived<SendWrapper<T>> {
    #[doc(hidden)]
    #[track_caller]
    pub fn new_mock<Fut>(fun: impl Fn() -> Fut + 'static) -> Self
    where
        T: 'static,
        Fut: Future<Output = T> + 'static,
    {
        let initial = None::<SendWrapper<T>>;
        let fun = move || {
            let fut = fun();
            async move {
                let value = fut.await;
                SendWrapper::new(value)
            }
        };
        let (this, _) = spawn_derived!(
            Executor::spawn_local,
            initial,
            fun,
            false,
            false,
            true,
            None::<ArcTrigger>
        );
        this
    }
}

impl<T: 'static> ReadUntracked for ArcAsyncDerived<T> {
    type Value = ReadGuard<Option<T>, AsyncPlain<Option<T>>>;

    fn try_read_untracked(&self) -> Option<Self::Value> {
        if let Some(suspense_context) = use_context::<SuspenseContext>() {
            let handle = suspense_context.task_id();
            let ready = SpecialNonReactiveFuture::new(self.ready());
            crate::spawn(async move {
                ready.await;
                drop(handle);
            });
            self.inner
                .write()
                .or_poisoned()
                .suspenses
                .push(suspense_context);
        }
        AsyncPlain::try_new(&self.value).map(ReadGuard::new)
    }
}

impl<T: 'static> Notify for ArcAsyncDerived<T> {
    fn notify(&self) {
        Self::notify_subs(&self.wakers, &self.inner, &self.loading, None);
    }
}

impl<T: 'static> Write for ArcAsyncDerived<T> {
    type Value = Option<T>;

    fn try_write(&self) -> Option<impl UntrackableGuard<Target = Self::Value>> {
        Some(WriteGuard::new(self.clone(), self.value.blocking_write()))
    }

    fn try_write_untracked(
        &self,
    ) -> Option<impl DerefMut<Target = Self::Value>> {
        Some(self.value.blocking_write())
    }
}

impl<T: 'static> IsDisposed for ArcAsyncDerived<T> {
    #[inline(always)]
    fn is_disposed(&self) -> bool {
        false
    }
}

impl<T: 'static> ToAnySource for ArcAsyncDerived<T> {
    fn to_any_source(&self) -> AnySource {
        AnySource(
            Arc::as_ptr(&self.inner) as usize,
            Arc::downgrade(&self.inner) as Weak<dyn Source + Send + Sync>,
            #[cfg(any(debug_assertions, leptos_debuginfo))]
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
