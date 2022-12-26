use std::rc::Rc;

use crate::{Memo, ReadSignal, RwSignal, Scope, UntrackedGettableSignal};

/// A wrapper for any kind of readable reactive signal: a [ReadSignal](crate::ReadSignal),
/// [Memo](crate::Memo), [RwSignal](crate::RwSignal), or derived signal closure.
///
/// This allows you to create APIs that take any kind of `Signal<T>` as an argument,
/// rather than adding a generic `F: Fn() -> T`. Values can be access with the same
/// function call, `with()`, and `get()` APIs as other signals.
///
/// ```rust
/// # use leptos_reactive::*;
/// # create_scope(create_runtime(), |cx| {
/// let (count, set_count) = create_signal(cx, 2);
/// let double_count = Signal::derive(cx, move || count() * 2);
/// let memoized_double_count = create_memo(cx, move |_| count() * 2);
///
/// // this function takes any kind of wrapped signal
/// fn above_3(arg: &Signal<i32>) -> bool {
///   // ✅ calling the signal clones and returns the value
///   //    it is a shorthand for arg.get()
///   arg() > 3
/// }
///
/// assert_eq!(above_3(&count.into()), false);
/// assert_eq!(above_3(&double_count), true);
/// assert_eq!(above_3(&memoized_double_count.into()), true);
/// # });
/// ```
#[derive(Debug, PartialEq, Eq)]
pub struct Signal<T>(SignalTypes<T>)
where
    T: 'static;

impl<T> Clone for Signal<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

/// Please note that using `Signal::with_untracked` still clones the inner value,
/// so there's no benefit to using it as opposed to calling
/// `Signal::get_untracked`.
impl<T> UntrackedGettableSignal<T> for Signal<T>
where
    T: 'static,
{
    fn get_untracked(&self) -> T
    where
        T: Clone,
    {
        match &self.0 {
            SignalTypes::ReadSignal(s) => s.get_untracked(),
            SignalTypes::Memo(m) => m.get_untracked(),
            SignalTypes::DerivedSignal(cx, f) => cx.untrack(|| f()),
        }
    }

    fn with_untracked<O>(&self, f: impl FnOnce(&T) -> O) -> O {
        match &self.0 {
            SignalTypes::ReadSignal(s) => s.with_untracked(f),
            SignalTypes::Memo(s) => s.with_untracked(f),
            SignalTypes::DerivedSignal(cx, v_f) => {
                let mut o = None;

                cx.untrack(|| o = Some(f(&v_f())));

                o.unwrap()
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
    /// let double_count = Signal::derive(cx, move || count() * 2);
    ///
    /// // this function takes any kind of wrapped signal
    /// fn above_3(arg: &Signal<i32>) -> bool {
    ///   arg() > 3
    /// }
    ///
    /// assert_eq!(above_3(&count.into()), false);
    /// assert_eq!(above_3(&double_count), true);
    /// # });
    /// ```
    pub fn derive(cx: Scope, derived_signal: impl Fn() -> T + 'static) -> Self {
        Self(SignalTypes::DerivedSignal(cx, Rc::new(derived_signal)))
    }

    /// Applies a function to the current value of the signal, and subscribes
    /// the running effect to this signal.
    /// ```
    /// # use leptos_reactive::*;
    /// # create_scope(create_runtime(), |cx| {
    /// let (name, set_name) = create_signal(cx, "Alice".to_string());
    /// let name_upper = Signal::derive(cx, move || name.with(|n| n.to_uppercase()));
    /// let memoized_lower = create_memo(cx, move |_| name.with(|n| n.to_lowercase()));
    ///
    /// // this function takes any kind of wrapped signal
    /// fn current_len_inefficient(arg: Signal<String>) -> usize {
    ///  // ❌ unnecessarily clones the string
    ///   arg().len()
    /// }
    ///
    /// fn current_len(arg: &Signal<String>) -> usize {
    ///  // ✅ gets the length without cloning the `String`
    ///   arg.with(|value| value.len())
    /// }
    ///
    /// assert_eq!(current_len(&name.into()), 5);
    /// assert_eq!(current_len(&name_upper), 5);
    /// assert_eq!(current_len(&memoized_lower.into()), 5);
    ///
    /// assert_eq!(name(), "Alice");
    /// assert_eq!(name_upper(), "ALICE");
    /// assert_eq!(memoized_lower(), "alice");
    /// });
    /// ```
    pub fn with<U>(&self, f: impl FnOnce(&T) -> U) -> U {
        match &self.0 {
            SignalTypes::ReadSignal(s) => s.with(f),
            SignalTypes::Memo(s) => s.with(f),
            SignalTypes::DerivedSignal(_, s) => f(&s()),
        }
    }

    /// Clones and returns the current value of the signal, and subscribes
    /// the running effect to this signal.
    ///
    /// If you want to get the value without cloning it, use [ReadSignal::with].
    /// (There’s no difference in behavior for derived signals: they re-run in any case.)
    /// ```
    /// # use leptos_reactive::*;
    /// # create_scope(create_runtime(), |cx| {
    /// let (count, set_count) = create_signal(cx, 2);
    /// let double_count = Signal::derive(cx, move || count() * 2);
    /// let memoized_double_count = create_memo(cx, move |_| count() * 2);
    ///
    /// // this function takes any kind of wrapped signal
    /// fn above_3(arg: &Signal<i32>) -> bool {
    ///   arg.get() > 3
    /// }
    ///
    /// assert_eq!(above_3(&count.into()), false);
    /// assert_eq!(above_3(&double_count), true);
    /// assert_eq!(above_3(&memoized_double_count.into()), true);
    /// # });
    /// ```
    pub fn get(&self) -> T
    where
        T: Clone,
    {
        match &self.0 {
            SignalTypes::ReadSignal(s) => s.get(),
            SignalTypes::Memo(s) => s.get(),
            SignalTypes::DerivedSignal(_, s) => s(),
        }
    }
}

impl<T> From<ReadSignal<T>> for Signal<T> {
    fn from(value: ReadSignal<T>) -> Self {
        Self(SignalTypes::ReadSignal(value))
    }
}

impl<T> From<RwSignal<T>> for Signal<T> {
    fn from(value: RwSignal<T>) -> Self {
        Self(SignalTypes::ReadSignal(value.read_only()))
    }
}

impl<T> From<Memo<T>> for Signal<T> {
    fn from(value: Memo<T>) -> Self {
        Self(SignalTypes::Memo(value))
    }
}

enum SignalTypes<T>
where
    T: 'static,
{
    ReadSignal(ReadSignal<T>),
    Memo(Memo<T>),
    DerivedSignal(Scope, Rc<dyn Fn() -> T>),
}

impl<T> Clone for SignalTypes<T> {
    fn clone(&self) -> Self {
        match self {
            Self::ReadSignal(arg0) => Self::ReadSignal(*arg0),
            Self::Memo(arg0) => Self::Memo(*arg0),
            Self::DerivedSignal(arg0, arg1) => Self::DerivedSignal(*arg0, Rc::clone(arg1)),
        }
    }
}

impl<T> std::fmt::Debug for SignalTypes<T>
where
    T: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReadSignal(arg0) => f.debug_tuple("ReadSignal").field(arg0).finish(),
            Self::Memo(arg0) => f.debug_tuple("Memo").field(arg0).finish(),
            Self::DerivedSignal(_, _) => f.debug_tuple("DerivedSignal").finish(),
        }
    }
}

