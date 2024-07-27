use crate::{
    create_isomorphic_effect, diagnostics::AccessDiagnostics, node::NodeId,
    on_cleanup, with_runtime, AnyComputation, Runtime, SignalDispose,
    SignalGet, SignalGetUntracked, SignalStream, SignalWith,
    SignalWithUntracked,
};
use std::{any::Any, cell::RefCell, fmt, marker::PhantomData, rc::Rc};

// IMPLEMENTATION NOTE:
// Memos are implemented "lazily," i.e., the inner computation is not run
// when the memo is created or when its value is marked as stale, but on demand
// when it is accessed, if the value is stale. This means that the value is stored
// internally as Option<T>, even though it can always be accessed by the user as T.
// This means the inner value can be unwrapped in circumstances in which we know
// `Runtime::update_if_necessary()` has already been called, e.g., in the
// `.try_with_no_subscription()` calls below that are unwrapped with
// `.expect("invariant: must have already been initialized")`.

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
/// As with [`create_effect`](crate::create_effect), the argument to the memo function is the previous value,
/// i.e., the current value of the memo, which will be `None` for the initial calculation.
///
/// ```
/// # use leptos_reactive::*;
/// # fn really_expensive_computation(value: i32) -> i32 { value };
/// # let runtime = create_runtime();
/// let (value, set_value) = create_signal(0);
///
/// // 🆗 we could create a derived signal with a simple function
/// let double_value = move || value.get() * 2;
/// set_value.set(2);
/// assert_eq!(double_value(), 4);
///
/// // but imagine the computation is really expensive
/// let expensive = move || really_expensive_computation(value.get()); // lazy: doesn't run until called
/// create_effect(move |_| {
///   // 🆗 run #1: calls `really_expensive_computation` the first time
///   log::debug!("expensive = {}", expensive());
/// });
/// create_effect(move |_| {
///   // ❌ run #2: this calls `really_expensive_computation` a second time!
///   let value = expensive();
///   // do something else...
/// });
///
/// // instead, we create a memo
/// // 🆗 run #1: the calculation runs once immediately
/// let memoized = create_memo(move |_| really_expensive_computation(value.get()));
/// create_effect(move |_| {
///   // 🆗 reads the current value of the memo
///   //    can be `memoized()` on nightly
///   log::debug!("memoized = {}", memoized.get());
/// });
/// create_effect(move |_| {
///   // ✅ reads the current value **without re-running the calculation**
///   let value = memoized.get();
///   // do something else...
/// });
/// # runtime.dispose();
/// ```
#[cfg_attr(
    any(debug_assertions, feature="ssr"),
    instrument(
        level = "trace",
        skip_all,
        fields(
            ty = %std::any::type_name::<T>()
        )
    )
)]
#[track_caller]
#[inline(always)]
pub fn create_memo<T>(f: impl Fn(Option<&T>) -> T + 'static) -> Memo<T>
where
    T: PartialEq + 'static,
{
    Runtime::current().create_owning_memo(move |current_value| {
        let new_value = f(current_value.as_ref());
        let is_different = current_value.as_ref() != Some(&new_value);
        (new_value, is_different)
    })
}

/// Like [`create_memo`], `create_owning_memo` creates an efficient derived reactive value based on
/// other reactive values, but with two differences:
/// 1. The argument to the memo function is owned instead of borrowed.
/// 2. The function must also return whether the value has changed, as the first element of the tuple.
///
/// All of the other caveats and guarantees are the same as the usual "borrowing" memos.
///
/// This type of memo is useful for memos which can avoid computation by re-using the last value,
/// especially slices that need to allocate.
///
/// ```
/// # use leptos_reactive::*;
/// # fn really_expensive_computation(value: i32) -> i32 { value };
/// # let runtime = create_runtime();
/// pub struct State {
///     name: String,
///     token: String,
/// }
///
/// let state = create_rw_signal(State {
///     name: "Alice".to_owned(),
///     token: "abcdef".to_owned(),
/// });
///
/// // If we used `create_memo`, we'd need to allocate every time the state changes, but by using
/// // `create_owning_memo` we can allocate only when `state.name` changes.
/// let name = create_owning_memo(move |old_name| {
///     state.with(move |state| {
///         if let Some(name) =
///             old_name.filter(|old_name| old_name == &state.name)
///         {
///             (name, false)
///         } else {
///             (state.name.clone(), true)
///         }
///     })
/// });
/// let set_name = move |name| state.update(|state| state.name = name);
///
/// // We can also re-use the last allocation even when the value changes, which is usually faster,
/// // but may have some caveats (e.g. if the value size is drastically reduced, the memory will
/// // still be used for the life of the memo).
/// let token = create_owning_memo(move |old_token| {
///     state.with(move |state| {
///         let is_different = old_token.as_ref() != Some(&state.token);
///         let mut token = old_token.unwrap_or_else(String::new);
///
///         if is_different {
///             token.clone_from(&state.token);
///         }
///         (token, is_different)
///     })
/// });
/// let set_token = move |new_token| state.update(|state| state.token = new_token);
/// # runtime.dispose();
/// ```
#[cfg_attr(
    any(debug_assertions, feature="ssr"),
    instrument(
        level = "trace",
        skip_all,
        fields(
            ty = %std::any::type_name::<T>()
        )
    )
)]
#[track_caller]
#[inline(always)]
pub fn create_owning_memo<T>(
    f: impl Fn(Option<T>) -> (T, bool) + 'static,
) -> Memo<T>
where
    T: 'static,
{
    Runtime::current().create_owning_memo(f)
}

