use super::{inner::MemoInner, ArcMemo};
use crate::{
    owner::StoredValue,
    signal::{
        guards::{Mapped, Plain, ReadGuard},
        ArcReadSignal,
    },
    traits::{DefinedAt, Dispose, Get, ReadUntracked, Track},
    unwrap_signal,
};
use std::{fmt::Debug, hash::Hash, panic::Location};

/// A memo is an efficient derived reactive value based on other reactive values.
///
/// Unlike a "derived signal," a memo comes with two guarantees:
/// 1. The memo will only run *once* per change, no matter how many times you
///    access its value.
/// 2. The memo will only notify its dependents if the value of the computation changes.
///
/// This makes a memo the perfect tool for expensive computations.
///
/// Memos have a certain overhead compared to derived signals. In most cases, you should
/// create a derived signal. But if the derivation calculation is expensive, you should
/// create a memo.
///
/// Memos are lazy: they do not run at all until they are read for the first time, and they will
/// not re-run the calculation when a source signal changes until they are read again.
///
/// This is an arena-allocated type, which is `Copy` and is disposed when its reactive
/// [`Owner`](crate::owner::Owner) cleans up. For a reference-counted signal that livesas
/// as long as a reference to it is alive, see [`ArcMemo`].
///
/// ```
/// # use reactive_graph::prelude::*;
/// # use reactive_graph::computed::Memo;
/// # use reactive_graph::effect::Effect;
/// # use reactive_graph::signal::signal;
/// # tokio_test::block_on(async move {
/// # any_spawner::Executor::init_tokio();
/// # tokio::task::LocalSet::new().run_until(async {
/// # fn really_expensive_computation(value: i32) -> i32 { value };
/// let (value, set_value) = signal(0);
///
/// // 🆗 we could create a derived signal with a simple function
/// let double_value = move || value.get() * 2;
/// set_value.set(2);
/// assert_eq!(double_value(), 4);
///
/// // but imagine the computation is really expensive
/// let expensive = move || really_expensive_computation(value.get()); // lazy: doesn't run until called
/// Effect::new(move |_| {
///   // 🆗 run #1: calls `really_expensive_computation` the first time
///   println!("expensive = {}", expensive());
/// });
/// Effect::new(move |_| {
///   // ❌ run #2: this calls `really_expensive_computation` a second time!
///   let value = expensive();
///   // do something else...
/// });
///
/// // instead, we create a memo
/// // 🆗 run #1: the calculation runs once immediately
/// let memoized = Memo::new(move |_| really_expensive_computation(value.get()));
/// Effect::new(move |_| {
///   // 🆗 reads the current value of the memo
///   //    can be `memoized()` on nightly
///   println!("memoized = {}", memoized.get());
/// });
/// Effect::new(move |_| {
///   // ✅ reads the current value **without re-running the calculation**
///   let value = memoized.get();
///   // do something else...
/// });
/// # });
/// # });
/// ```
///
/// ## Core Trait Implementations
/// - [`.get()`](crate::traits::Get) clones the current value of the memo.
///   If you call it within an effect, it will cause that effect to subscribe
///   to the memo, and to re-run whenever the value of the memo changes.
///   - [`.get_untracked()`](crate::traits::GetUntracked) clones the value of
///     the memo without reactively tracking it.
/// - [`.read()`](crate::traits::Read) returns a guard that allows accessing the
///   value of the memo by reference. If you call it within an effect, it will
///   cause that effect to subscribe to the memo, and to re-run whenever the
///   value of the memo changes.
///   - [`.read_untracked()`](crate::traits::ReadUntracked) gives access to the
///     current value of the memo without reactively tracking it.
/// - [`.with()`](crate::traits::With) allows you to reactively access the memo’s
///   value without cloning by applying a callback function.
///   - [`.with_untracked()`](crate::traits::WithUntracked) allows you to access
///     the memo’s value by applying a callback function without reactively
///     tracking it.
/// - [`.to_stream()`](crate::traits::ToStream) converts the memo to an `async`
///   stream of values.
/// - [`::from_stream()`](crate::traits::FromStream) converts an `async` stream
///   of values into a memo containing the latest value.
pub struct Memo<T> {
    #[cfg(debug_assertions)]
    defined_at: &'static Location<'static>,
    inner: StoredValue<ArcMemo<T>>,
}

impl<T: 'static> Dispose for Memo<T> {
    fn dispose(self) {
        self.inner.dispose()
    }
}