impl<T> PartialEq for SignalTypes<T>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::ReadSignal(l0), Self::ReadSignal(r0)) => l0 == r0,
            (Self::Memo(l0), Self::Memo(r0)) => l0 == r0,
            (Self::DerivedSignal(_, l0), Self::DerivedSignal(_, r0)) => std::ptr::eq(l0, r0),
            _ => false,
        }
    }
}

impl<T> Eq for SignalTypes<T> where T: PartialEq {}

#[cfg(not(feature = "stable"))]
impl<T> FnOnce<()> for Signal<T>
where
    T: Clone,
{
    type Output = T;

    extern "rust-call" fn call_once(self, _args: ()) -> Self::Output {
        self.get()
    }
}

#[cfg(not(feature = "stable"))]
impl<T> FnMut<()> for Signal<T>
where
    T: Clone,
{
    extern "rust-call" fn call_mut(&mut self, _args: ()) -> Self::Output {
        self.get()
    }
}

#[cfg(not(feature = "stable"))]
impl<T> Fn<()> for Signal<T>
where
    T: Clone,
{
    extern "rust-call" fn call(&self, _args: ()) -> Self::Output {
        self.get()
    }
}

/// A wrapper for a value that is *either* `T` or [`Signal<T>`](crate::Signal).
///
/// This allows you to create APIs that take either a reactive or a non-reactive value
/// of the same type. This is especially useful for component properties.
///
/// ```rust
/// # use leptos_reactive::*;
/// # create_scope(create_runtime(), |cx| {
/// let (count, set_count) = create_signal(cx, 2);
/// let double_count = MaybeSignal::derive(cx, move || count() * 2);
/// let memoized_double_count = create_memo(cx, move |_| count() * 2);
/// let static_value = 5;
///
/// // this function takes either a reactive or non-reactive value
/// fn above_3(arg: &MaybeSignal<i32>) -> bool {
///   // ✅ calling the signal clones and returns the value
///   //    it is a shorthand for arg.get()
///   arg() > 3
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

