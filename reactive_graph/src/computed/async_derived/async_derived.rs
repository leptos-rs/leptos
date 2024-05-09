use super::{
    ArcAsyncDerived, AsyncDerivedFuture, AsyncDerivedReadyFuture, AsyncState,
};
use crate::{
    graph::{
        AnySource, AnySubscriber, ReactiveNode, Source, Subscriber,
        ToAnySource, ToAnySubscriber,
    },
    owner::StoredValue,
    signal::guards::{Mapped, Plain, ReadGuard},
    traits::{DefinedAt, Dispose, ReadUntracked},
    unwrap_signal,
};
use core::fmt::Debug;
use std::{
    future::{Future, IntoFuture},
    panic::Location,
};

pub struct AsyncDerived<T: Send + Sync + 'static> {
    #[cfg(debug_assertions)]
    defined_at: &'static Location<'static>,
    inner: StoredValue<ArcAsyncDerived<T>>,
}

impl<T: Send + Sync + 'static> Dispose for AsyncDerived<T> {
    fn dispose(self) {
        self.inner.dispose()
    }
}

impl<T: Send + Sync + 'static> From<ArcAsyncDerived<T>> for AsyncDerived<T> {
    fn from(value: ArcAsyncDerived<T>) -> Self {
        #[cfg(debug_assertions)]
        let defined_at = value.defined_at;
        Self {
            #[cfg(debug_assertions)]
            defined_at,
            inner: StoredValue::new(value),
        }
    }
}

impl<T: Send + Sync + 'static> AsyncDerived<T> {
    #[track_caller]
    pub fn new<Fut>(fun: impl Fn() -> Fut + Send + Sync + 'static) -> Self
    where
        T: Send + Sync + 'static,
        Fut: Future<Output = T> + Send + Sync + 'static,
    {
        Self {
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
            inner: StoredValue::new(ArcAsyncDerived::new(fun)),
        }
    }

    pub fn new_with_initial<Fut>(
        initial_value: AsyncState<T>,
        fun: impl Fn() -> Fut + Send + Sync + 'static,
    ) -> Self
    where
        T: Send + Sync + 'static,
        Fut: Future<Output = T> + Send + 'static,
    {
        Self {
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
            inner: StoredValue::new(ArcAsyncDerived::new_with_initial(
                initial_value,
                fun,
            )),
        }
    }

    pub fn new_unsync<Fut>(fun: impl Fn() -> Fut + 'static) -> Self
    where
        T: 'static,
        Fut: Future<Output = T> + 'static,
    {
        Self {
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
            inner: StoredValue::new(ArcAsyncDerived::new_unsync(fun)),
        }
    }

    pub fn new_unsync_with_initial<Fut>(
        initial_value: AsyncState<T>,
        fun: impl Fn() -> Fut + 'static,
    ) -> Self
    where
        T: 'static,
        Fut: Future<Output = T> + 'static,
    {
        Self {
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
            inner: StoredValue::new(ArcAsyncDerived::new_unsync_with_initial(
                initial_value,
                fun,
            )),
        }
    }

    #[track_caller]
    pub fn ready(&self) -> AsyncDerivedReadyFuture<T> {
        let this = self.inner.get().unwrap_or_else(unwrap_signal!(self));
        this.ready()
    }
}

impl<T: Send + Sync + 'static> Copy for AsyncDerived<T> {}

impl<T: Send + Sync + 'static> Clone for AsyncDerived<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Send + Sync + 'static> Debug for AsyncDerived<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AsyncDerived")
            .field("type", &std::any::type_name::<T>())
            .field("store", &self.inner)
            .finish()
    }
}

impl<T: Send + Sync + 'static> DefinedAt for AsyncDerived<T> {
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

impl<T: Send + Sync + Clone + 'static> IntoFuture for AsyncDerived<T>
where
    T: Clone,
{
    type Output = T;
    type IntoFuture = AsyncDerivedFuture<T>;

    #[track_caller]
    fn into_future(self) -> Self::IntoFuture {
        let this = self.inner.get().unwrap_or_else(unwrap_signal!(self));
        this.into_future()
    }
}

impl<T: Send + Sync + 'static> ReadUntracked for AsyncDerived<T> {
    type Value = ReadGuard<AsyncState<T>, Plain<AsyncState<T>>>;

    fn try_read_untracked(&self) -> Option<Self::Value> {
        self.inner.get().map(|inner| inner.read_untracked())
    }
}

impl<T: Send + Sync + 'static> ToAnySource for AsyncDerived<T> {
    fn to_any_source(&self) -> AnySource {
        self.inner
            .get()
            .map(|inner| inner.to_any_source())
            .unwrap_or_else(unwrap_signal!(self))
    }
}

impl<T: Send + Sync + 'static> ToAnySubscriber for AsyncDerived<T> {
    fn to_any_subscriber(&self) -> AnySubscriber {
        self.inner
            .get()
            .map(|inner| inner.to_any_subscriber())
            .unwrap_or_else(unwrap_signal!(self))
    }
}

impl<T: Send + Sync + 'static> Source for AsyncDerived<T> {
    fn add_subscriber(&self, subscriber: AnySubscriber) {
        if let Some(inner) = self.inner.get() {
            inner.add_subscriber(subscriber);
        }
    }

    fn remove_subscriber(&self, subscriber: &AnySubscriber) {
        if let Some(inner) = self.inner.get() {
            inner.remove_subscriber(subscriber);
        }
    }

    fn clear_subscribers(&self) {
        if let Some(inner) = self.inner.get() {
            inner.clear_subscribers();
        }
    }
}

impl<T: Send + Sync + 'static> ReactiveNode for AsyncDerived<T> {
    fn mark_dirty(&self) {
        if let Some(inner) = self.inner.get() {
            inner.mark_dirty();
        }
    }

    fn mark_check(&self) {
        if let Some(inner) = self.inner.get() {
            inner.mark_check();
        }
    }

    fn mark_subscribers_check(&self) {
        if let Some(inner) = self.inner.get() {
            inner.mark_subscribers_check();
        }
    }

    fn update_if_necessary(&self) -> bool {
        if let Some(inner) = self.inner.get() {
            inner.update_if_necessary()
        } else {
            false
        }
    }
}

impl<T: Send + Sync + 'static> Subscriber for AsyncDerived<T> {
    fn add_source(&self, source: AnySource) {
        if let Some(inner) = self.inner.get() {
            inner.add_source(source);
        }
    }

    fn clear_sources(&self, subscriber: &AnySubscriber) {
        if let Some(inner) = self.inner.get() {
            inner.clear_sources(subscriber);
        }
    }
}
