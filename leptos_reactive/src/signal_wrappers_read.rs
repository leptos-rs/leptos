use crate::{
    create_isomorphic_effect, on_cleanup, runtime::untrack, store_value, Memo,
    Oco, ReadSignal, RwSignal, SignalDispose, SignalGet, SignalGetUntracked,
    SignalStream, SignalWith, SignalWithUntracked, StoredValue,
};
use std::{borrow::Cow, fmt::Debug, rc::Rc};

/// Helper trait for converting `Fn() -> T` closures into
/// [`Signal<T>`].
pub trait IntoSignal: Sized {
    /// The value yielded by the signal.
    type Value;

    /// Consumes `self`, returning a [`Signal<T>`].
    #[deprecated = "Will be removed in `leptos v0.6`. Please use \
                    `IntoSignal::into_signal()` instead."]
    fn derive_signal(self) -> Signal<Self::Value>;

    /// Consumes `self`, returning a [`Signal<T>`].
    fn into_signal(self) -> Signal<Self::Value>;
}

impl<F, T> IntoSignal for F
where
    F: Fn() -> T + 'static,
{
    type Value = T;

    fn derive_signal(self) -> Signal<T> {
        self.into_signal()
    }

    fn into_signal(self) -> Signal<Self::Value> {
        Signal::derive(self)
    }
}

/// A wrapper for any kind of readable reactive signal: a [`ReadSignal`](crate::ReadSignal),
/// [`Memo`](crate::Memo), [`RwSignal`](crate::RwSignal), or derived signal closure.
///
/// This allows you to create APIs that take any kind of `Signal<T>` as an argument,
/// rather than adding a generic `F: Fn() -> T`. Values can be access with the same
/// function call, `with()`, and `get()` APIs as other signals.
///
/// ## Core Trait Implementations
/// - [`.get()`](#impl-SignalGet-for-Signal<T>) (or calling the signal as a function) clones the current
///   value of the signal. If you call it within an effect, it will cause that effect
///   to subscribe to the signal, and to re-run whenever the value of the signal changes.
/// - [`.get_untracked()`](#impl-SignalGetUntracked<T>-for-Signal<T>) clones the value of the signal
///   without reactively tracking it.
/// - [`.with()`](#impl-SignalWith-for-Signal<T>) allows you to reactively access the signal’s value without
///   cloning by applying a callback function.
/// - [`.with_untracked()`](#impl-SignalWithUntracked<T>-for-Signal<T>) allows you to access the signal’s
///   value without reactively tracking it.
/// - [`.to_stream()`](#impl-SignalStream<T>-for-Signal<T>) converts the signal to an `async` stream of values.
///
/// ## Examples
/// ```rust
/// # use leptos_reactive::*;
/// # let runtime = create_runtime();
/// let (count, set_count) = create_signal(2);
/// let double_count = Signal::derive(move || count.get() * 2);
/// let memoized_double_count = create_memo(move |_| count.get() * 2);
///
/// // this function takes any kind of wrapped signal
/// fn above_3(arg: &Signal<i32>) -> bool {
///     // ✅ calling the signal clones and returns the value
///     //    can be `arg() > 3` on nightly
///     arg.get() > 3
/// }
///
/// assert_eq!(above_3(&count.into()), false);
/// assert_eq!(above_3(&double_count), true);
/// assert_eq!(above_3(&memoized_double_count.into()), true);
/// # runtime.dispose();
/// ```
pub struct Signal<T>
where
    T: 'static,
{
    inner: SignalTypes<T>,
    #[cfg(any(debug_assertions, feature = "ssr"))]
    defined_at: &'static std::panic::Location<'static>,
}

impl<T> Clone for Signal<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Signal<T> {}

impl<T> core::fmt::Debug for Signal<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut s = f.debug_struct("Signal");
        s.field("inner", &self.inner);
        #[cfg(any(debug_assertions, feature = "ssr"))]
        s.field("defined_at", &self.defined_at);
        s.finish()
    }
}

impl<T> Eq for Signal<T> {}

impl<T> PartialEq for Signal<T> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