impl<T> UntrackedGettableSignal<T> for MaybeSignal<T>
where
    T: 'static,
{
    fn get_untracked(&self) -> T
    where
        T: Clone,
    {
        match self {
            Self::Static(t) => t.clone(),
            Self::Dynamic(s) => s.get_untracked(),
        }
    }

    fn with_untracked<O>(&self, f: impl FnOnce(&T) -> O) -> O {
        match self {
            Self::Static(t) => f(t),
            Self::Dynamic(s) => s.with_untracked(f),
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
    /// let double_count = Signal::derive(cx, move || count() * 2);
    ///
    /// // this function takes any kind of wrapped signal
    /// fn above_3(arg: &Signal<i32>) -> bool {
    ///   arg() > 3
    /// }
    ///
    /// assert_eq!(above_3(&count.into()), false);
    /// assert_eq!(above_3(&double_count), true);
    /// # });
    /// ```
    pub fn derive(cx: Scope, derived_signal: impl Fn() -> T + 'static) -> Self {
        Self::Dynamic(Signal::derive(cx, derived_signal))
    }

    /// Applies a function to the current value of the signal, and subscribes
    /// the running effect to this signal.
    /// ```
    /// # use leptos_reactive::*;
    /// # create_scope(create_runtime(), |cx| {
    /// let (name, set_name) = create_signal(cx, "Alice".to_string());
    /// let name_upper = MaybeSignal::derive(cx, move || name.with(|n| n.to_uppercase()));
    /// let memoized_lower = create_memo(cx, move |_| name.with(|n| n.to_lowercase()));
    /// let static_value: MaybeSignal<String> = "Bob".to_string().into();
    ///
    /// // this function takes any kind of wrapped signal
    /// fn current_len_inefficient(arg: &MaybeSignal<String>) -> usize {
    ///  // ❌ unnecessarily clones the string
    ///   arg().len()
    /// }
    ///
    /// fn current_len(arg: &MaybeSignal<String>) -> usize {
    ///  // ✅ gets the length without cloning the `String`
    ///   arg.with(|value| value.len())
    /// }
    ///
    /// assert_eq!(current_len(&name.into()), 5);
    /// assert_eq!(current_len(&name_upper), 5);
    /// assert_eq!(current_len(&memoized_lower.into()), 5);
    /// assert_eq!(current_len(&static_value), 3);
    ///
    /// assert_eq!(name(), "Alice");
    /// assert_eq!(name_upper(), "ALICE");
    /// assert_eq!(memoized_lower(), "alice");
    /// assert_eq!(static_value(), "Bob");
    /// });
    /// ```
    pub fn with<U>(&self, f: impl FnOnce(&T) -> U) -> U {
        match &self {
            Self::Static(value) => f(value),
            Self::Dynamic(signal) => signal.with(f),
        }
    }

    /// Clones and returns the current value of the signal, and subscribes
    /// the running effect to this signal.
    ///
    /// If you want to get the value without cloning it, use [ReadSignal::with].
    /// (There’s no difference in behavior for derived signals: they re-run in any case.)
    /// ```
    /// # use leptos_reactive::*;
    /// # create_scope(create_runtime(), |cx| {
    /// let (count, set_count) = create_signal(cx, 2);
    /// let double_count = MaybeSignal::derive(cx, move || count() * 2);
    /// let memoized_double_count = create_memo(cx, move |_| count() * 2);
    /// let static_value: MaybeSignal<i32> = 5.into();
    ///
    /// // this function takes any kind of wrapped signal
    /// fn above_3(arg: &MaybeSignal<i32>) -> bool {
    ///   arg.get() > 3
    /// }
    ///
    /// assert_eq!(above_3(&count.into()), false);
    /// assert_eq!(above_3(&double_count), true);
    /// assert_eq!(above_3(&memoized_double_count.into()), true);
    /// assert_eq!(above_3(&static_value.into()), true);
    /// # });
    /// ```
    pub fn get(&self) -> T
    where
        T: Clone,
    {
        match &self {
            Self::Static(value) => value.clone(),
            Self::Dynamic(signal) => signal.get(),
        }
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

#[cfg(not(feature = "stable"))]
impl<T> FnOnce<()> for MaybeSignal<T>
where
    T: Clone,
{
    type Output = T;

    extern "rust-call" fn call_once(self, _args: ()) -> Self::Output {
        self.get()
    }
}

#[cfg(not(feature = "stable"))]
impl<T> FnMut<()> for MaybeSignal<T>
where
    T: Clone,
{
    extern "rust-call" fn call_mut(&mut self, _args: ()) -> Self::Output {
        self.get()
    }
}

#[cfg(not(feature = "stable"))]
impl<T> Fn<()> for MaybeSignal<T>
where
    T: Clone,
{
    extern "rust-call" fn call(&self, _args: ()) -> Self::Output {
        self.get()
    }
}
