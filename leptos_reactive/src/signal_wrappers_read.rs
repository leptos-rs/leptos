#![forbid(unsafe_code)]
use crate::{
    create_effect, on_cleanup, store_value, Memo, ReadSignal, RwSignal, Scope,
    SignalGet, SignalGetUntracked, SignalStream, SignalWith,
    SignalWithUntracked, StoredValue,
};

/// Helper trait for converting `Fn() -> T` closures into
/// [`Signal<T>`].
pub trait IntoSignal<T>: Sized {
    /// Consumes `self`, returning a [`Signal<T>`].
    fn derive_signal(self, cx: Scope) -> Signal<T>;
}

impl<F, T> IntoSignal<T> for F
where
    F: Fn() -> T + 'static,
{
    fn derive_signal(self, cx: Scope) -> Signal<T> {
        Signal::derive(cx, self)
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
/// - [`.get()`](#impl-SignalGet<T>-for-Signal<T>) (or calling the signal as a function) clones the current
///   value of the signal. If you call it within an effect, it will cause that effect
///   to subscribe to the signal, and to re-run whenever the value of the signal changes.
///   - [`.get_untracked()`](#impl-SignalGetUntracked<T>-for-Signal<T>) clones the value of the signal
///   without reactively tracking it.
/// - [`.with()`](#impl-SignalWith<T>-for-Signal<T>) allows you to reactively access the signal’s value without
///   cloning by applying a callback function.
///   - [`.with_untracked()`](#impl-SignalWithUntracked<T>-for-Signal<T>) allows you to access the signal’s
///   value without reactively tracking it.
/// - [`.to_stream()`](#impl-SignalStream<T>-for-Signal<T>) converts the signal to an `async` stream of values.
///
/// ## Examples
/// ```rust
/// # use leptos_reactive::*;
/// # create_scope(create_runtime(), |cx| {
/// let (count, set_count) = create_signal(cx, 2);
/// let double_count = Signal::derive(cx, move || count.get() * 2);
/// let memoized_double_count = create_memo(cx, move |_| count.get() * 2);
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
/// # });
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

impl<T> std::fmt::Debug for Signal<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
impl<T: Clone> SignalGetUntracked<T> for Signal<T> {
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
            SignalTypes::DerivedSignal(cx, f) => {
                cx.untrack(|| f.with_value(|f| f()))
            }
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
            SignalTypes::DerivedSignal(cx, f) => {
                cx.untrack(|| f.try_with_value(|f| f()))
            }
        }
    }
}

impl<T> SignalWithUntracked<T> for Signal<T> {
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
            SignalTypes::DerivedSignal(cx, v_f) => {
                let mut o = None;

                cx.untrack(|| o = Some(f(&v_f.with_value(|v_f| v_f()))));

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
            SignalTypes::DerivedSignal(_, s) => s.try_with_value(|t| f(&t())),
        }
    }
}

/// # Examples
///
/// ```
/// # use leptos_reactive::*;
/// # create_scope(create_runtime(), |cx| {
/// let (name, set_name) = create_signal(cx, "Alice".to_string());
/// let name_upper =
///     Signal::derive(cx, move || name.with(|n| n.to_uppercase()));
/// let memoized_lower =
///     create_memo(cx, move |_| name.with(|n| n.to_lowercase()));
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
/// # });
/// ```
impl<T> SignalWith<T> for Signal<T> {
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
            SignalTypes::DerivedSignal(_, s) => f(&s.with_value(|s| s())),
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
            SignalTypes::DerivedSignal(_, s) => s.try_with_value(|t| f(&t())),
        }
    }
}

/// # Examples
///
/// ```
/// # use leptos_reactive::*;
/// # create_scope(create_runtime(), |cx| {
/// let (count, set_count) = create_signal(cx, 2);
/// let double_count = Signal::derive(cx, move || count.get() * 2);
/// let memoized_double_count = create_memo(cx, move |_| count.get() * 2);
///
/// // this function takes any kind of wrapped signal
/// fn above_3(arg: &Signal<i32>) -> bool {
///     arg.get() > 3
/// }
///
/// assert_eq!(above_3(&count.into()), false);
/// assert_eq!(above_3(&double_count), true);
/// assert_eq!(above_3(&memoized_double_count.into()), true);
/// # });
/// ```
impl<T: Clone> SignalGet<T> for Signal<T> {
    fn get(&self) -> T {
        match self.inner {
            SignalTypes::ReadSignal(r) => r.get(),
            SignalTypes::Memo(m) => m.get(),
            SignalTypes::DerivedSignal(_, s) => s.with_value(|t| t()),
        }
    }

