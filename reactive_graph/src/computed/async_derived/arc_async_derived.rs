use super::{
    inner::ArcAsyncDerivedInner, AsyncDerivedReadyFuture, AsyncState,
    ScopedFuture,
};
#[cfg(feature = "sandboxed-arenas")]
use crate::owner::Sandboxed;
use crate::{
    channel::channel,
    graph::{
        AnySource, AnySubscriber, ReactiveNode, Source, SourceSet, Subscriber,
        SubscriberSet, ToAnySource, ToAnySubscriber,
    },
    owner::Owner,
    signal::guards::{Plain, ReadGuard},
    traits::{DefinedAt, ReadUntracked},
};
use any_spawner::Executor;
use core::fmt::Debug;
use futures::StreamExt;
use or_poisoned::OrPoisoned;
use std::{
    future::Future,
    mem,
    panic::Location,
    sync::{Arc, RwLock, Weak},
    task::Waker,
};

pub struct ArcAsyncDerived<T> {
    #[cfg(debug_assertions)]
    pub(crate) defined_at: &'static Location<'static>,
    // the current state of this signal
    pub(crate) value: Arc<RwLock<AsyncState<T>>>,
    // holds wakers generated when you .await this
    pub(crate) wakers: Arc<RwLock<Vec<Waker>>>,
    pub(crate) inner: Arc<RwLock<ArcAsyncDerivedInner>>,
}

impl<T> Clone for ArcAsyncDerived<T> {
    fn clone(&self) -> Self {
        Self {
            #[cfg(debug_assertions)]
            defined_at: self.defined_at,
            value: Arc::clone(&self.value),
            wakers: Arc::clone(&self.wakers),
            inner: Arc::clone(&self.inner),
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
        let (mut notifier, mut rx) = channel();

        // begin loading eagerly but asynchronously, if not already loaded
        if matches!($initial, AsyncState::Loading) {
            notifier.notify();
        }
        let is_ready = matches!($initial, AsyncState::Complete(_));

        let inner = Arc::new(RwLock::new(ArcAsyncDerivedInner {
            owner: Owner::new(),
            notifier,
            sources: SourceSet::new(),
            subscribers: SubscriberSet::new(),
        }));
        let value = Arc::new(RwLock::new($initial));
        let wakers = Arc::new(RwLock::new(Vec::new()));

        let this = ArcAsyncDerived {
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
            value,
            wakers,
            inner: Arc::clone(&inner),
        };
        let any_subscriber = this.to_any_subscriber();

        $spawner({
            let value = Arc::downgrade(&this.value);
            let inner = Arc::downgrade(&this.inner);
            let wakers = Arc::downgrade(&this.wakers);
            let fut = async move {
                while rx.next().await.is_some() {
                    match (value.upgrade(), inner.upgrade(), wakers.upgrade()) {
                        (Some(value), Some(inner), Some(wakers)) => {
                            // generate new Future
                            let owner = inner.read().or_poisoned().owner.clone();
                            let fut = owner.with_cleanup(|| {
                                any_subscriber
                                    .with_observer(|| ScopedFuture::new($fun()))
                            });
                            #[cfg(feature = "sandboxed-arenas")]
                            let fut = Sandboxed::new(fut);

                            // update state from Complete to Reloading
                            {
                                let mut value = value.write().or_poisoned();
                                // if it's initial Loading, it will just reset to Loading
                                if let AsyncState::Complete(old) =
                                    mem::take(&mut *value)
                                {
                                    *value = AsyncState::Reloading(old);
                                }
                            }

                            // notify reactive subscribers that we're now loading
                            for sub in (&inner.read().or_poisoned().subscribers).into_iter() {
                                sub.mark_check();
                            }

                            // generate and assign new value
                            let new_value = fut.await;
                            *value.write().or_poisoned() = AsyncState::Complete(new_value);

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
        Self::new_with_initial(AsyncState::Loading, fun)
    }

    #[track_caller]
    pub fn new_with_initial<Fut>(
        initial_value: AsyncState<T>,
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
        Self::new_unsync_with_initial(AsyncState::Loading, fun)
    }

    #[track_caller]
    pub fn new_unsync_with_initial<Fut>(
        initial_value: AsyncState<T>,
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

    pub fn ready(&self) -> AsyncDerivedReadyFuture<T> {
        AsyncDerivedReadyFuture {
            source: self.to_any_source(),
            value: Arc::clone(&self.value),
            wakers: Arc::clone(&self.wakers),
        }
    }
}

impl<T: 'static> ReadUntracked for ArcAsyncDerived<T> {
    type Value = ReadGuard<AsyncState<T>, Plain<AsyncState<T>>>;

    fn try_read_untracked(&self) -> Option<Self::Value> {
        Plain::try_new(Arc::clone(&self.value)).map(ReadGuard::new)
    }
}

impl<T: 'static> ToAnySource for ArcAsyncDerived<T> {
    fn to_any_source(&self) -> AnySource {
        AnySource(
            Arc::as_ptr(&self.inner) as usize,
            Arc::downgrade(&self.inner) as Weak<dyn Source + Send + Sync>,
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