/// An efficient derived reactive value based on other reactive values.
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
/// As with [`create_effect`](crate::create_effect), the argument to the memo function is the previous value,
/// i.e., the current value of the memo, which will be `None` for the initial calculation.
///
/// ## Core Trait Implementations
/// - [`.get()`](#impl-SignalGet<T>-for-Memo<T>) (or calling the signal as a function) clones the current
///   value of the signal. If you call it within an effect, it will cause that effect
///   to subscribe to the signal, and to re-run whenever the value of the signal changes.
/// - [`.get_untracked()`](#impl-SignalGetUntracked<T>-for-Memo<T>) clones the value of the signal
///   without reactively tracking it.
/// - [`.with()`](#impl-SignalWith<T>-for-Memo<T>) allows you to reactively access the signal’s value without
///   cloning by applying a callback function.
/// - [`.with_untracked()`](#impl-SignalWithUntracked<T>-for-Memo<T>) allows you to access the signal’s
///   value without reactively tracking it.
/// - [`.to_stream()`](#impl-SignalStream<T>-for-Memo<T>) converts the signal to an `async` stream of values.
///
/// ## Examples
/// ```
/// # use leptos_reactive::*;
/// # fn really_expensive_computation(value: i32) -> i32 { value };
/// # let runtime = create_runtime();
/// let (value, set_value) = create_signal(0);
///
/// // 🆗 we could create a derived signal with a simple function
/// let double_value = move || value.get() * 2;
/// set_value.set(2);
/// assert_eq!(double_value(), 4);
///
/// // but imagine the computation is really expensive
/// let expensive = move || really_expensive_computation(value.get()); // lazy: doesn't run until called
/// create_effect(move |_| {
///   // 🆗 run #1: calls `really_expensive_computation` the first time
///   log::debug!("expensive = {}", expensive());
/// });
/// create_effect(move |_| {
///   // ❌ run #2: this calls `really_expensive_computation` a second time!
///   let value = expensive();
///   // do something else...
/// });
///
/// // instead, we create a memo
/// // 🆗 run #1: the calculation runs once immediately
/// let memoized = create_memo(move |_| really_expensive_computation(value.get()));
/// create_effect(move |_| {
///  // 🆗 reads the current value of the memo
///   log::debug!("memoized = {}", memoized.get());
/// });
/// create_effect(move |_| {
///   // ✅ reads the current value **without re-running the calculation**
///   //    can be `memoized()` on nightly
///   let value = memoized.get();
///   // do something else...
/// });
/// # runtime.dispose();
/// ```
pub struct Memo<T>
where
    T: 'static,
{
    pub(crate) id: NodeId,
    pub(crate) ty: PhantomData<T>,
    #[cfg(any(debug_assertions, feature = "ssr"))]
    pub(crate) defined_at: &'static std::panic::Location<'static>,
}