/// Please note that using `Signal::with_untracked` still clones the inner value,
/// so there's no benefit to using it as opposed to calling
/// `Signal::get_untracked`.
impl<T: Clone> SignalGetUntracked for Signal<T> {
    type Value = T;

    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "Signal::get_untracked()",
            skip_all,
            fields(
                defined_at = %self.defined_at,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    fn get_untracked(&self) -> T {
        match &self.inner {
            SignalTypes::ReadSignal(s) => s.get_untracked(),
            SignalTypes::Memo(m) => m.get_untracked(),
            SignalTypes::DerivedSignal(f) => untrack(|| f.with_value(|f| f())),
        }
    }

    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "Signal::try_get_untracked()",
            skip_all,
            fields(
                defined_at = %self.defined_at,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    fn try_get_untracked(&self) -> Option<T> {
        match &self.inner {
            SignalTypes::ReadSignal(s) => s.try_get_untracked(),
            SignalTypes::Memo(m) => m.try_get_untracked(),
            SignalTypes::DerivedSignal(f) => {
                untrack(|| f.try_with_value(|f| f()))
            }
        }
    }
}

impl<T> SignalWithUntracked for Signal<T> {
    type Value = T;

    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "Signal::with_untracked()",
            skip_all,
            fields(
                defined_at = %self.defined_at,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    fn with_untracked<O>(&self, f: impl FnOnce(&T) -> O) -> O {
        match &self.inner {
            SignalTypes::ReadSignal(s) => s.with_untracked(f),
            SignalTypes::Memo(s) => s.with_untracked(f),
            SignalTypes::DerivedSignal(v_f) => {
                let mut o = None;

                untrack(|| o = Some(f(&v_f.with_value(|v_f| v_f()))));

                o.unwrap()
            }
        }
    }

    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "Signal::try_with_untracked()",
            skip_all,
            fields(
                defined_at = %self.defined_at,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    fn try_with_untracked<O>(&self, f: impl FnOnce(&T) -> O) -> Option<O> {
        match self.inner {
            SignalTypes::ReadSignal(r) => r.try_with_untracked(f),
            SignalTypes::Memo(m) => m.try_with_untracked(f),
            SignalTypes::DerivedSignal(s) => {
                untrack(move || s.try_with_value(|t| f(&t())))
            }
        }
    }
}

/// # Examples
///
/// ```
/// # use leptos_reactive::*;
/// # let runtime = create_runtime();
/// let (name, set_name) = create_signal("Alice".to_string());
/// let name_upper = Signal::derive(move || name.with(|n| n.to_uppercase()));
/// let memoized_lower = create_memo(move |_| name.with(|n| n.to_lowercase()));
///
/// // this function takes any kind of wrapped signal
/// fn current_len_inefficient(arg: Signal<String>) -> usize {
///     // ❌ unnecessarily clones the string
///     arg.get().len()
/// }
///
/// fn current_len(arg: &Signal<String>) -> usize {
///     // ✅ gets the length without cloning the `String`
///     arg.with(|value| value.len())
/// }
///
/// assert_eq!(current_len(&name.into()), 5);
/// assert_eq!(current_len(&name_upper), 5);
/// assert_eq!(current_len(&memoized_lower.into()), 5);
///
/// assert_eq!(name.get(), "Alice");
/// assert_eq!(name_upper.get(), "ALICE");
/// assert_eq!(memoized_lower.get(), "alice");
/// # runtime.dispose();
/// ```
impl<T> SignalWith for Signal<T> {
    type Value = T;

    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "Signal::with()",
            skip_all,
            fields(
                defined_at = %self.defined_at,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    fn with<U>(&self, f: impl FnOnce(&T) -> U) -> U {
        match &self.inner {
            SignalTypes::ReadSignal(s) => s.with(f),
            SignalTypes::Memo(s) => s.with(f),
            SignalTypes::DerivedSignal(s) => f(&s.with_value(|s| s())),
        }
    }

    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "Signal::try_with()",
            skip_all,
            fields(
                defined_at = %self.defined_at,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    fn try_with<O>(&self, f: impl FnOnce(&T) -> O) -> Option<O> {
        match self.inner {
            SignalTypes::ReadSignal(r) => r.try_with(f).ok(),

            SignalTypes::Memo(m) => m.try_with(f),
            SignalTypes::DerivedSignal(s) => s.try_with_value(|t| f(&t())),
        }
    }
}

/// # Examples
///
/// ```
/// # use leptos_reactive::*;
/// # let runtime = create_runtime();
/// let (count, set_count) = create_signal(2);
/// let double_count = Signal::derive(move || count.get() * 2);
/// let memoized_double_count = create_memo(move |_| count.get() * 2);
///
/// // this function takes any kind of wrapped signal
/// fn above_3(arg: &Signal<i32>) -> bool {
///     arg.get() > 3
/// }
///
/// assert_eq!(above_3(&count.into()), false);
/// assert_eq!(above_3(&double_count), true);
/// assert_eq!(above_3(&memoized_double_count.into()), true);
/// # runtime.dispose();
/// ```
impl<T: Clone> SignalGet for Signal<T> {
    type Value = T;

    fn get(&self) -> T {
        match self.inner {
            SignalTypes::ReadSignal(r) => r.get(),
            SignalTypes::Memo(m) => m.get(),
            SignalTypes::DerivedSignal(s) => s.with_value(|t| t()),
        }
    }

    fn try_get(&self) -> Option<T> {
        match self.inner {
            SignalTypes::ReadSignal(r) => r.try_get(),
            SignalTypes::Memo(m) => m.try_get(),
            SignalTypes::DerivedSignal(s) => s.try_with_value(|t| t()),
        }
    }
}

impl<T> SignalDispose for Signal<T> {
    fn dispose(self) {
        match self.inner {
            SignalTypes::ReadSignal(s) => s.dispose(),
            SignalTypes::Memo(m) => m.dispose(),
            SignalTypes::DerivedSignal(s) => s.dispose(),
        }
    }
}

impl<T: Clone> SignalStream<T> for Signal<T> {
    fn to_stream(&self) -> std::pin::Pin<Box<dyn futures::Stream<Item = T>>> {
        match self.inner {
            SignalTypes::ReadSignal(r) => r.to_stream(),
            SignalTypes::Memo(m) => m.to_stream(),
            SignalTypes::DerivedSignal(s) => {
                let (tx, rx) = futures::channel::mpsc::unbounded();

                let close_channel = tx.clone();

                on_cleanup(move || close_channel.close_channel());

                create_isomorphic_effect(move |_| {
                    let _ = s.try_with_value(|t| tx.unbounded_send(t()));
                });

                Box::pin(rx)
            }
        }
    }
}

impl<T> Signal<T>
where
    T: 'static,
{
    /// Wraps a derived signal, i.e., any computation that accesses one or more
    /// reactive signals.
    /// ```rust
    /// # use leptos_reactive::*;
    /// # let runtime = create_runtime();
    /// let (count, set_count) = create_signal(2);
    /// let double_count = Signal::derive(move || count.get() * 2);
    ///
    /// // this function takes any kind of wrapped signal
    /// fn above_3(arg: &Signal<i32>) -> bool {
    ///     arg.get() > 3
    /// }
    ///
    /// assert_eq!(above_3(&count.into()), false);
    /// assert_eq!(above_3(&double_count), true);
    /// # runtime.dispose();
    /// ```
    #[track_caller]
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(level = "trace", skip_all)
    )]
    pub fn derive(derived_signal: impl Fn() -> T + 'static) -> Self {
        let span = ::tracing::Span::current();

        let derived_signal = move || {
            let _guard = span.enter();
            derived_signal()
        };

        Self {
            inner: SignalTypes::DerivedSignal(store_value(Box::new(
                derived_signal,
            ))),
            #[cfg(any(debug_assertions, feature = "ssr"))]
            defined_at: std::panic::Location::caller(),
        }
    }
}

impl<T> Default for Signal<T>
where
    T: Default,
{
    fn default() -> Self {
        Self::derive(|| Default::default())
    }
}

impl<T> From<ReadSignal<T>> for Signal<T> {
    #[track_caller]
    fn from(value: ReadSignal<T>) -> Self {
        Self {
            inner: SignalTypes::ReadSignal(value),
            #[cfg(any(debug_assertions, feature = "ssr"))]
            defined_at: std::panic::Location::caller(),
        }
    }
}

impl<T> From<RwSignal<T>> for Signal<T> {
    #[track_caller]
    fn from(value: RwSignal<T>) -> Self {
        Self {
            inner: SignalTypes::ReadSignal(value.read_only()),
            #[cfg(any(debug_assertions, feature = "ssr"))]
            defined_at: std::panic::Location::caller(),
        }
    }
}

impl<T> From<Memo<T>> for Signal<T> {
    #[track_caller]
    fn from(value: Memo<T>) -> Self {
        Self {
            inner: SignalTypes::Memo(value),
            #[cfg(any(debug_assertions, feature = "ssr"))]
            defined_at: std::panic::Location::caller(),
        }
    }
}

enum SignalTypes<T>
where
    T: 'static,
{
    ReadSignal(ReadSignal<T>),
    Memo(Memo<T>),
    DerivedSignal(StoredValue<Box<dyn Fn() -> T>>),
}

impl<T> Clone for SignalTypes<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for SignalTypes<T> {}

impl<T> core::fmt::Debug for SignalTypes<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::ReadSignal(arg0) => {
                f.debug_tuple("ReadSignal").field(arg0).finish()
            }
            Self::Memo(arg0) => f.debug_tuple("Memo").field(arg0).finish(),
            Self::DerivedSignal(_) => f.debug_tuple("DerivedSignal").finish(),
        }
    }
}

impl<T> PartialEq for SignalTypes<T> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::ReadSignal(l0), Self::ReadSignal(r0)) => l0 == r0,
            (Self::Memo(l0), Self::Memo(r0)) => l0 == r0,
            (Self::DerivedSignal(l0), Self::DerivedSignal(r0)) => {
                std::ptr::eq(l0, r0)
            }
            _ => false,
        }
    }
}

