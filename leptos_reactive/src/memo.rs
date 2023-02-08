#![forbid(unsafe_code)]
use crate::{
    GettableSignal, ReadSignal, RefSignal, Scope, UntrackedGettableSignal, UntrackedRefSignal,
};
use std::fmt::Debug;

/// Creates an efficient derived reactive value based on other reactive values.
///
/// Unlike a "derived signal," a memo comes with two guarantees:
/// 1. The memo will only run *once* per change, no matter how many times you
/// access its value.
/// 2. The memo will only notify its dependents if the value of the computation changes.
///
/// This makes a memo the perfect tool for expensive computations.
///
/// Memos have a certain overhead compared to derived signals. In most cases, you should
/// create a derived signal. But if the derivation calculation is expensive, you should
/// create a memo.
///
/// As with [create_effect](crate::create_effect), the argument to the memo function is the previous value,
/// i.e., the current value of the memo, which will be `None` for the initial calculation.
///
/// ```
/// # use leptos_reactive::*;
/// # fn really_expensive_computation(value: i32) -> i32 { value };
/// # create_scope(create_runtime(), |cx| {
/// let (value, set_value) = create_signal(cx, 0);
///
/// // 🆗 we could create a derived signal with a simple function
/// let double_value = move || value() * 2;
/// set_value(2);
/// assert_eq!(double_value(), 4);
///
/// // but imagine the computation is really expensive
/// let expensive = move || really_expensive_computation(value()); // lazy: doesn't run until called
/// create_effect(cx, move |_| {
///   // 🆗 run #1: calls `really_expensive_computation` the first time
///   log::debug!("expensive = {}", expensive());
/// });
/// create_effect(cx, move |_| {
///   // ❌ run #2: this calls `really_expensive_computation` a second time!
///   let value = expensive();
///   // do something else...
/// });
///
/// // instead, we create a memo
/// // 🆗 run #1: the calculation runs once immediately
/// let memoized = create_memo(cx, move |_| really_expensive_computation(value()));
/// create_effect(cx, move |_| {
///  // 🆗 reads the current value of the memo
///   log::debug!("memoized = {}", memoized());
/// });
/// create_effect(cx, move |_| {
///   // ✅ reads the current value **without re-running the calculation**
///   let value = memoized();
///   // do something else...
/// });
/// # }).dispose();
/// ```
#[cfg_attr(
    debug_assertions,
    instrument(
        level = "trace",
        skip_all,
        fields(
            cx = ?cx.id,
        )
    )
)]
pub fn create_memo<T>(cx: Scope, f: impl Fn(Option<&T>) -> T + 'static) -> Memo<T>
where
    T: PartialEq + 'static,
{
    cx.runtime.create_memo(f)
}

/// An efficient derived reactive value based on other reactive values.
///
/// Unlike a "derived signal," a memo comes with two guarantees:
/// 1. The memo will only run *once* per change, no matter how many times you
/// access its value.
/// 2. The memo will only notify its dependents if the value of the computation changes.
///
/// This makes a memo the perfect tool for expensive computations.
///
/// Memos have a certain overhead compared to derived signals. In most cases, you should
/// create a derived signal. But if the derivation calculation is expensive, you should
/// create a memo.
///
/// As with [create_effect](crate::create_effect), the argument to the memo function is the previous value,
/// i.e., the current value of the memo, which will be `None` for the initial calculation.
///
/// ```
/// # use leptos_reactive::*;
/// # fn really_expensive_computation(value: i32) -> i32 { value };
/// # create_scope(create_runtime(), |cx| {
/// let (value, set_value) = create_signal(cx, 0);
///
/// // 🆗 we could create a derived signal with a simple function
/// let double_value = move || value() * 2;
/// set_value(2);
/// assert_eq!(double_value(), 4);
///
/// // but imagine the computation is really expensive
/// let expensive = move || really_expensive_computation(value()); // lazy: doesn't run until called
/// create_effect(cx, move |_| {
///   // 🆗 run #1: calls `really_expensive_computation` the first time
///   log::debug!("expensive = {}", expensive());
/// });
/// create_effect(cx, move |_| {
///   // ❌ run #2: this calls `really_expensive_computation` a second time!
///   let value = expensive();
///   // do something else...
/// });
///
/// // instead, we create a memo
/// // 🆗 run #1: the calculation runs once immediately
/// let memoized = create_memo(cx, move |_| really_expensive_computation(value()));
/// create_effect(cx, move |_| {
///  // 🆗 reads the current value of the memo
///   log::debug!("memoized = {}", memoized());
/// });
/// create_effect(cx, move |_| {
///   // ✅ reads the current value **without re-running the calculation**
///   let value = memoized();
///   // do something else...
/// });
/// # }).dispose();
/// ```
#[derive(Debug, PartialEq, Eq)]
pub struct Memo<T>(
    pub(crate) ReadSignal<Option<T>>,
    #[cfg(debug_assertions)] pub(crate) &'static std::panic::Location<'static>,
)
where
    T: 'static;

impl<T> Clone for Memo<T>
where
    T: 'static,
{
    fn clone(&self) -> Self {
        Self(
            self.0,
            #[cfg(debug_assertions)]
            self.1,
        )
    }
}

impl<T> Copy for Memo<T> {}

impl<T> UntrackedGettableSignal<T> for Memo<T> {
    #[cfg_attr(
        debug_assertions,
        instrument(
            level = "trace",
            name = "Memo::get_untracked()",
            skip_all,
            fields(
                id = ?self.0.id,
                defined_at = %self.1,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    fn get_untracked(&self) -> T
    where
        T: Clone,
    {
        // Unwrapping is fine because `T` will already be `Some(T)` by
        // the time this method can be called
        self.0.get_untracked().unwrap()
    }
}

impl<T> UntrackedRefSignal<T> for Memo<T> {
    #[cfg_attr(
        debug_assertions,
        instrument(
            level = "trace",
            name = "Memo::with_untracked()",
            skip_all,
            fields(
                id = ?self.0.id,
                defined_at = %self.1,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    fn with_untracked<O>(&self, f: impl FnOnce(&T) -> O) -> O {
        // Unwrapping here is fine for the same reasons as <Memo as
        // UntrackedSignal>::get_untracked
        self.0.with_untracked(|v| f(v.as_ref().unwrap()))
    }
}

/// # Examples
///
/// ```
/// # use leptos_reactive::*;
/// # create_scope(create_runtime(), |cx| {
/// let (count, set_count) = create_signal(cx, 0);
/// let double_count = create_memo(cx, move |_| count() * 2);
///
/// assert_eq!(double_count.get(), 0);
/// set_count(1);
///
/// // double_count() is shorthand for double_count.get()
/// assert_eq!(double_count(), 2);
/// # }).dispose();
/// #
/// ```
impl<T: Clone> GettableSignal<T> for Memo<T> {
    #[cfg_attr(
        debug_assertions,
        instrument(
            name = "Memo::get()",
            level = "trace",
            skip_all,
            fields(
                id = ?self.0.id,
                defined_at = %self.1
            )
        )
    )]
    fn get(&self) -> T {
        self.0.get().unwrap()
    }

    #[cfg_attr(
        debug_assertions,
        instrument(
            level = "trace",
            name = "Memo::try_get()",
            skip_all,
            fields(
                id = ?self.0.id,
                defined_at = %self.1,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    fn try_get(&self) -> Option<T> {
        self.0.try_get().flatten()
    }
}

impl<T> RefSignal<T> for Memo<T> {
    #[cfg_attr(
        debug_assertions,
        instrument(
            level = "trace",
            name = "Memo::with()",
            skip_all,
            fields(
                id = ?self.0.id,
                defined_at = %self.1,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    fn with<O>(&self, f: impl FnOnce(&T) -> O) -> O {
        self.0.with(|t| f(t.as_ref().unwrap()))
    }

    #[cfg_attr(
        debug_assertions,
        instrument(
            level = "trace",
            name = "Memo::try_with()",
            skip_all,
            fields(
                id = ?self.0.id,
                defined_at = %self.1,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    fn try_with<O>(&self, f: impl FnOnce(&T) -> O) -> Option<O> {
        self.0.try_with(|t| f(t.as_ref().unwrap())).ok()
    }
}

impl<T> Memo<T>
where
    T: 'static,
{
    #[cfg(feature = "hydrate")]
    pub(crate) fn subscribe(&self) {
        self.0.subscribe()
    }
}

#[cfg(not(feature = "stable"))]
impl<T> FnOnce<()> for Memo<T>
where
    T: Clone,
{
    type Output = T;

    extern "rust-call" fn call_once(self, _args: ()) -> Self::Output {
        self.get()
    }
}

#[cfg(not(feature = "stable"))]
impl<T> FnMut<()> for Memo<T>
where
    T: Clone,
{
    extern "rust-call" fn call_mut(&mut self, _args: ()) -> Self::Output {
        self.get()
    }
}

#[cfg(not(feature = "stable"))]
impl<T> Fn<()> for Memo<T>
where
    T: Clone,
{
    extern "rust-call" fn call(&self, _args: ()) -> Self::Output {
        self.get()
    }
}