impl<T> Memo<T> {
    /// Creates a new memo from the given function.
    ///
    /// This is identical to [`create_memo`].
    /// ```
    /// # use leptos_reactive::*;
    /// # fn really_expensive_computation(value: i32) -> i32 { value };
    /// # let runtime = create_runtime();
    /// let value = RwSignal::new(0);
    ///
    /// // 🆗 we could create a derived signal with a simple function
    /// let double_value = move || value.get() * 2;
    /// value.set(2);
    /// assert_eq!(double_value(), 4);
    ///
    /// // but imagine the computation is really expensive
    /// let expensive = move || really_expensive_computation(value.get()); // lazy: doesn't run until called
    /// Effect::new(move |_| {
    ///   // 🆗 run #1: calls `really_expensive_computation` the first time
    ///   log::debug!("expensive = {}", expensive());
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
    ///   log::debug!("memoized = {}", memoized.get());
    /// });
    /// Effect::new(move |_| {
    ///   // ✅ reads the current value **without re-running the calculation**
    ///   let value = memoized.get();
    ///   // do something else...
    /// });
    /// # runtime.dispose();
    /// ```
    #[inline(always)]
    #[track_caller]
    pub fn new(f: impl Fn(Option<&T>) -> T + 'static) -> Memo<T>
    where
        T: PartialEq + 'static,
    {
        create_memo(f)
    }

    /// Creates a new owning memo from the given function.
    ///
    /// This is identical to [`create_owning_memo`].
    ///
    /// ```
    /// # use leptos_reactive::*;
    /// # fn really_expensive_computation(value: i32) -> i32 { value };
    /// # let runtime = create_runtime();
    /// pub struct State {
    ///     name: String,
    ///     token: String,
    /// }
    ///
    /// let state = RwSignal::new(State {
    ///     name: "Alice".to_owned(),
    ///     token: "abcdef".to_owned(),
    /// });
    ///
    /// // If we used `Memo::new`, we'd need to allocate every time the state changes, but by using
    /// // `Memo::new_owning` we can allocate only when `state.name` changes.
    /// let name = Memo::new_owning(move |old_name| {
    ///     state.with(move |state| {
    ///         if let Some(name) =
    ///             old_name.filter(|old_name| old_name == &state.name)
    ///         {
    ///             (name, false)
    ///         } else {
    ///             (state.name.clone(), true)
    ///         }
    ///     })
    /// });
    /// let set_name = move |name| state.update(|state| state.name = name);
    ///
    /// // We can also re-use the last allocation even when the value changes, which is usually faster,
    /// // but may have some caveats (e.g. if the value size is drastically reduced, the memory will
    /// // still be used for the life of the memo).
    /// let token = Memo::new_owning(move |old_token| {
    ///     state.with(move |state| {
    ///         let is_different = old_token.as_ref() != Some(&state.token);
    ///         let mut token = old_token.unwrap_or_else(String::new);
    ///
    ///         if is_different {
    ///             token.clone_from(&state.token);
    ///         }
    ///         (token, is_different)
    ///     })
    /// });
    /// let set_token = move |new_token| state.update(|state| state.token = new_token);
    /// # runtime.dispose();
    /// ```
    #[inline(always)]
    #[track_caller]
    pub fn new_owning(f: impl Fn(Option<T>) -> (T, bool) + 'static) -> Memo<T>
    where
        T: 'static,
    {
        create_owning_memo(f)
    }
}

impl<T> Clone for Memo<T>
where
    T: 'static,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Memo<T> {}

impl<T> fmt::Debug for Memo<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = f.debug_struct("Memo");
        s.field("id", &self.id);
        s.field("ty", &self.ty);
        #[cfg(any(debug_assertions, feature = "ssr"))]
        s.field("defined_at", &self.defined_at);
        s.finish()
    }
}