impl<T> Eq for SignalTypes<T> where T: PartialEq {}

/// A wrapper for a value that is *either* `T` or [`Signal<T>`](crate::Signal).
///
/// This allows you to create APIs that take either a reactive or a non-reactive value
/// of the same type. This is especially useful for component properties.
///
/// ## Core Trait Implementations
/// - [`.get()`](#impl-SignalGet-for-MaybeSignal<T>) (or calling the signal as a function) clones the current
///   value of the signal. If you call it within an effect, it will cause that effect
///   to subscribe to the signal, and to re-run whenever the value of the signal changes.
/// - [`.get_untracked()`](#impl-SignalGetUntracked<T>-for-MaybeSignal<T>) clones the value of the signal
///   without reactively tracking it.
/// - [`.with()`](#impl-SignalWith-for-MaybeSignal<T>) allows you to reactively access the signal’s value without
///   cloning by applying a callback function.
/// - [`.with_untracked()`](#impl-SignalWithUntracked<T>-for-MaybeSignal<T>) allows you to access the signal’s
///   value without reactively tracking it.
/// - [`.to_stream()`](#impl-SignalStream<T>-for-MaybeSignal<T>) converts the signal to an `async` stream of values.
///
/// ## Examples
/// ```rust
/// # use leptos_reactive::*;
/// # let runtime = create_runtime();
/// let (count, set_count) = create_signal(2);
/// let double_count = MaybeSignal::derive(move || count.get() * 2);
/// let memoized_double_count = create_memo(move |_| count.get() * 2);
/// let static_value = 5;
///
/// // this function takes either a reactive or non-reactive value
/// fn above_3(arg: &MaybeSignal<i32>) -> bool {
///     // ✅ calling the signal clones and returns the value
///     //    it is a shorthand for arg.get()
///     arg.get() > 3
/// }
///
/// assert_eq!(above_3(&static_value.into()), true);
/// assert_eq!(above_3(&count.into()), false);
/// assert_eq!(above_3(&double_count), true);
/// assert_eq!(above_3(&memoized_double_count.into()), true);
/// # runtime.dispose();
/// ```
#[derive(Debug, PartialEq, Eq)]
pub enum MaybeSignal<T>
where
    T: 'static,
{
    /// An unchanging value of type `T`.
    Static(T),
    /// A reactive signal that contains a value of type `T`.
    Dynamic(Signal<T>),
}

