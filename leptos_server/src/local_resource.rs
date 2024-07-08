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
    traits::{DefinedAt, ReadUntracked},
};
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
    pub fn new<Fut>(fetcher: impl Fn() -> Fut + Send + Sync + 'static) -> Self
    where
        T: Send + Sync + 'static,
        Fut: Future<Output = T> + Send + 'static,
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
                    fut.await
                }
            }
        };
        Self {
            data: ArcAsyncDerived::new(fetcher),
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
    T: Send + Sync + 'static,
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
    data: AsyncDerived<T>,
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
    pub fn new<Fut>(fetcher: impl Fn() -> Fut + Send + Sync + 'static) -> Self
    where
        T: Send + Sync + 'static,
        Fut: Future<Output = T> + Send + 'static,
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
                    fut.await
                }
            }
        };
        Self {
            data: AsyncDerived::new(fetcher),
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
    T: Send + Sync + 'static,
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

impl<T: 'static> ToAnySource for LocalResource<T>
where
    T: Send + Sync + 'static,
{
    fn to_any_source(&self) -> AnySource {
        self.data.to_any_source()
    }
}

impl<T: 'static> ToAnySubscriber for LocalResource<T>
where
    T: Send + Sync + 'static,
{
    fn to_any_subscriber(&self) -> AnySubscriber {
        self.data.to_any_subscriber()
    }
}

impl<T> Source for LocalResource<T>
where
    T: Send + Sync + 'static,
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
    T: Send + Sync + 'static,
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
    T: Send + Sync + 'static,
{
    fn add_source(&self, source: AnySource) {
        self.data.add_source(source);
    }

    fn clear_sources(&self, subscriber: &AnySubscriber) {
        self.data.clear_sources(subscriber);
    }
}