impl<T> Eq for Memo<T> {}

impl<T> PartialEq for Memo<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

fn forward_ref_to<T, O, F: FnOnce(&T) -> O>(
    f: F,
) -> impl FnOnce(&Option<T>) -> O {
    |maybe_value: &Option<T>| {
        let ref_t = maybe_value
            .as_ref()
            .expect("invariant: must have already been initialized");
        f(ref_t)
    }
}

impl<T: Clone> SignalGetUntracked for Memo<T> {
    type Value = T;

    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "Memo::get_untracked()",
            skip_all,
            fields(
                id = ?self.id,
                defined_at = %self.defined_at,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    fn get_untracked(&self) -> T {
        with_runtime(move |runtime| {
            let f = |maybe_value: &Option<T>| {
                maybe_value
                    .clone()
                    .expect("invariant: must have already been initialized")
            };
            match self.id.try_with_no_subscription(runtime, f) {
                Ok(t) => t,
                Err(_) => panic_getting_dead_memo(
                    #[cfg(any(debug_assertions, feature = "ssr"))]
                    self.defined_at,
                ),
            }
        })
        .expect("runtime to be alive")
    }

    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "Memo::try_get_untracked()",
            skip_all,
            fields(
                id = ?self.id,
                defined_at = %self.defined_at,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    #[inline(always)]
    fn try_get_untracked(&self) -> Option<T> {
        self.try_with_untracked(T::clone)
    }
}

impl<T> SignalWithUntracked for Memo<T> {
    type Value = T;

    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "Memo::with_untracked()",
            skip_all,
            fields(
                id = ?self.id,
                defined_at = %self.defined_at,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    fn with_untracked<O>(&self, f: impl FnOnce(&T) -> O) -> O {
        with_runtime(|runtime| {
            match self.id.try_with_no_subscription(runtime, forward_ref_to(f)) {
                Ok(t) => t,
                Err(_) => panic_getting_dead_memo(
                    #[cfg(any(debug_assertions, feature = "ssr"))]
                    self.defined_at,
                ),
            }
        })
        .expect("runtime to be alive")
    }

    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "Memo::try_with_untracked()",
            skip_all,
            fields(
                id = ?self.id,
                defined_at = %self.defined_at,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    #[inline]
    fn try_with_untracked<O>(&self, f: impl FnOnce(&T) -> O) -> Option<O> {
        with_runtime(|runtime| {
            self.id
                .try_with_no_subscription(runtime, |v: &Option<T>| {
                    v.as_ref().map(f)
                })
                .ok()
                .flatten()
        })
        .ok()
        .flatten()
    }
}

/// # Examples
///
/// ```
/// # use leptos_reactive::*;
/// # let runtime = create_runtime();
/// let (count, set_count) = create_signal(0);
/// let double_count = create_memo(move |_| count.get() * 2);
///
/// assert_eq!(double_count.get(), 0);
/// set_count.set(1);
///
/// // can be `double_count()` on nightly
/// // assert_eq!(double_count(), 2);
/// assert_eq!(double_count.get(), 2);
/// # runtime.dispose();
/// #
/// ```
impl<T: Clone> SignalGet for Memo<T> {
    type Value = T;

    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            name = "Memo::get()",
            level = "trace",
            skip_all,
            fields(
                id = ?self.id,
                defined_at = %self.defined_at,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    #[track_caller]
    #[inline(always)]
    fn get(&self) -> T {
        self.with(T::clone)
    }

    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "Memo::try_get()",
            skip_all,
            fields(
                id = ?self.id,
                defined_at = %self.defined_at,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    #[track_caller]
    #[inline(always)]
    fn try_get(&self) -> Option<T> {
        self.try_with(T::clone)
    }
}

impl<T> SignalWith for Memo<T> {
    type Value = T;

    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "Memo::with()",
            skip_all,
            fields(
                id = ?self.id,
                defined_at = %self.defined_at,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    #[track_caller]
    fn with<O>(&self, f: impl FnOnce(&T) -> O) -> O {
        match self.try_with(f) {
            Some(t) => t,
            None => panic_getting_dead_memo(
                #[cfg(any(debug_assertions, feature = "ssr"))]
                self.defined_at,
            ),
        }
    }

    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "Memo::try_with()",
            skip_all,
            fields(
                id = ?self.id,
                defined_at = %self.defined_at,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    #[track_caller]
    fn try_with<O>(&self, f: impl FnOnce(&T) -> O) -> Option<O> {
        let diagnostics = diagnostics!(self);

        with_runtime(|runtime| {
            self.id.subscribe(runtime, diagnostics);
            self.id
                .try_with_no_subscription(runtime, forward_ref_to(f))
                .ok()
        })
        .ok()
        .flatten()
    }
}

impl<T: Clone> SignalStream<T> for Memo<T> {
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "Memo::to_stream()",
            skip_all,
            fields(
                id = ?self.id,
                defined_at = %self.defined_at,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    fn to_stream(&self) -> std::pin::Pin<Box<dyn futures::Stream<Item = T>>> {
        let (tx, rx) = futures::channel::mpsc::unbounded();

        let close_channel = tx.clone();

        on_cleanup(move || close_channel.close_channel());

        let this = *self;

        create_isomorphic_effect(move |_| {
            let _ = tx.unbounded_send(this.get());
        });

        Box::pin(rx)
    }
}

impl<T> SignalDispose for Memo<T> {
    fn dispose(self) {
        _ = with_runtime(|runtime| runtime.dispose_node(self.id));
    }
}

impl_get_fn_traits![Memo];

pub(crate) struct MemoState<T, F>
where
    T: 'static,
    F: Fn(Option<T>) -> (T, bool),
{
    pub f: F,
    pub t: PhantomData<T>,
    #[cfg(any(debug_assertions, feature = "ssr"))]
    pub(crate) defined_at: &'static std::panic::Location<'static>,
}

impl<T, F> AnyComputation for MemoState<T, F>
where
    T: 'static,
    F: Fn(Option<T>) -> (T, bool),
{
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            name = "Memo::run()",
            level = "trace",
            skip_all,
            fields(
              defined_at = %self.defined_at,
              ty = %std::any::type_name::<T>()
            )
        )
    )]
    fn run(&self, value: Rc<RefCell<dyn Any>>) -> bool {
        let mut value = value.borrow_mut();
        let curr_value = value
            .downcast_mut::<Option<T>>()
            .expect("to downcast memo value");

        // run the memo
        let (new_value, is_different) = (self.f)(curr_value.take());

        // set new value
        *curr_value = Some(new_value);

        is_different
    }
}

#[cold]
#[inline(never)]
#[track_caller]
fn format_memo_warning(
    msg: &str,
    #[cfg(any(debug_assertions, feature = "ssr"))]
    defined_at: &'static std::panic::Location<'static>,
) -> String {
    let location = std::panic::Location::caller();

    let defined_at_msg = {
        #[cfg(any(debug_assertions, feature = "ssr"))]
        {
            format!("signal created here: {defined_at}\n")
        }

        #[cfg(not(any(debug_assertions, feature = "ssr")))]
        {
            String::default()
        }
    };

    format!("{msg}\n{defined_at_msg}warning happened here: {location}",)
}

#[cold]
#[inline(never)]
#[track_caller]
pub(crate) fn panic_getting_dead_memo(
    #[cfg(any(debug_assertions, feature = "ssr"))]
    defined_at: &'static std::panic::Location<'static>,
) -> ! {
    panic!(
        "{}",
        format_memo_warning(
            "Attempted to get a memo after it was disposed.",
            #[cfg(any(debug_assertions, feature = "ssr"))]
            defined_at,
        )
    )
}