impl<T: Clone> Clone for MaybeSignal<T> {
    fn clone(&self) -> Self {
        match self {
            Self::Static(item) => Self::Static(item.clone()),
            Self::Dynamic(signal) => Self::Dynamic(*signal),
        }
    }
}

impl<T: Copy> Copy for MaybeSignal<T> {}

impl<T: Default> Default for MaybeSignal<T> {
    fn default() -> Self {
        Self::Static(Default::default())
    }
}

/// # Examples
///
/// ```
/// # use leptos_reactive::*;
/// # let runtime = create_runtime();
/// let (count, set_count) = create_signal(2);
/// let double_count = MaybeSignal::derive(move || count.get() * 2);
/// let memoized_double_count = create_memo(move |_| count.get() * 2);
/// let static_value: MaybeSignal<i32> = 5.into();
///
/// // this function takes any kind of wrapped signal
/// fn above_3(arg: &MaybeSignal<i32>) -> bool {
///     arg.get() > 3
/// }
///
/// assert_eq!(above_3(&count.into()), false);
/// assert_eq!(above_3(&double_count), true);
/// assert_eq!(above_3(&memoized_double_count.into()), true);
/// assert_eq!(above_3(&static_value.into()), true);
/// # runtime.dispose();
/// ```
impl<T: Clone> SignalGet for MaybeSignal<T> {
    type Value = T;

    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "MaybeSignal::get()",
            skip_all,
            fields(ty = %std::any::type_name::<T>())
        )
    )]
    fn get(&self) -> T {
        match self {
            Self::Static(t) => t.clone(),
            Self::Dynamic(s) => s.get(),
        }
    }

    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "MaybeSignal::try_get()",
            skip_all,
            fields(ty = %std::any::type_name::<T>())
        )
    )]
    fn try_get(&self) -> Option<T> {
        match self {
            Self::Static(t) => Some(t.clone()),
            Self::Dynamic(s) => s.try_get(),
        }
    }
}

/// # Examples
///
/// ```
/// # use leptos_reactive::*;
/// # let runtime = create_runtime();
/// let (name, set_name) = create_signal("Alice".to_string());
/// let name_upper =
///     MaybeSignal::derive(move || name.with(|n| n.to_uppercase()));
/// let memoized_lower = create_memo(move |_| name.with(|n| n.to_lowercase()));
/// let static_value: MaybeSignal<String> = "Bob".to_string().into();
///
/// // this function takes any kind of wrapped signal
/// fn current_len_inefficient(arg: &MaybeSignal<String>) -> usize {
///     // ❌ unnecessarily clones the string
///     arg.get().len()
/// }
///
/// fn current_len(arg: &MaybeSignal<String>) -> usize {
///     // ✅ gets the length without cloning the `String`
///     arg.with(|value| value.len())
/// }
///
/// assert_eq!(current_len(&name.into()), 5);
/// assert_eq!(current_len(&name_upper), 5);
/// assert_eq!(current_len(&memoized_lower.into()), 5);
/// assert_eq!(current_len(&static_value), 3);
///
/// assert_eq!(name.get(), "Alice");
/// assert_eq!(name_upper.get(), "ALICE");
/// assert_eq!(memoized_lower.get(), "alice");
/// assert_eq!(static_value.get(), "Bob");
/// # runtime.dispose();
/// ```
impl<T> SignalWith for MaybeSignal<T> {
    type Value = T;

    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "MaybeSignal::with()",
            skip_all,
            fields(ty = %std::any::type_name::<T>())
        )
    )]
    fn with<O>(&self, f: impl FnOnce(&T) -> O) -> O {
        match self {
            Self::Static(t) => f(t),
            Self::Dynamic(s) => s.with(f),
        }
    }

    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "MaybeSignal::try_with()",
            skip_all,
            fields(ty = %std::any::type_name::<T>())
        )
    )]
    fn try_with<O>(&self, f: impl FnOnce(&T) -> O) -> Option<O> {
        match self {
            Self::Static(t) => Some(f(t)),
            Self::Dynamic(s) => s.try_with(f),
        }
    }
}