    fn try_get(&self) -> Option<T> {
        match self.inner {
            SignalTypes::ReadSignal(r) => r.try_get(),
            SignalTypes::Memo(m) => m.try_get(),
            SignalTypes::DerivedSignal(_, s) => s.try_with_value(|t| t()),
        }
    }
}

impl<T: Clone> SignalStream<T> for Signal<T> {
    fn to_stream(
        &self,
        cx: Scope,
    ) -> std::pin::Pin<Box<dyn futures::Stream<Item = T>>> {
        match self.inner {
            SignalTypes::ReadSignal(r) => r.to_stream(cx),
            SignalTypes::Memo(m) => m.to_stream(cx),
            SignalTypes::DerivedSignal(_, s) => {
                let (tx, rx) = futures::channel::mpsc::unbounded();

                let close_channel = tx.clone();

                on_cleanup(cx, move || close_channel.close_channel());

                create_effect(cx, move |_| {
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
    /// # create_scope(create_runtime(), |cx| {
    /// let (count, set_count) = create_signal(cx, 2);
    /// let double_count = Signal::derive(cx, move || count.get() * 2);
    ///
    /// // this function takes any kind of wrapped signal
    /// fn above_3(arg: &Signal<i32>) -> bool {
    ///     arg.get() > 3
    /// }
    ///
    /// assert_eq!(above_3(&count.into()), false);
    /// assert_eq!(above_3(&double_count), true);
    /// # });
    /// ```
    #[track_caller]
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            skip_all,
            fields(
                cx = ?cx.id
            )
        )
    )]
    pub fn derive(cx: Scope, derived_signal: impl Fn() -> T + 'static) -> Self {
        let span = ::tracing::Span::current();

        let derived_signal = move || {
            let _guard = span.enter();
            derived_signal()
        };

        Self {
            inner: SignalTypes::DerivedSignal(
                cx,
                store_value(cx, Box::new(derived_signal)),
            ),
            #[cfg(any(debug_assertions, feature = "ssr"))]
            defined_at: std::panic::Location::caller(),
        }
    }

    /// Creates a signal that yields the default value of `T` when
    /// you call `.get()` or `signal()`.
    pub fn default(cx: Scope) -> Self
    where
        T: Default,
    {
        Self::derive(cx, || Default::default())
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
    DerivedSignal(Scope, StoredValue<Box<dyn Fn() -> T>>),
}

impl<T> Clone for SignalTypes<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for SignalTypes<T> {}

impl<T> std::fmt::Debug for SignalTypes<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReadSignal(arg0) => {
                f.debug_tuple("ReadSignal").field(arg0).finish()
            }
            Self::Memo(arg0) => f.debug_tuple("Memo").field(arg0).finish(),
            Self::DerivedSignal(_, _) => {
                f.debug_tuple("DerivedSignal").finish()
            }
        }
    }
}

