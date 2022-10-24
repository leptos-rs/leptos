use crate::{ReadSignal, Scope, SignalError};
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
/// # create_scope(|cx| {
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
pub fn create_memo<T>(cx: Scope, f: impl FnMut(Option<&T>) -> T + 'static) -> Memo<T>
where
    T: PartialEq + Debug + 'static,
{
    cx.runtime.create_memo(f)
}

#[derive(Debug, PartialEq, Eq)]
pub struct Memo<T>(pub(crate) ReadSignal<Option<T>>)
where
    T: 'static;

impl<T> Clone for Memo<T>
where
    T: 'static,
{
    fn clone(&self) -> Self {
        Self(self.0)
    }
}

impl<T> Copy for Memo<T> {}

impl<T> Memo<T>
where
    T: 'static,
{
    pub fn get(&self) -> T
    where
        T: Clone,
    {
        self.with(T::clone)
    }

    pub fn with<U>(&self, f: impl Fn(&T) -> U) -> U {
        // okay to unwrap here, because the value will *always* have initially
        // been set by the effect, synchronously
        self.0
            .with(|n| f(n.as_ref().expect("Memo is missing its initial value")))
    }

    pub(crate) fn try_with<U>(&self, f: impl Fn(&T) -> U) -> Result<U, SignalError> {
        self.0
            .try_with(|n| f(n.as_ref().expect("Memo is missing its initial value")))
    }

    pub(crate) fn subscribe(&self) {
        self.0.subscribe()
    }
}

#[cfg(not(feature = "stable"))]
impl<T> FnOnce<()> for Memo<T>
where
    T: Debug + Clone,
{
    type Output = T;

    extern "rust-call" fn call_once(self, _args: ()) -> Self::Output {
        self.get()
    }
}

#[cfg(not(feature = "stable"))]
impl<T> FnMut<()> for Memo<T>
where
    T: Debug + Clone,
{
    extern "rust-call" fn call_mut(&mut self, _args: ()) -> Self::Output {
        self.get()
    }
}

#[cfg(not(feature = "stable"))]
impl<T> Fn<()> for Memo<T>
where
    T: Debug + Clone,
{
    extern "rust-call" fn call(&self, _args: ()) -> Self::Output {
        self.get()
    }
}