impl<T> SignalWithUntracked for MaybeSignal<T> {
    type Value = T;

    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "MaybeSignal::with_untracked()",
            skip_all,
            fields(ty = %std::any::type_name::<T>())
        )
    )]
    fn with_untracked<O>(&self, f: impl FnOnce(&T) -> O) -> O {
        match self {
            Self::Static(t) => f(t),
            Self::Dynamic(s) => s.with_untracked(f),
        }
    }

    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "MaybeSignal::try_with_untracked()",
            skip_all,
            fields(ty = %std::any::type_name::<T>())
        )
    )]
    fn try_with_untracked<O>(&self, f: impl FnOnce(&T) -> O) -> Option<O> {
        match self {
            Self::Static(t) => Some(f(t)),
            Self::Dynamic(s) => s.try_with_untracked(f),
        }
    }
}

impl<T: Clone> SignalGetUntracked for MaybeSignal<T> {
    type Value = T;

    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "MaybeSignal::get_untracked()",
            skip_all,
            fields(ty = %std::any::type_name::<T>())
        )
    )]
    fn get_untracked(&self) -> T {
        match self {
            Self::Static(t) => t.clone(),
            Self::Dynamic(s) => s.get_untracked(),
        }
    }

    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "MaybeSignal::try_get_untracked()",
            skip_all,
            fields(ty = %std::any::type_name::<T>())
        )
    )]
    fn try_get_untracked(&self) -> Option<T> {
        match self {
            Self::Static(t) => Some(t.clone()),
            Self::Dynamic(s) => s.try_get_untracked(),
        }
    }
}

impl<T: Clone> SignalStream<T> for MaybeSignal<T> {
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "MaybeSignal::to_stream()",
            skip_all,
            fields(ty = %std::any::type_name::<T>())
        )
    )]
    fn to_stream(&self) -> std::pin::Pin<Box<dyn futures::Stream<Item = T>>> {
        match self {
            Self::Static(t) => {
                let t = t.clone();

                let stream = futures::stream::once(async move { t });

                Box::pin(stream)
            }
            Self::Dynamic(s) => s.to_stream(),
        }
    }
}

