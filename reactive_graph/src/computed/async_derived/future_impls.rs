use super::{ArcAsyncDerived, AsyncDerived, AsyncState};
use crate::{
    graph::{AnySource, ToAnySource},
    signal::guards::Plain,
    traits::{DefinedAt, Track},
    unwrap_signal,
};
use or_poisoned::OrPoisoned;
use pin_project_lite::pin_project;
use std::{
    future::{Future, IntoFuture},
    pin::Pin,
    sync::{Arc, RwLock},
    task::{Context, Poll, Waker},
};

/// A [`Future`] that is ready when an [`ArcAsyncDerived`] is finished loading or reloading,
/// but does not contain its value.
pub struct ArcAsyncDerivedReadyFuture<T> {
    pub(crate) source: AnySource,
    pub(crate) value: Arc<RwLock<AsyncState<T>>>,
    pub(crate) wakers: Arc<RwLock<Vec<Waker>>>,
}

impl<T: 'static> Future for ArcAsyncDerivedReadyFuture<T> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let waker = cx.waker();
        self.source.track();
        match &*self.value.read().or_poisoned() {
            AsyncState::Loading | AsyncState::Reloading(_) => {
                self.wakers.write().or_poisoned().push(waker.clone());
                Poll::Pending
            }
            AsyncState::Complete(_) => Poll::Ready(()),
        }
    }
}

/// A [`Future`] that is ready when an [`ArcAsyncDerived`] is finished loading or reloading,
/// and contains its value.
pub struct ArcAsyncDerivedFuture<T> {
    source: AnySource,
    value: Arc<RwLock<AsyncState<T>>>,
    wakers: Arc<RwLock<Vec<Waker>>>,
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
            wakers: Arc::clone(&self.wakers),
        }
    }
}

// this is implemented to output T by cloning it because read guards should not be held across
// .await points, and it's way too easy to trip up by doing that!
impl<T> Future for ArcAsyncDerivedFuture<T>
where
    T: Clone + 'static,
{
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let waker = cx.waker();
        self.source.track();
        let value =
            Plain::try_new(Arc::clone(&self.value)).expect("lock poisoned");
        match &*value {
            AsyncState::Loading | AsyncState::Reloading(_) => {
                self.wakers.write().or_poisoned().push(waker.clone());
                Poll::Pending
            }
            AsyncState::Complete(value) => Poll::Ready(value.clone()),
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

// this is implemented to output T by cloning it because read guards should not be held across
// .await points, and it's way too easy to trip up by doing that!
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
