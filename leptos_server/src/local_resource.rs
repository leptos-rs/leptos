use reactive_graph::{
    computed::{
        suspense::LocalResourceNotifier, ArcAsyncDerived, AsyncDerived,
        AsyncDerivedFuture,
    },
    graph::{
        AnySource, AnySubscriber, ReactiveNode, Source, Subscriber,
        ToAnySource, ToAnySubscriber,
    },
    owner::use_context,
    signal::{
        guards::{AsyncPlain, ReadGuard},
        ArcRwSignal, RwSignal,
    },
    traits::{
        DefinedAt, IsDisposed, ReadUntracked, Track, Update, With, Write,
    },
};
use send_wrapper::SendWrapper;
use std::{
    future::{pending, Future, IntoFuture},
    panic::Location,
};

/// A reference-counted resource that only loads its data locally on the client.
pub struct ArcLocalResource<T> {
    data: ArcAsyncDerived<SendWrapper<T>>,
    refetch: ArcRwSignal<usize>,
    #[cfg(any(debug_assertions, leptos_debuginfo))]
    defined_at: &'static Location<'static>,
}

impl<T> Clone for ArcLocalResource<T> {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            refetch: self.refetch.clone(),
            #[cfg(any(debug_assertions, leptos_debuginfo))]
            defined_at: self.defined_at,
        }
    }
}

impl<T> ArcLocalResource<T> {
    /// Creates the resource.
    ///
    /// This will only begin loading data if you are on the client (i.e., if you do not have the
    /// `ssr` feature activated).
    #[track_caller]
    pub fn new<Fut>(fetcher: impl Fn() -> Fut + 'static) -> Self
    where
        T: 'static,
        Fut: Future<Output = T> + 'static,
    {
        let fetcher = move || {
            let fut = fetcher();
            async move {
                // in SSR mode, this will simply always be pending
                // if we try to read from it, we will trigger Suspense automatically to fall back
                // so this will never need to return anything
                if cfg!(feature = "ssr") {
                    pending().await
                } else {
                    // LocalResources that are immediately available can cause a hydration error,
                    // because the future *looks* like it is alredy ready (and therefore would
                    // already have been rendered to html on the server), but in fact was ignored
                    // on the server. the simplest way to avoid this is to ensure that we always
                    // wait a tick before resolving any value for a localresource.
                    any_spawner::Executor::tick().await;
                    fut.await
                }
            }
        };
        let fetcher = SendWrapper::new(fetcher);
        let refetch = ArcRwSignal::new(0);
        let data = {
            let refetch = refetch.clone();
            ArcAsyncDerived::new(move || {
                refetch.track();
                let fut = fetcher();
                SendWrapper::new(async move { SendWrapper::new(fut.await) })
            })
        };
        Self {
            data,
            refetch,
            #[cfg(any(debug_assertions, leptos_debuginfo))]
            defined_at: Location::caller(),
        }
    }

    /// Re-runs the async function.
    pub fn refetch(&self) {
        *self.refetch.write() += 1;
    }

    /// Synchronously, reactively reads the current value of the resource and applies the function
    /// `f` to its value if it is `Some(_)`.
    #[track_caller]
    pub fn map<U>(&self, f: impl FnOnce(&SendWrapper<T>) -> U) -> Option<U>
    where
        T: 'static,
    {
        self.data.try_with(|n| n.as_ref().map(f))?
    }
}

impl<T, E> ArcLocalResource<Result<T, E>>
where
    T: 'static,
    E: Clone + 'static,
{
    /// Applies the given function when a resource that returns `Result<T, E>`
    /// has resolved and loaded an `Ok(_)`, rather than requiring nested `.map()`
    /// calls over the `Option<Result<_, _>>` returned by the resource.
    ///
    /// This is useful when used with features like server functions, in conjunction
    /// with `<ErrorBoundary/>` and `<Suspense/>`, when these other components are
    /// left to handle the `None` and `Err(_)` states.
    #[track_caller]
    pub fn and_then<U>(&self, f: impl FnOnce(&T) -> U) -> Option<Result<U, E>> {
        self.map(|data| data.as_ref().map(f).map_err(|e| e.clone()))
    }
}

impl<T> IntoFuture for ArcLocalResource<T>
where
    T: Clone + 'static,
{
    type Output = T;
    type IntoFuture = futures::future::Map<
        AsyncDerivedFuture<SendWrapper<T>>,
        fn(SendWrapper<T>) -> T,
    >;

    fn into_future(self) -> Self::IntoFuture {
        use futures::FutureExt;

        if let Some(mut notifier) = use_context::<LocalResourceNotifier>() {
            notifier.notify();
        } else if cfg!(feature = "ssr") {
            panic!(
                "Reading from a LocalResource outside Suspense in `ssr` mode \
                 will cause the response to hang, because LocalResources are \
                 always pending on the server."
            );
        }
        self.data.into_future().map(|value| (*value).clone())
    }
}

impl<T> DefinedAt for ArcLocalResource<T> {
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

impl<T> ReadUntracked for ArcLocalResource<T>
where
    T: 'static,
{
    type Value =
        ReadGuard<Option<SendWrapper<T>>, AsyncPlain<Option<SendWrapper<T>>>>;

    fn try_read_untracked(&self) -> Option<Self::Value> {
        if let Some(mut notifier) = use_context::<LocalResourceNotifier>() {
            notifier.notify();
        } else if cfg!(feature = "ssr") {
            panic!(
                "Reading from a LocalResource outside Suspense in `ssr` mode \
                 will cause the response to hang, because LocalResources are \
                 always pending on the server."
            );
        }
        self.data.try_read_untracked()
    }
}

impl<T: 'static> IsDisposed for ArcLocalResource<T> {
    #[inline(always)]
    fn is_disposed(&self) -> bool {
        false
    }
}

impl<T: 'static> ToAnySource for ArcLocalResource<T> {
    fn to_any_source(&self) -> AnySource {
        self.data.to_any_source()
    }
}