impl<T> MaybeSignal<T>
where
    T: 'static,
{
    /// Wraps a derived signal, i.e., any computation that accesses one or more
    /// reactive signals.
    /// ```rust
    /// # use leptos_reactive::*;
    /// # let runtime = create_runtime();
    /// let (count, set_count) = create_signal(2);
    /// let double_count = Signal::derive(move || count.get() * 2);
    ///
    /// // this function takes any kind of wrapped signal
    /// fn above_3(arg: &MaybeSignal<i32>) -> bool {
    ///     arg.get() > 3
    /// }
    ///
    /// assert_eq!(above_3(&count.into()), false);
    /// assert_eq!(above_3(&double_count.into()), true);
    /// assert_eq!(above_3(&2.into()), false);
    /// # runtime.dispose();
    /// ```
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "MaybeSignal::derive()",
            skip_all,
            fields(
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    pub fn derive(derived_signal: impl Fn() -> T + 'static) -> Self {
        Self::Dynamic(Signal::derive(derived_signal))
    }
}

impl<T> From<T> for MaybeSignal<T> {
    fn from(value: T) -> Self {
        Self::Static(value)
    }
}

impl<T> From<ReadSignal<T>> for MaybeSignal<T> {
    fn from(value: ReadSignal<T>) -> Self {
        Self::Dynamic(value.into())
    }
}

impl<T> From<RwSignal<T>> for MaybeSignal<T> {
    fn from(value: RwSignal<T>) -> Self {
        Self::Dynamic(value.into())
    }
}

impl<T> From<Memo<T>> for MaybeSignal<T> {
    fn from(value: Memo<T>) -> Self {
        Self::Dynamic(value.into())
    }
}

impl<T> From<Signal<T>> for MaybeSignal<T> {
    fn from(value: Signal<T>) -> Self {
        Self::Dynamic(value)
    }
}

impl From<&str> for MaybeSignal<String> {
    fn from(value: &str) -> Self {
        Self::Static(value.to_string())
    }
}

#[cfg(feature = "nightly")]
mod from_fn_for_signals {
    use super::{MaybeSignal, Memo, ReadSignal, RwSignal, Signal};
    auto trait NotSignalMarker {}

    impl<T> !NotSignalMarker for Signal<T> {}
    impl<T> !NotSignalMarker for ReadSignal<T> {}
    impl<T> !NotSignalMarker for Memo<T> {}
    impl<T> !NotSignalMarker for RwSignal<T> {}
    impl<T> !NotSignalMarker for MaybeSignal<T> {}

    impl<F, T> From<F> for Signal<T>
    where
        F: Fn() -> T + NotSignalMarker + 'static,
    {
        fn from(value: F) -> Self {
            Signal::derive(value)
        }
    }
}
#[cfg(not(feature = "nightly"))]
impl<F, T> From<F> for Signal<T>
where
    F: Fn() -> T + 'static,
{
    fn from(value: F) -> Self {
        Signal::derive(value)
    }
}

/// A wrapping type for an optional component prop, which can either be a signal or a
/// non-reactive value, and which may or may not have a value. In other words, this is
/// an `Option<MaybeSignal<Option<T>>>` that automatically flattens its getters.
///
/// This creates an extremely flexible type for component libraries, etc.
///
/// ## Core Trait Implementations
/// - [`.get()`](#impl-SignalGet-for-MaybeProp<T>) (or calling the signal as a function) clones the current
///   value of the signal. If you call it within an effect, it will cause that effect
///   to subscribe to the signal, and to re-run whenever the value of the signal changes.
///   - [`.get_untracked()`](#impl-SignalGetUntracked<T>-for-MaybeProp<T>) clones the value of the signal
///     without reactively tracking it.
/// - [`.with()`](#impl-SignalWith-for-MaybeProp<T>) allows you to reactively access the signal’s value without
///   cloning by applying a callback function.
///   - [`.with_untracked()`](#impl-SignalWithUntracked<T>-for-MaybeProp<T>) allows you to access the signal’s
///     value without reactively tracking it.
/// - [`.to_stream()`](#impl-SignalStream<T>-for-MaybeProp<T>) converts the signal to an `async` stream of values.
///
/// ## Examples
/// ```rust
/// # use leptos_reactive::*;
/// # let runtime = create_runtime();
/// let (count, set_count) = create_signal(Some(2));
/// let double = |n| n * 2;
/// let double_count = MaybeProp::derive(move || count.get().map(double));
/// let memoized_double_count = create_memo(move |_| count.get().map(double));
/// let static_value = 5;
///
/// // this function takes either a reactive or non-reactive value
/// fn above_3(arg: &MaybeProp<i32>) -> bool {
///     // ✅ calling the signal clones and returns the value
///     //    it is a shorthand for arg.get()q
///     arg.get().map(|arg| arg > 3).unwrap_or(false)
/// }
///
/// assert_eq!(above_3(&None::<i32>.into()), false);
/// assert_eq!(above_3(&static_value.into()), true);
/// assert_eq!(above_3(&count.into()), false);
/// assert_eq!(above_3(&double_count), true);
/// assert_eq!(above_3(&memoized_double_count.into()), true);
/// # runtime.dispose();
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MaybeProp<T: 'static>(pub(crate) Option<MaybeSignal<Option<T>>>);

impl<T: Copy> Copy for MaybeProp<T> {}

impl<T> Default for MaybeProp<T> {
    fn default() -> Self {
        Self(None)
    }
}

/// # Examples
///
/// ```
/// # use leptos_reactive::*;
/// # let runtime = create_runtime();
/// let (count, set_count) = create_signal(Some(2));
/// let double = |n| n * 2;
/// let double_count = MaybeProp::derive(move || count.get().map(double));
/// let memoized_double_count = create_memo(move |_| count.get().map(double));
/// let static_value = 5;
///
/// // this function takes either a reactive or non-reactive value
/// fn above_3(arg: &MaybeProp<i32>) -> bool {
///     // ✅ calling the signal clones and returns the value
///     //    it is a shorthand for arg.get()q
///     arg.get().map(|arg| arg > 3).unwrap_or(false)
/// }
///
/// assert_eq!(above_3(&None::<i32>.into()), false);
/// assert_eq!(above_3(&static_value.into()), true);
/// assert_eq!(above_3(&count.into()), false);
/// assert_eq!(above_3(&double_count), true);
/// assert_eq!(above_3(&memoized_double_count.into()), true);
/// # runtime.dispose();
/// ```
impl<T: Clone> SignalGet for MaybeProp<T> {
    type Value = Option<T>;

    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "MaybeProp::get()",
            skip_all,
            fields(ty = %std::any::type_name::<T>())
        )
    )]
    fn get(&self) -> Option<T> {
        self.0.as_ref().and_then(|s| s.get())
    }

    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "MaybeProp::try_get()",
            skip_all,
            fields(ty = %std::any::type_name::<T>())
        )
    )]
    fn try_get(&self) -> Option<Option<T>> {
        self.0.as_ref().and_then(|s| s.try_get())
    }
}