impl<T: Send + Sync + 'static> From<ArcMemo<T>> for Memo<T> {
    #[track_caller]
    fn from(value: ArcMemo<T>) -> Self {
        Self {
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
            inner: StoredValue::new(value),
        }
    }
}

impl<T: Send + Sync + 'static> Memo<T> {
    #[track_caller]
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "debug", skip_all,)
    )]
    /// Creates a new memoized, computed reactive value.
    ///
    /// As with an [`Effect`](crate::effect::Effect), the argument to the memo function is the previous value,
    /// i.e., the current value of the memo, which will be `None` for the initial calculation.
    /// ```
    /// # use reactive_graph::prelude::*;
    /// # use reactive_graph::computed::Memo;
    /// # use reactive_graph::effect::Effect;
    /// # use reactive_graph::signal::signal;
    /// # tokio_test::block_on(async move {
    /// # any_spawner::Executor::init_tokio();
    /// # fn really_expensive_computation(value: i32) -> i32 { value };
    /// let (value, set_value) = signal(0);
    ///
    /// // the memo will reactively update whenever `value` changes
    /// let memoized =
    ///     Memo::new(move |_| really_expensive_computation(value.get()));
    /// # });
    /// ```
    pub fn new(fun: impl Fn(Option<&T>) -> T + Send + Sync + 'static) -> Self
    where
        T: PartialEq,
    {
        Self {
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
            inner: StoredValue::new(ArcMemo::new(fun)),
        }
    }

    #[track_caller]
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "trace", skip_all,)
    )]
    /// Creates a new memo with a custom comparison function. By default, memos simply use
    /// [`PartialEq`] to compare the previous value to the new value. Passing a custom comparator
    /// allows you to compare the old and new values using any criteria.
    ///
    /// `changed` should be a function that returns `true` if the new value is different from the
    /// old value.
    pub fn new_with_compare(
        fun: impl Fn(Option<&T>) -> T + Send + Sync + 'static,
        changed: fn(Option<&T>, Option<&T>) -> bool,
    ) -> Self
    where
        T: PartialEq,
    {
        Self {
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
            inner: StoredValue::new(ArcMemo::new_with_compare(fun, changed)),
        }
    }

    /// Creates a new memo by passing a function that computes the value.
    ///
    /// Unlike [`ArcMemo::new`](), this receives ownership of the previous value. As a result, it
    /// must return both the new value and a `bool` that is `true` if the value has changed.
    ///
    /// This is lazy: the function will not be called until the memo's value is read for the first
    /// time.
    #[track_caller]
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "trace", skip_all,)
    )]
    pub fn new_owning(
        fun: impl Fn(Option<T>) -> (T, bool) + Send + Sync + 'static,
    ) -> Self
    where
        T: PartialEq,
    {
        Self {
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
            inner: StoredValue::new(ArcMemo::new_owning(fun)),
        }
    }
}

impl<T> Copy for Memo<T> {}

impl<T> Clone for Memo<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Debug for Memo<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Memo")
            .field("type", &std::any::type_name::<T>())
            .field("store", &self.inner)
            .finish()
    }
}

impl<T> PartialEq for Memo<T> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<T> Eq for Memo<T> {}

impl<T> Hash for Memo<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
    }
}

impl<T> DefinedAt for Memo<T> {
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

impl<T: Send + Sync + 'static> Track for Memo<T> {
    #[track_caller]
    fn track(&self) {
        if let Some(inner) = self.inner.get() {
            inner.track();
        }
    }
}

impl<T: Send + Sync + 'static> ReadUntracked for Memo<T> {
    type Value = ReadGuard<T, Mapped<Plain<MemoInner<T>>, T>>;

    fn try_read_untracked(&self) -> Option<Self::Value> {
        self.inner.get().map(|inner| inner.read_untracked())
    }
}

impl<T: Send + Sync + 'static> From<Memo<T>> for ArcMemo<T> {
    #[track_caller]
    fn from(value: Memo<T>) -> Self {
        value.inner.get().unwrap_or_else(unwrap_signal!(value))
    }
}

impl<T> From<ArcReadSignal<T>> for Memo<T>
where
    T: Clone + PartialEq + Send + Sync + 'static,
{
    #[track_caller]
    fn from(value: ArcReadSignal<T>) -> Self {
        Memo::new(move |_| value.get())
    }
}