impl<T: 'static> ToAnySubscriber for ArcLocalResource<T> {
    fn to_any_subscriber(&self) -> AnySubscriber {
        self.data.to_any_subscriber()
    }
}

impl<T> Source for ArcLocalResource<T> {
    fn add_subscriber(&self, subscriber: AnySubscriber) {
        self.data.add_subscriber(subscriber)
    }

    fn remove_subscriber(&self, subscriber: &AnySubscriber) {
        self.data.remove_subscriber(subscriber);
    }

    fn clear_subscribers(&self) {
        self.data.clear_subscribers();
    }
}

impl<T> ReactiveNode for ArcLocalResource<T> {
    fn mark_dirty(&self) {
        self.data.mark_dirty();
    }

    fn mark_check(&self) {
        self.data.mark_check();
    }

    fn mark_subscribers_check(&self) {
        self.data.mark_subscribers_check();
    }

    fn update_if_necessary(&self) -> bool {
        self.data.update_if_necessary()
    }
}

impl<T> Subscriber for ArcLocalResource<T> {
    fn add_source(&self, source: AnySource) {
        self.data.add_source(source);
    }

    fn clear_sources(&self, subscriber: &AnySubscriber) {
        self.data.clear_sources(subscriber);
    }
}

/// A resource that only loads its data locally on the client.
pub struct LocalResource<T> {
    data: AsyncDerived<SendWrapper<T>>,
    refetch: RwSignal<usize>,
    #[cfg(any(debug_assertions, leptos_debuginfo))]
    defined_at: &'static Location<'static>,
}

impl<T> Clone for LocalResource<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for LocalResource<T> {}