/// # Examples
///
/// ```
/// # use leptos_reactive::*;
/// # let runtime = create_runtime();
/// let (name, set_name) = create_signal("Alice".to_string());
/// let (maybe_name, set_maybe_name) = create_signal(None);
/// let name_upper =
///     MaybeProp::derive(move || Some(name.with(|n| n.to_uppercase())));
/// let memoized_lower = create_memo(move |_| name.with(|n| n.to_lowercase()));
/// let static_value: MaybeProp<String> = "Bob".to_string().into();
///
/// // this function takes any kind of wrapped signal
/// fn current_len_inefficient(arg: &MaybeProp<String>) -> usize {
///     // ❌ unnecessarily clones the string
///     arg.get().map(|n| n.len()).unwrap_or(0)
/// }
///
/// fn current_len(arg: &MaybeProp<String>) -> usize {
///     // ✅ gets the length without cloning the `String`
///     arg.with(|value| value.len()).unwrap_or(0)
/// }
///
/// assert_eq!(current_len(&None::<String>.into()), 0);
/// assert_eq!(current_len(&maybe_name.into()), 0);
/// assert_eq!(current_len(&name_upper), 5);
/// assert_eq!(current_len(&memoized_lower.into()), 5);
/// assert_eq!(current_len(&static_value), 3);
///
/// // Normal signals/memos return T
/// assert_eq!(name.get(), "Alice".to_string());
/// assert_eq!(memoized_lower.get(), "alice".to_string());
///
/// // MaybeProp::get() returns Option<T>
/// assert_eq!(name_upper.get(), Some("ALICE".to_string()));
/// assert_eq!(static_value.get(), Some("Bob".to_string()));
/// # runtime.dispose();
/// ```
impl<T> MaybeProp<T> {
    /// Applies a function to the current value, returning the result.
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "MaybeProp::with()",
            skip_all,
            fields(ty = %std::any::type_name::<T>())
        )
    )]
    pub fn with<O>(&self, f: impl FnOnce(&T) -> O) -> Option<O> {
        self.0
            .as_ref()
            .and_then(|inner| inner.with(|value| value.as_ref().map(f)))
    }

    /// Applies a function to the current value, returning the result. Returns `None`
    /// if the value has already been disposed.
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "MaybeProp::try_with()",
            skip_all,
            fields(ty = %std::any::type_name::<T>())
        )
    )]
    pub fn try_with<O>(&self, f: impl FnOnce(&T) -> O) -> Option<O> {
        self.0
            .as_ref()
            .and_then(|inner| inner.try_with(|value| value.as_ref().map(f)))
            .flatten()
    }

    /// Applies a function to the current value, returning the result, without
    /// causing the current reactive scope to track changes.
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "MaybeProp::with_untracked()",
            skip_all,
            fields(ty = %std::any::type_name::<T>())
        )
    )]
    pub fn with_untracked<O>(&self, f: impl FnOnce(&T) -> O) -> Option<O> {
        self.0.as_ref().and_then(|inner| {
            inner.with_untracked(|value| value.as_ref().map(f))
        })
    }

    /// Applies a function to the current value, returning the result, without
    /// causing the current reactive scope to track changes. Returns `None` if
    /// the value has already been disposed.
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "MaybeProp::try_with_untracked()",
            skip_all,
            fields(ty = %std::any::type_name::<T>())
        )
    )]
    pub fn try_with_untracked<O>(&self, f: impl FnOnce(&T) -> O) -> Option<O> {
        self.0
            .as_ref()
            .and_then(|inner| {
                inner.try_with_untracked(|value| value.as_ref().map(f))
            })
            .flatten()
    }
}

impl<T: Clone> SignalGetUntracked for MaybeProp<T> {
    type Value = Option<T>;

    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "MaybeProp::get_untracked()",
            skip_all,
            fields(ty = %std::any::type_name::<T>())
        )
    )]
    fn get_untracked(&self) -> Option<T> {
        self.0.as_ref().and_then(|inner| inner.get_untracked())
    }

    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "MaybeProp::try_get_untracked()",
            skip_all,
            fields(ty = %std::any::type_name::<T>())
        )
    )]
    fn try_get_untracked(&self) -> Option<Option<T>> {
        self.0.as_ref().and_then(|inner| inner.try_get_untracked())
    }
}

impl<T: Clone> SignalStream<Option<T>> for MaybeProp<T> {
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "MaybeProp::to_stream()",
            skip_all,
            fields(ty = %std::any::type_name::<T>())
        )
    )]
    fn to_stream(
        &self,
    ) -> std::pin::Pin<Box<dyn futures::Stream<Item = Option<T>>>> {
        match &self.0 {
            None => Box::pin(futures::stream::once(async move { None })),
            Some(MaybeSignal::Static(t)) => {
                let t = t.clone();

                let stream = futures::stream::once(async move { t });

                Box::pin(stream)
            }
            Some(MaybeSignal::Dynamic(s)) => s.to_stream(),
        }
    }
}

