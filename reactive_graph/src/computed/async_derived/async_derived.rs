use super::{ArcAsyncDerived, AsyncDerivedReadyFuture};
use crate::{
    graph::{
        AnySource, AnySubscriber, ReactiveNode, Source, Subscriber,
        ToAnySource, ToAnySubscriber,
    },
    owner::StoredValue,
    signal::guards::{AsyncPlain, ReadGuard},
    traits::{DefinedAt, Dispose, ReadUntracked},
    unwrap_signal,
};
use core::fmt::Debug;
use std::{future::Future, panic::Location};

/// A reactive value that is derived by running an asynchronous computation in response to changes
/// in its sources.
///
/// When one of its dependencies changes, this will re-run its async computation, then notify other
/// values that depend on it that it has changed.
///
/// This is an arena-allocated type, which is `Copy` and is disposed when its reactive
/// [`Owner`](crate::owner::Owner) cleans up. For a reference-counted signal that livesas
/// as long as a reference to it is alive, see [`ArcAsyncDerived`].
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
/// let derived = AsyncDerived::new(move || async move {
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
/// assert_eq!(derived.await, 0);
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
pub struct AsyncDerived<T> {
    #[cfg(debug_assertions)]
    defined_at: &'static Location<'static>,
    pub(crate) inner: StoredValue<ArcAsyncDerived<T>>,
}

impl<T: 'static> Dispose for AsyncDerived<T> {
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
        Self {
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
            inner: StoredValue::new(ArcAsyncDerived::new(fun)),
        }
    }

    /// Creates a new async derived computation with an initial value.
    ///
    /// If the initial value is `Some(_)`, the task will not be run initially.
    pub fn new_with_initial<Fut>(
        initial_value: Option<T>,
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

    /// Creates a new async derived computation that will be guaranteed to run on the current
    /// thread.
    ///
    /// This runs eagerly: i.e., calls `fun` once when created and immediately spawns the `Future`
    /// as a new task.
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

    /// Creates a new async derived computation with an initial value. Async work will be
    /// guaranteed to run only on the current thread.
    ///
    /// If the initial value is `Some(_)`, the task will not be run initially.
    pub fn new_unsync_with_initial<Fut>(
        initial_value: Option<T>,
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

    /// Returns a `Future` that is ready when this resource has next finished loading.
    #[track_caller]
    pub fn ready(&self) -> AsyncDerivedReadyFuture {
        let this = self.inner.get().unwrap_or_else(unwrap_signal!(self));
        this.ready()
    }
}

impl<T> Copy for AsyncDerived<T> {}

impl<T> Clone for AsyncDerived<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Debug for AsyncDerived<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AsyncDerived")
            .field("type", &std::any::type_name::<T>())
            .field("store", &self.inner)
            .finish()
    }
}

impl<T> DefinedAt for AsyncDerived<T> {
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

impl<T: Send + Sync + 'static> ReadUntracked for AsyncDerived<T> {
    type Value = ReadGuard<Option<T>, AsyncPlain<Option<T>>>;

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