impl<T> LocalResource<T> {
    /// Creates the resource.
    ///
    /// This will only begin loading data if you are on the client (i.e., if you do not have the
    /// `ssr` feature activated).
    #[track_caller]
    pub fn new<Fut>(fetcher: impl Fn() -> Fut + 'static) -> Self
    where
        T: 'static,
        Fut: Future<Output = T> + 'static,
    {
        let fetcher = move || {
            let fut = fetcher();
            async move {
                // in SSR mode, this will simply always be pending
                // if we try to read from it, we will trigger Suspense automatically to fall back
                // so this will never need to return anything
                if cfg!(feature = "ssr") {
                    pending().await
                } else {
                    // LocalResources that are immediately available can cause a hydration error,
                    // because the future *looks* like it is alredy ready (and therefore would
                    // already have been rendered to html on the server), but in fact was ignored
                    // on the server. the simplest way to avoid this is to ensure that we always
                    // wait a tick before resolving any value for a localresource.
                    any_spawner::Executor::tick().await;
                    fut.await
                }
            }
        };
        let refetch = RwSignal::new(0);

        Self {
            data: if cfg!(feature = "ssr") {
                AsyncDerived::new_mock(fetcher)
            } else {
                let fetcher = SendWrapper::new(fetcher);
                AsyncDerived::new(move || {
                    refetch.track();
                    let fut = fetcher();
                    SendWrapper::new(async move { SendWrapper::new(fut.await) })
                })
            },
            refetch,
            #[cfg(any(debug_assertions, leptos_debuginfo))]
            defined_at: Location::caller(),
        }
    }

    /// Re-runs the async function.
    pub fn refetch(&self) {
        self.refetch.try_update(|n| *n += 1);
    }
}

impl<T> IntoFuture for LocalResource<T>
where
    T: Clone + 'static,
{
    type Output = T;
    type IntoFuture = futures::future::Map<
        AsyncDerivedFuture<SendWrapper<T>>,
        fn(SendWrapper<T>) -> T,
    >;

    fn into_future(self) -> Self::IntoFuture {
        use futures::FutureExt;

        if let Some(mut notifier) = use_context::<LocalResourceNotifier>() {
            notifier.notify();
        } else if cfg!(feature = "ssr") {
            panic!(
                "Reading from a LocalResource outside Suspense in `ssr` mode \
                 will cause the response to hang, because LocalResources are \
                 always pending on the server."
            );
        }
        self.data.into_future().map(|value| (*value).clone())
    }
}

impl<T> DefinedAt for LocalResource<T> {
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

impl<T> ReadUntracked for LocalResource<T>
where
    T: 'static,
{
    type Value =
        ReadGuard<Option<SendWrapper<T>>, AsyncPlain<Option<SendWrapper<T>>>>;

    fn try_read_untracked(&self) -> Option<Self::Value> {
        if let Some(mut notifier) = use_context::<LocalResourceNotifier>() {
            notifier.notify();
        } else if cfg!(feature = "ssr") {
            panic!(
                "Reading from a LocalResource outside Suspense in `ssr` mode \
                 will cause the response to hang, because LocalResources are \
                 always pending on the server."
            );
        }
        self.data.try_read_untracked()
    }
}

impl<T: 'static> IsDisposed for LocalResource<T> {
    fn is_disposed(&self) -> bool {
        self.data.is_disposed()
    }
}

impl<T: 'static> ToAnySource for LocalResource<T>
where
    T: 'static,
{
    fn to_any_source(&self) -> AnySource {
        self.data.to_any_source()
    }
}

impl<T: 'static> ToAnySubscriber for LocalResource<T>
where
    T: 'static,
{
    fn to_any_subscriber(&self) -> AnySubscriber {
        self.data.to_any_subscriber()
    }
}

impl<T> Source for LocalResource<T>
where
    T: 'static,
{
    fn add_subscriber(&self, subscriber: AnySubscriber) {
        self.data.add_subscriber(subscriber)
    }

    fn remove_subscriber(&self, subscriber: &AnySubscriber) {
        self.data.remove_subscriber(subscriber);
    }

    fn clear_subscribers(&self) {
        self.data.clear_subscribers();
    }
}

impl<T> ReactiveNode for LocalResource<T>
where
    T: 'static,
{
    fn mark_dirty(&self) {
        self.data.mark_dirty();
    }

    fn mark_check(&self) {
        self.data.mark_check();
    }

    fn mark_subscribers_check(&self) {
        self.data.mark_subscribers_check();
    }

    fn update_if_necessary(&self) -> bool {
        self.data.update_if_necessary()
    }
}

impl<T> Subscriber for LocalResource<T>
where
    T: 'static,
{
    fn add_source(&self, source: AnySource) {
        self.data.add_source(source);
    }

    fn clear_sources(&self, subscriber: &AnySubscriber) {
        self.data.clear_sources(subscriber);
    }
}

impl<T: 'static> From<ArcLocalResource<T>> for LocalResource<T> {
    fn from(arc: ArcLocalResource<T>) -> Self {
        Self {
            data: arc.data.into(),
            refetch: arc.refetch.into(),
            #[cfg(any(debug_assertions, leptos_debuginfo))]
            defined_at: arc.defined_at,
        }
    }
}

impl<T: 'static> From<LocalResource<T>> for ArcLocalResource<T> {
    fn from(local: LocalResource<T>) -> Self {
        Self {
            data: local.data.into(),
            refetch: local.refetch.into(),
            #[cfg(any(debug_assertions, leptos_debuginfo))]
            defined_at: local.defined_at,
        }
    }
}
