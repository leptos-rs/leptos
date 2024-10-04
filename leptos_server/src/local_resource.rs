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
    signal::guards::{AsyncPlain, ReadGuard},
    traits::{DefinedAt, IsDisposed, ReadUntracked},
};
use send_wrapper::SendWrapper;
use std::{
    future::{pending, Future, IntoFuture},
    panic::Location,
};

pub struct ArcLocalResource<T> {
    data: ArcAsyncDerived<T>,
    #[cfg(debug_assertions)]
    defined_at: &'static Location<'static>,
}

impl<T> Clone for ArcLocalResource<T> {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            #[cfg(debug_assertions)]
            defined_at: self.defined_at,
        }
    }
}

impl<T> ArcLocalResource<T> {
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
        Self {
            data: ArcAsyncDerived::new_unsync(fetcher),
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
        }
    }
}

impl<T> IntoFuture for ArcLocalResource<T>
where
    T: Clone + 'static,
{
    type Output = T;
    type IntoFuture = AsyncDerivedFuture<T>;

    fn into_future(self) -> Self::IntoFuture {
        if let Some(mut notifier) = use_context::<LocalResourceNotifier>() {
            notifier.notify();
        } else if cfg!(feature = "ssr") {
            panic!(
                "Reading from a LocalResource outside Suspense in `ssr` mode \
                 will cause the response to hang, because LocalResources are \
                 always pending on the server."
            );
        }
        self.data.into_future()
    }
}

impl<T> DefinedAt for ArcLocalResource<T> {
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

impl<T> ReadUntracked for ArcLocalResource<T>
where
    T: 'static,
{
    type Value = ReadGuard<Option<T>, AsyncPlain<Option<T>>>;

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

pub struct LocalResource<T> {
    data: AsyncDerived<SendWrapper<T>>,
    #[cfg(debug_assertions)]
    defined_at: &'static Location<'static>,
}

impl<T> Clone for LocalResource<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for LocalResource<T> {}

impl<T> LocalResource<T> {
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

        Self {
            data: if cfg!(feature = "ssr") {
                AsyncDerived::new_mock(fetcher)
            } else {
                let fetcher = SendWrapper::new(fetcher);
                AsyncDerived::new(move || {
                    let fut = fetcher();
                    SendWrapper::new(async move { SendWrapper::new(fut.await) })
                })
            },
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
        }
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