impl<T> MaybeProp<T>
where
    T: 'static,
{
    /// Wraps a derived signal, i.e., any computation that accesses one or more
    /// reactive signals.
    /// ```rust
    /// # use leptos_reactive::*;
    /// # let runtime = create_runtime();
    /// let (count, set_count) = create_signal(2);
    /// let double_count = MaybeProp::derive(move || Some(count.get() * 2));
    ///
    /// // this function takes any kind of wrapped signal
    /// fn above_3(arg: &MaybeProp<i32>) -> bool {
    ///     arg.get().unwrap_or(0) > 3
    /// }
    ///
    /// assert_eq!(above_3(&count.into()), false);
    /// assert_eq!(above_3(&double_count.into()), true);
    /// assert_eq!(above_3(&2.into()), false);
    /// # runtime.dispose();
    /// ```
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "MaybeProp::derive()",
            skip_all,
            fields(
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    pub fn derive(derived_signal: impl Fn() -> Option<T> + 'static) -> Self {
        Self(Some(MaybeSignal::derive(derived_signal)))
    }
}

impl<T> From<T> for MaybeProp<T> {
    fn from(value: T) -> Self {
        Self(Some(MaybeSignal::from(Some(value))))
    }
}

impl<T> From<Option<T>> for MaybeProp<T> {
    fn from(value: Option<T>) -> Self {
        Self(Some(MaybeSignal::from(value)))
    }
}

impl<T> From<MaybeSignal<Option<T>>> for MaybeProp<T> {
    fn from(value: MaybeSignal<Option<T>>) -> Self {
        Self(Some(value))
    }
}

impl<T> From<Option<MaybeSignal<Option<T>>>> for MaybeProp<T> {
    fn from(value: Option<MaybeSignal<Option<T>>>) -> Self {
        Self(value)
    }
}

impl<T> From<ReadSignal<Option<T>>> for MaybeProp<T> {
    fn from(value: ReadSignal<Option<T>>) -> Self {
        Self(Some(value.into()))
    }
}

impl<T> From<RwSignal<Option<T>>> for MaybeProp<T> {
    fn from(value: RwSignal<Option<T>>) -> Self {
        Self(Some(value.into()))
    }
}

impl<T> From<Memo<Option<T>>> for MaybeProp<T> {
    fn from(value: Memo<Option<T>>) -> Self {
        Self(Some(value.into()))
    }
}

impl<T> From<Signal<Option<T>>> for MaybeProp<T> {
    fn from(value: Signal<Option<T>>) -> Self {
        Self(Some(value.into()))
    }
}

impl<T: Clone> From<ReadSignal<T>> for MaybeProp<T> {
    fn from(value: ReadSignal<T>) -> Self {
        Self(Some(MaybeSignal::derive(move || Some(value.get()))))
    }
}

impl<T: Clone> From<RwSignal<T>> for MaybeProp<T> {
    fn from(value: RwSignal<T>) -> Self {
        Self(Some(MaybeSignal::derive(move || Some(value.get()))))
    }
}

impl<T: Clone> From<Memo<T>> for MaybeProp<T> {
    fn from(value: Memo<T>) -> Self {
        Self(Some(MaybeSignal::derive(move || Some(value.get()))))
    }
}

impl<T: Clone> From<Signal<T>> for MaybeProp<T> {
    fn from(value: Signal<T>) -> Self {
        Self(Some(MaybeSignal::derive(move || Some(value.get()))))
    }
}

impl From<&str> for MaybeProp<String> {
    fn from(value: &str) -> Self {
        Self(Some(MaybeSignal::from(Some(value.to_string()))))
    }
}

impl_get_fn_traits![Signal, MaybeSignal];

#[cfg(feature = "nightly")]
impl<T: Clone> FnOnce<()> for MaybeProp<T> {
    type Output = Option<T>;

    #[inline(always)]
    extern "rust-call" fn call_once(self, _args: ()) -> Self::Output {
        self.get()
    }
}

#[cfg(feature = "nightly")]
impl<T: Clone> FnMut<()> for MaybeProp<T> {
    #[inline(always)]
    extern "rust-call" fn call_mut(&mut self, _args: ()) -> Self::Output {
        self.get()
    }
}

#[cfg(feature = "nightly")]
impl<T: Clone> Fn<()> for MaybeProp<T> {
    #[inline(always)]
    extern "rust-call" fn call(&self, _args: ()) -> Self::Output {
        self.get()
    }
}

/// Describes a value that is either a static or a reactive string, i.e.,
/// a [`String`], a [`&str`], or a reactive `Fn() -> String`.
#[derive(Clone)]
pub struct TextProp(Rc<dyn Fn() -> Oco<'static, str>>);

impl TextProp {
    /// Accesses the current value of the property.
    #[inline(always)]
    pub fn get(&self) -> Oco<'static, str> {
        (self.0)()
    }
}

impl Debug for TextProp {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("TextProp").finish()
    }
}

impl From<String> for TextProp {
    fn from(s: String) -> Self {
        let s: Oco<'_, str> = Oco::Counted(Rc::from(s));
        TextProp(Rc::new(move || s.clone()))
    }
}

impl From<&'static str> for TextProp {
    fn from(s: &'static str) -> Self {
        let s: Oco<'_, str> = s.into();
        TextProp(Rc::new(move || s.clone()))
    }
}

impl From<Rc<str>> for TextProp {
    fn from(s: Rc<str>) -> Self {
        let s: Oco<'_, str> = s.into();
        TextProp(Rc::new(move || s.clone()))
    }
}

impl From<Oco<'static, str>> for TextProp {
    fn from(s: Oco<'static, str>) -> Self {
        TextProp(Rc::new(move || s.clone()))
    }
}

impl From<String> for MaybeProp<TextProp> {
    fn from(s: String) -> Self {
        Self(Some(MaybeSignal::from(Some(Oco::from(s).into()))))
    }
}

impl From<Rc<str>> for MaybeProp<TextProp> {
    fn from(s: Rc<str>) -> Self {
        Self(Some(MaybeSignal::from(Some(Oco::from(s).into()))))
    }
}

impl From<&'static str> for MaybeProp<TextProp> {
    fn from(s: &'static str) -> Self {
        Self(Some(MaybeSignal::from(Some(Oco::from(s).into()))))
    }
}

impl From<Box<str>> for MaybeProp<TextProp> {
    fn from(s: Box<str>) -> Self {
        Self(Some(MaybeSignal::from(Some(Oco::from(s).into()))))
    }
}

impl From<Cow<'static, str>> for MaybeProp<TextProp> {
    fn from(s: Cow<'static, str>) -> Self {
        Self(Some(MaybeSignal::from(Some(Oco::from(s).into()))))
    }
}

impl<F, S> From<F> for TextProp
where
    F: Fn() -> S + 'static,
    S: Into<Oco<'static, str>>,
{
    #[inline(always)]
    fn from(s: F) -> Self {
        TextProp(Rc::new(move || s().into()))
    }
}

impl Default for TextProp {
    fn default() -> Self {
        Self(Rc::new(|| Oco::Borrowed("")))
    }
}