impl<T> PartialEq for SignalTypes<T> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::ReadSignal(l0), Self::ReadSignal(r0)) => l0 == r0,
            (Self::Memo(l0), Self::Memo(r0)) => l0 == r0,
            (Self::DerivedSignal(_, l0), Self::DerivedSignal(_, r0)) => {
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
/// - [`.get()`](#impl-SignalGet<T>-for-MaybeSignal<T>) (or calling the signal as a function) clones the current
///   value of the signal. If you call it within an effect, it will cause that effect
///   to subscribe to the signal, and to re-run whenever the value of the signal changes.
///   - [`.get_untracked()`](#impl-SignalGetUntracked<T>-for-MaybeSignal<T>) clones the value of the signal
///   without reactively tracking it.
/// - [`.with()`](#impl-SignalWith<T>-for-MaybeSignal<T>) allows you to reactively access the signal’s value without
///   cloning by applying a callback function.
///   - [`.with_untracked()`](#impl-SignalWithUntracked<T>-for-MaybeSignal<T>) allows you to access the signal’s
///   value without reactively tracking it.
/// - [`.to_stream()`](#impl-SignalStream<T>-for-MaybeSignal<T>) converts the signal to an `async` stream of values.
///
/// ## Examples
/// ```rust
/// # use leptos_reactive::*;
/// # create_scope(create_runtime(), |cx| {
/// let (count, set_count) = create_signal(cx, 2);
/// let double_count = MaybeSignal::derive(cx, move || count.get() * 2);
/// let memoized_double_count = create_memo(cx, move |_| count.get() * 2);
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
/// # });
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
/// # create_scope(create_runtime(), |cx| {
/// let (count, set_count) = create_signal(cx, 2);
/// let double_count = MaybeSignal::derive(cx, move || count.get() * 2);
/// let memoized_double_count = create_memo(cx, move |_| count.get() * 2);
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
/// # });
/// ```
impl<T: Clone> SignalGet<T> for MaybeSignal<T> {
    fn get(&self) -> T {
        match self {
            Self::Static(t) => t.clone(),
            Self::Dynamic(s) => s.get(),
        }
    }

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
/// # create_scope(create_runtime(), |cx| {
/// let (name, set_name) = create_signal(cx, "Alice".to_string());
/// let name_upper =
///     MaybeSignal::derive(cx, move || name.with(|n| n.to_uppercase()));
/// let memoized_lower =
///     create_memo(cx, move |_| name.with(|n| n.to_lowercase()));
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
/// # });
/// ```
impl<T> SignalWith<T> for MaybeSignal<T> {
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

impl<T> SignalWithUntracked<T> for MaybeSignal<T> {
    fn with_untracked<O>(&self, f: impl FnOnce(&T) -> O) -> O {
        match self {
            Self::Static(t) => f(t),
            Self::Dynamic(s) => s.with_untracked(f),
        }
    }

    fn try_with_untracked<O>(&self, f: impl FnOnce(&T) -> O) -> Option<O> {
        match self {
            Self::Static(t) => Some(f(t)),
            Self::Dynamic(s) => s.try_with_untracked(f),
        }
    }
}

impl<T: Clone> SignalGetUntracked<T> for MaybeSignal<T> {
    fn get_untracked(&self) -> T {
        match self {
            Self::Static(t) => t.clone(),
            Self::Dynamic(s) => s.get_untracked(),
        }
    }

    fn try_get_untracked(&self) -> Option<T> {
        match self {
            Self::Static(t) => Some(t.clone()),
            Self::Dynamic(s) => s.try_get_untracked(),
        }
    }
}

impl<T: Clone> SignalStream<T> for MaybeSignal<T> {
    fn to_stream(
        &self,
        cx: Scope,
    ) -> std::pin::Pin<Box<dyn futures::Stream<Item = T>>> {
        match self {
            Self::Static(t) => {
                let t = t.clone();

                let stream = futures::stream::once(async move { t });

                Box::pin(stream)
            }
            Self::Dynamic(s) => s.to_stream(cx),
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
    /// # create_scope(create_runtime(), |cx| {
    /// let (count, set_count) = create_signal(cx, 2);
    /// let double_count = Signal::derive(cx, move || count.get() * 2);
    ///
    /// // this function takes any kind of wrapped signal
    /// fn above_3(arg: &MaybeSignal<i32>) -> bool {
    ///     arg.get() > 3
    /// }
    ///
    /// assert_eq!(above_3(&count.into()), false);
    /// assert_eq!(above_3(&double_count.into()), true);
    /// assert_eq!(above_3(&2.into()), false);
    /// # });
    /// ```
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            name = "MaybeSignal::derive()",
            skip_all,
            fields(
                cx = ?cx.id,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    pub fn derive(cx: Scope, derived_signal: impl Fn() -> T + 'static) -> Self {
        Self::Dynamic(Signal::derive(cx, derived_signal))
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

impl_get_fn_traits![Signal, MaybeSignal];
