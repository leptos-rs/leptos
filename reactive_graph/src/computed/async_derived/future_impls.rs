use super::{ArcAsyncDerived, AsyncDerived};
use crate::{
    graph::{AnySource, ToAnySource},
    signal::guards::{AsyncPlain, Mapped, ReadGuard},
    traits::{DefinedAt, Track},
    unwrap_signal,
};
use futures::pin_mut;
use or_poisoned::OrPoisoned;
use pin_project_lite::pin_project;
use std::{
    future::{Future, IntoFuture},
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, RwLock,
    },
    task::{Context, Poll, Waker},
};

/// A read guard that holds access to an async derived resource.
///
/// Implements [`Deref`](std::ops::Deref) to access the inner value. This should not be held longer
/// than it is needed, as it prevents updates to the inner value.
pub type AsyncDerivedGuard<T> = ReadGuard<T, Mapped<AsyncPlain<Option<T>>, T>>;

/// A [`Future`] that is ready when an [`ArcAsyncDerived`] is finished loading or reloading,
/// but does not contain its value.
pub struct ArcAsyncDerivedReadyFuture {
    pub(crate) source: AnySource,
    pub(crate) loading: Arc<AtomicBool>,
    pub(crate) wakers: Arc<RwLock<Vec<Waker>>>,
}

impl Future for ArcAsyncDerivedReadyFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let waker = cx.waker();
        self.source.track();
        if self.loading.load(Ordering::Relaxed) {
            self.wakers.write().or_poisoned().push(waker.clone());
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }
}

impl<T> IntoFuture for ArcAsyncDerived<T>
where
    T: Clone + 'static,
{
    type Output = T;
    type IntoFuture = ArcAsyncDerivedFuture<T>;

    fn into_future(self) -> Self::IntoFuture {
        ArcAsyncDerivedFuture {
            source: self.to_any_source(),
            value: Arc::clone(&self.value),
            loading: Arc::clone(&self.loading),
            wakers: Arc::clone(&self.wakers),
        }
    }
}

/// A [`Future`] that is ready when an [`ArcAsyncDerived`] is finished loading or reloading,
/// and contains its value. `.await`ing this clones the value `T`.
pub struct ArcAsyncDerivedFuture<T> {
    source: AnySource,
    value: Arc<async_lock::RwLock<Option<T>>>,
    loading: Arc<AtomicBool>,
    wakers: Arc<RwLock<Vec<Waker>>>,
}

impl<T> Future for ArcAsyncDerivedFuture<T>
where
    T: Clone + 'static,
{
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let waker = cx.waker();
        self.source.track();
        let value = self.value.read_arc();
        pin_mut!(value);
        match (self.loading.load(Ordering::Relaxed), value.poll(cx)) {
            (true, _) => {
                self.wakers.write().or_poisoned().push(waker.clone());
                Poll::Pending
            }
            (_, Poll::Pending) => Poll::Pending,
            (_, Poll::Ready(guard)) => {
                Poll::Ready(guard.as_ref().unwrap().clone())
            }
        }
    }
}

impl<T: 'static> ArcAsyncDerived<T> {
    pub fn by_ref(&self) -> ArcAsyncDerivedRefFuture<T> {
        ArcAsyncDerivedRefFuture {
            source: self.to_any_source(),
            value: Arc::clone(&self.value),
            loading: Arc::clone(&self.loading),
            wakers: Arc::clone(&self.wakers),
        }
    }
}

/// A [`Future`] that is ready when an [`ArcAsyncDerived`] is finished loading or reloading,
/// and yields an [`AsyncDerivedGuard`] that dereferences to its value.
pub struct ArcAsyncDerivedRefFuture<T> {
    source: AnySource,
    value: Arc<async_lock::RwLock<Option<T>>>,
    loading: Arc<AtomicBool>,
    wakers: Arc<RwLock<Vec<Waker>>>,
}

impl<T> Future for ArcAsyncDerivedRefFuture<T>
where
    T: 'static,
{
    type Output = AsyncDerivedGuard<T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let waker = cx.waker();
        self.source.track();
        let value = self.value.read_arc();
        pin_mut!(value);
        match (self.loading.load(Ordering::Relaxed), value.poll(cx)) {
            (true, _) => {
                self.wakers.write().or_poisoned().push(waker.clone());
                Poll::Pending
            }
            (_, Poll::Pending) => Poll::Pending,
            (_, Poll::Ready(guard)) => Poll::Ready(ReadGuard::new(
                Mapped::new_with_guard(AsyncPlain { guard }, |guard| {
                    guard.as_ref().unwrap()
                }),
            )),
        }
    }
}

pin_project! {
    /// A [`Future`] that is ready when an [`AsyncDerived`] is finished loading or reloading,
    /// and contains its value.
    pub struct AsyncDerivedFuture<T> {
        this: AsyncDerived<T>,
        #[pin]
        inner: Option<ArcAsyncDerivedFuture<T>>,
    }
}

impl<T> IntoFuture for AsyncDerived<T>
where
    T: Send + Sync + Clone + 'static,
{
    type Output = T;
    type IntoFuture = AsyncDerivedFuture<T>;

    fn into_future(self) -> Self::IntoFuture {
        AsyncDerivedFuture {
            this: self,
            inner: None,
        }
    }
}

impl<T> Future for AsyncDerivedFuture<T>
where
    T: Send + Sync + Clone + 'static,
{
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();
        if this.inner.is_none() {
            let stored = *this.this;
            this.inner.set(Some(
                stored
                    .inner
                    .get()
                    .unwrap_or_else(unwrap_signal!(stored))
                    .into_future(),
            ));
        }
        this.inner.as_pin_mut().unwrap().poll(cx)
    }
}

impl<T: 'static> AsyncDerived<T> {
    pub fn by_ref(&self) -> AsyncDerivedRefFuture<T> {
        AsyncDerivedRefFuture {
            this: *self,
            inner: None,
        }
    }
}

pin_project! {
    /// A [`Future`] that is ready when an [`AsyncDerived`] is finished loading or reloading,
    /// and contains its value.
    pub struct AsyncDerivedRefFuture<T> {
        this: AsyncDerived<T>,
        #[pin]
        inner: Option<ArcAsyncDerivedRefFuture<T>>,
    }
}

impl<T> Future for AsyncDerivedRefFuture<T>
where
    T: Send + Sync + 'static,
{
    type Output = AsyncDerivedGuard<T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();
        if this.inner.is_none() {
            let stored = *this.this;
            this.inner.set(Some(
                stored
                    .inner
                    .get()
                    .unwrap_or_else(unwrap_signal!(stored))
                    .by_ref(),
            ));
        }
        this.inner.as_pin_mut().unwrap().poll(cx)
    }
}
