//! Types to abstract over different kinds of readable or writable reactive values.

/// Types that abstract over signals with values that can be read.
pub mod read {
    use crate::{
        computed::{ArcMemo, Memo},
        owner::{FromLocal, LocalStorage, Storage, StoredValue, SyncStorage},
        signal::{ArcReadSignal, ArcRwSignal, ReadSignal, RwSignal},
        traits::{DefinedAt, Dispose, Get, With, WithUntracked},
        untrack, unwrap_signal,
    };
    use send_wrapper::SendWrapper;
    use std::{panic::Location, sync::Arc};

    /// Possibilities for the inner type of a [`Signal`].
    pub enum SignalTypes<T> {
        /// A readable signal.
        ReadSignal(ArcReadSignal<T>),
        /// A memo.
        Memo(ArcMemo<T>),
        /// A derived signal.
        DerivedSignal(Arc<dyn Fn() -> T + Send + Sync>),
    }

    impl<T> Clone for SignalTypes<T> {
        fn clone(&self) -> Self {
            match self {
                Self::ReadSignal(arg0) => Self::ReadSignal(arg0.clone()),
                Self::Memo(arg0) => Self::Memo(arg0.clone()),
                Self::DerivedSignal(arg0) => Self::DerivedSignal(arg0.clone()),
            }
        }
    }

    impl<T> core::fmt::Debug for SignalTypes<T> {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match self {
                Self::ReadSignal(arg0) => {
                    f.debug_tuple("ReadSignal").field(arg0).finish()
                }
                Self::Memo(arg0) => f.debug_tuple("Memo").field(arg0).finish(),
                Self::DerivedSignal(_) => {
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
                (Self::DerivedSignal(l0), Self::DerivedSignal(r0)) => {
                    std::ptr::eq(l0, r0)
                }
                _ => false,
            }
        }
    }

    /// A wrapper for any kind of reference-counted reactive signal:
    /// an [`ArcReadSignal`], [`ArcMemo`], [`ArcRwSignal`],
    /// or derived signal closure.
    ///
    /// This allows you to create APIs that take any kind of `ArcSignal<T>` as an argument,
    /// rather than adding a generic `F: Fn() -> T`. Values can be access with the same
    /// function call, `with()`, and `get()` APIs as other signals.
    pub struct ArcSignal<T: 'static> {
        #[cfg(debug_assertions)]
        defined_at: &'static Location<'static>,
        inner: SignalTypes<T>,
    }

    impl<T> Clone for ArcSignal<T> {
        fn clone(&self) -> Self {
            Self {
                #[cfg(debug_assertions)]
                defined_at: self.defined_at,
                inner: self.inner.clone(),
            }
        }
    }

    impl<T> core::fmt::Debug for ArcSignal<T> {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            let mut s = f.debug_struct("ArcSignal");
            s.field("inner", &self.inner);
            #[cfg(debug_assertions)]
            s.field("defined_at", &self.defined_at);
            s.finish()
        }
    }

    impl<T> Eq for ArcSignal<T> {}

    impl<T> PartialEq for ArcSignal<T> {
        fn eq(&self, other: &Self) -> bool {
            self.inner == other.inner
        }
    }

    impl<T> ArcSignal<T>
    where
        T: Send + Sync + 'static,
    {
        /// Wraps a derived signal, i.e., any computation that accesses one or more
        /// reactive signals.
        /// ```rust
        /// # use reactive_graph::signal::*;
        /// # use reactive_graph::wrappers::read::ArcSignal;
        /// # use reactive_graph::prelude::*;
        /// let (count, set_count) = arc_signal(2);
        /// let double_count = ArcSignal::derive({
        ///     let count = count.clone();
        ///     move || count.get() * 2
        /// });
        ///
        /// // this function takes any kind of wrapped signal
        /// fn above_3(arg: &ArcSignal<i32>) -> bool {
        ///     arg.get() > 3
        /// }
        ///
        /// assert_eq!(above_3(&count.into()), false);
        /// assert_eq!(above_3(&double_count), true);
        /// ```
        #[track_caller]
        pub fn derive(
            derived_signal: impl Fn() -> T + Send + Sync + 'static,
        ) -> Self {
            #[cfg(feature = "tracing")]
            let span = ::tracing::Span::current();

            let derived_signal = move || {
                #[cfg(feature = "tracing")]
                let _guard = span.enter();
                derived_signal()
            };

            Self {
                inner: SignalTypes::DerivedSignal(Arc::new(derived_signal)),
                #[cfg(debug_assertions)]
                defined_at: std::panic::Location::caller(),
            }
        }
    }

    impl<T> Default for ArcSignal<T>
    where
        T: Default + Send + Sync + 'static,
    {
        fn default() -> Self {
            Self::derive(|| Default::default())
        }
    }

    impl<T: Send + Sync> From<ArcReadSignal<T>> for ArcSignal<T> {
        #[track_caller]
        fn from(value: ArcReadSignal<T>) -> Self {
            Self {
                inner: SignalTypes::ReadSignal(value),
                #[cfg(debug_assertions)]
                defined_at: std::panic::Location::caller(),
            }
        }
    }

    impl<T: Send + Sync> From<ArcRwSignal<T>> for ArcSignal<T> {
        #[track_caller]
        fn from(value: ArcRwSignal<T>) -> Self {
            Self {
                inner: SignalTypes::ReadSignal(value.read_only()),
                #[cfg(debug_assertions)]
                defined_at: std::panic::Location::caller(),
            }
        }
    }

    impl<T: Send + Sync> From<ArcMemo<T>> for ArcSignal<T> {
        #[track_caller]
        fn from(value: ArcMemo<T>) -> Self {
            Self {
                inner: SignalTypes::Memo(value),
                #[cfg(debug_assertions)]
                defined_at: std::panic::Location::caller(),
            }
        }
    }

    impl<T> DefinedAt for ArcSignal<T> {
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

    impl<T> WithUntracked for ArcSignal<T>
    where
        T: Send + Sync,
    {
        type Value = T;

        fn try_with_untracked<U>(
            &self,
            fun: impl FnOnce(&Self::Value) -> U,
        ) -> Option<U> {
            match &self.inner {
                SignalTypes::ReadSignal(i) => i.try_with_untracked(fun),
                SignalTypes::Memo(i) => i.try_with_untracked(fun),
                SignalTypes::DerivedSignal(i) => Some(untrack(|| fun(&i()))),
            }
        }
    }

    impl<T> With for ArcSignal<T>
    where
        T: Send + Sync + Clone,
    {
        type Value = T;

        fn try_with<U>(
            &self,
            fun: impl FnOnce(&Self::Value) -> U,
        ) -> Option<U> {
            match &self.inner {
                SignalTypes::ReadSignal(i) => i.try_with(fun),
                SignalTypes::Memo(i) => i.try_with(fun),
                SignalTypes::DerivedSignal(i) => Some(fun(&i())),
            }
        }
    }

    /// A wrapper for any kind of arena-allocated reactive signal:
    /// an [`ReadSignal`], [`Memo`], [`RwSignal`], or derived signal closure.
    ///
    /// This allows you to create APIs that take any kind of `Signal<T>` as an argument,
    /// rather than adding a generic `F: Fn() -> T`. Values can be access with the same
    /// function call, `with()`, and `get()` APIs as other signals.
    pub struct Signal<T, S = SyncStorage> {
        #[cfg(debug_assertions)]
        defined_at: &'static Location<'static>,
        inner: StoredValue<SignalTypes<T>, S>,
    }

    impl<T, S> Dispose for Signal<T, S> {
        fn dispose(self) {
            self.inner.dispose()
        }
    }

    impl<T, S> Clone for Signal<T, S> {
        fn clone(&self) -> Self {
            *self
        }
    }

    impl<T, S> Copy for Signal<T, S> {}

    impl<T, S> core::fmt::Debug for Signal<T, S>
    where
        S: std::fmt::Debug,
    {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            let mut s = f.debug_struct("Signal");
            s.field("inner", &self.inner);
            #[cfg(debug_assertions)]
            s.field("defined_at", &self.defined_at);
            s.finish()
        }
    }

    impl<T, S> Eq for Signal<T, S> {}

    impl<T, S> PartialEq for Signal<T, S> {
        fn eq(&self, other: &Self) -> bool {
            self.inner == other.inner
        }
    }

    impl<T, S> DefinedAt for Signal<T, S> {
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

    impl<T, S> WithUntracked for Signal<T, S>
    where
        T: 'static,
        S: Storage<SignalTypes<T>>,
    {
        type Value = T;

        fn try_with_untracked<U>(
            &self,
            fun: impl FnOnce(&Self::Value) -> U,
        ) -> Option<U> {
            self.inner
                // clone the inner Arc type and release the lock
                // prevents deadlocking if the derived value includes taking a lock on the arena
                .try_with_value(Clone::clone)
                .and_then(|inner| match &inner {
                    SignalTypes::ReadSignal(i) => i.try_with_untracked(fun),
                    SignalTypes::Memo(i) => i.try_with_untracked(fun),
                    SignalTypes::DerivedSignal(i) => {
                        Some(untrack(|| fun(&i())))
                    }
                })
        }
    }

    impl<T, S> With for Signal<T, S>
    where
        T: Send + Sync + 'static,
        S: Storage<SignalTypes<T>>,
    {
        type Value = T;

        fn try_with<U>(
            &self,
            fun: impl FnOnce(&Self::Value) -> U,
        ) -> Option<U> {
            self.inner
                // clone the inner Arc type and release the lock
                // prevents deadlocking if the derived value includes taking a lock on the arena
                .try_with_value(Clone::clone)
                .and_then(|inner| match &inner {
                    SignalTypes::ReadSignal(i) => i.try_with(fun),
                    SignalTypes::Memo(i) => i.try_with(fun),
                    SignalTypes::DerivedSignal(i) => Some(fun(&i())),
                })
        }
    }

    impl<T> Signal<T>
    where
        T: Send + Sync + 'static,
    {
        /// Wraps a derived signal, i.e., any computation that accesses one or more
        /// reactive signals.
        /// ```rust
        /// # use reactive_graph::signal::*;
        /// # use reactive_graph::wrappers::read::Signal;
        /// # use reactive_graph::prelude::*;
        /// let (count, set_count) = signal(2);
        /// let double_count = Signal::derive(move || count.get() * 2);
        ///
        /// // this function takes any kind of wrapped signal
        /// fn above_3(arg: &Signal<i32>) -> bool {
        ///     arg.get() > 3
        /// }
        ///
        /// assert_eq!(above_3(&count.into()), false);
        /// assert_eq!(above_3(&double_count), true);
        /// ```
        #[track_caller]
        pub fn derive(
            derived_signal: impl Fn() -> T + Send + Sync + 'static,
        ) -> Self {
            #[cfg(feature = "tracing")]
            let span = ::tracing::Span::current();

            let derived_signal = move || {
                #[cfg(feature = "tracing")]
                let _guard = span.enter();
                derived_signal()
            };

            Self {
                inner: StoredValue::new_with_storage(
                    SignalTypes::DerivedSignal(Arc::new(derived_signal)),
                ),
                #[cfg(debug_assertions)]
                defined_at: std::panic::Location::caller(),
            }
        }
    }

    impl<T> Signal<T, LocalStorage>
    where
        T: 'static,
    {
        /// Wraps a derived signal. Works like [`Signal::derive`] but uses [`LocalStorage`].
        #[track_caller]
        pub fn derive_local(derived_signal: impl Fn() -> T + 'static) -> Self {
            let derived_signal = SendWrapper::new(derived_signal);
            #[cfg(feature = "tracing")]
            let span = ::tracing::Span::current();

            let derived_signal = move || {
                #[cfg(feature = "tracing")]
                let _guard = span.enter();
                derived_signal()
            };

            Self {
                inner: StoredValue::new_local(SignalTypes::DerivedSignal(
                    Arc::new(derived_signal),
                )),
                #[cfg(debug_assertions)]
                defined_at: std::panic::Location::caller(),
            }
        }
    }

    impl<T> Default for Signal<T>
    where
        T: Send + Sync + Default + 'static,
    {
        fn default() -> Self {
            Self::derive(|| Default::default())
        }
    }

    impl<T> Default for Signal<T, LocalStorage>
    where
        T: Default + 'static,
    {
        fn default() -> Self {
            Self::derive_local(|| Default::default())
        }
    }

    impl<T: Clone + Send + Sync + 'static> From<T> for ArcSignal<T> {
        #[track_caller]
        fn from(value: T) -> Self {
            Self::derive(move || value.clone())
        }
    }

    impl<T> From<T> for Signal<T>
    where
        T: Clone + Send + Sync + 'static,
    {
        #[track_caller]
        fn from(value: T) -> Self {
            Self::derive(move || value.clone())
        }
    }

    impl<T> From<T> for Signal<T, LocalStorage>
    where
        T: Clone + 'static,
    {
        #[track_caller]
        fn from(value: T) -> Self {
            Self::derive_local(move || value.clone())
        }
    }

    impl<T> From<ArcSignal<T>> for Signal<T>
    where
        T: Send + Sync + 'static,
    {
        #[track_caller]
        fn from(value: ArcSignal<T>) -> Self {
            Signal {
                #[cfg(debug_assertions)]
                defined_at: Location::caller(),
                inner: StoredValue::new(value.inner),
            }
        }
    }

    impl<T> FromLocal<ArcSignal<T>> for Signal<T, LocalStorage>
    where
        T: 'static,
    {
        #[track_caller]
        fn from_local(value: ArcSignal<T>) -> Self {
            Signal {
                #[cfg(debug_assertions)]
                defined_at: Location::caller(),
                inner: StoredValue::new_local(value.inner),
            }
        }
    }

    impl<T, S> From<Signal<T, S>> for ArcSignal<T>
    where
        S: Storage<SignalTypes<T>>,
    {
        #[track_caller]
        fn from(value: Signal<T, S>) -> Self {
            ArcSignal {
                #[cfg(debug_assertions)]
                defined_at: Location::caller(),
                inner: value
                    .inner
                    .try_get_value()
                    .unwrap_or_else(unwrap_signal!(value)),
            }
        }
    }

    impl<T> From<ReadSignal<T>> for Signal<T>
    where
        T: Send + Sync + 'static,
    {
        #[track_caller]
        fn from(value: ReadSignal<T>) -> Self {
            Self {
                inner: StoredValue::new(SignalTypes::ReadSignal(value.into())),
                #[cfg(debug_assertions)]
                defined_at: std::panic::Location::caller(),
            }
        }
    }

    impl<T> From<ReadSignal<T, LocalStorage>> for Signal<T, LocalStorage>
    where
        T: 'static,
    {
        #[track_caller]
        fn from(value: ReadSignal<T, LocalStorage>) -> Self {
            Self {
                inner: StoredValue::new_local(SignalTypes::ReadSignal(
                    value.into(),
                )),
                #[cfg(debug_assertions)]
                defined_at: std::panic::Location::caller(),
            }
        }
    }

    impl<T> From<RwSignal<T>> for Signal<T>
    where
        T: Send + Sync + 'static,
    {
        #[track_caller]
        fn from(value: RwSignal<T>) -> Self {
            Self {
                inner: StoredValue::new(SignalTypes::ReadSignal(
                    value.read_only().into(),
                )),
                #[cfg(debug_assertions)]
                defined_at: std::panic::Location::caller(),
            }
        }
    }

    impl<T> From<RwSignal<T, LocalStorage>> for Signal<T, LocalStorage>
    where
        T: 'static,
    {
        #[track_caller]
        fn from(value: RwSignal<T, LocalStorage>) -> Self {
            Self {
                inner: StoredValue::new_local(SignalTypes::ReadSignal(
                    value.read_only().into(),
                )),
                #[cfg(debug_assertions)]
                defined_at: std::panic::Location::caller(),
            }
        }
    }

    impl<T> From<Memo<T>> for Signal<T>
    where
        T: Send + Sync + 'static,
    {
        #[track_caller]
        fn from(value: Memo<T>) -> Self {
            Self {
                inner: StoredValue::new(SignalTypes::Memo(value.into())),
                #[cfg(debug_assertions)]
                defined_at: std::panic::Location::caller(),
            }
        }
    }

    impl<T> From<Memo<T, LocalStorage>> for Signal<T, LocalStorage>
    where
        T: 'static,
    {
        #[track_caller]
        fn from(value: Memo<T, LocalStorage>) -> Self {
            Self {
                inner: StoredValue::new_local(SignalTypes::Memo(value.into())),
                #[cfg(debug_assertions)]
                defined_at: std::panic::Location::caller(),
            }
        }
    }

    /// A wrapper for a value that is *either* `T` or [`Signal<T>`].
    ///
    /// This allows you to create APIs that take either a reactive or a non-reactive value
    /// of the same type. This is especially useful for component properties.
    ///
    /// ```
    /// # use reactive_graph::signal::*;
    /// # use reactive_graph::wrappers::read::MaybeSignal;
    /// # use reactive_graph::computed::Memo;
    /// # use reactive_graph::prelude::*;
    /// let (count, set_count) = signal(2);
    /// let double_count = MaybeSignal::derive(move || count.get() * 2);
    /// let memoized_double_count = Memo::new(move |_| count.get() * 2);
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
    /// ```
    #[derive(Debug, PartialEq, Eq)]
    pub enum MaybeSignal<T, S = SyncStorage>
    where
        T: 'static,
    {
        /// An unchanging value of type `T`.
        Static(T),
        /// A reactive signal that contains a value of type `T`.
        Dynamic(Signal<T, S>),
    }

    impl<T: Clone, S> Clone for MaybeSignal<T, S> {
        fn clone(&self) -> Self {
            match self {
                Self::Static(item) => Self::Static(item.clone()),
                Self::Dynamic(signal) => Self::Dynamic(*signal),
            }
        }
    }

    impl<T: Copy, S> Copy for MaybeSignal<T, S> {}

    impl<T: Default, S> Default for MaybeSignal<T, S> {
        fn default() -> Self {
            Self::Static(Default::default())
        }
    }

    impl<T, S> DefinedAt for MaybeSignal<T, S> {
        fn defined_at(&self) -> Option<&'static Location<'static>> {
            // TODO this could be improved, but would require moving from an enum to a struct here.
            // Probably not worth it for relatively small benefits.
            None
        }
    }

    impl<T, S> WithUntracked for MaybeSignal<T, S>
    where
        S: Storage<SignalTypes<T>>,
    {
        type Value = T;

        fn try_with_untracked<U>(
            &self,
            fun: impl FnOnce(&Self::Value) -> U,
        ) -> Option<U> {
            match self {
                Self::Static(t) => Some(fun(t)),
                Self::Dynamic(s) => s.try_with_untracked(fun),
            }
        }
    }

    impl<T, S> With for MaybeSignal<T, S>
    where
        T: Send + Sync + 'static,
        S: Storage<SignalTypes<T>>,
    {
        type Value = T;

        fn try_with<U>(
            &self,
            fun: impl FnOnce(&Self::Value) -> U,
        ) -> Option<U> {
            match self {
                Self::Static(t) => Some(fun(t)),
                Self::Dynamic(s) => s.try_with(fun),
            }
        }
    }

    impl<T> MaybeSignal<T>
    where
        T: Send + Sync,
    {
        /// Wraps a derived signal, i.e., any computation that accesses one or more
        /// reactive signals.
        pub fn derive(
            derived_signal: impl Fn() -> T + Send + Sync + 'static,
        ) -> Self {
            Self::Dynamic(Signal::derive(derived_signal))
        }
    }

    impl<T> MaybeSignal<T, LocalStorage> {
        /// Wraps a derived signal, i.e., any computation that accesses one or more
        /// reactive signals.
        pub fn derive_local(derived_signal: impl Fn() -> T + 'static) -> Self {
            Self::Dynamic(Signal::derive_local(derived_signal))
        }
    }

    impl<T, S> From<T> for MaybeSignal<T, S> {
        fn from(value: T) -> Self {
            Self::Static(value)
        }
    }

    impl<T> From<ReadSignal<T>> for MaybeSignal<T>
    where
        T: Send + Sync,
    {
        fn from(value: ReadSignal<T>) -> Self {
            Self::Dynamic(value.into())
        }
    }

    impl<T> From<ReadSignal<T, LocalStorage>> for MaybeSignal<T, LocalStorage> {
        fn from(value: ReadSignal<T, LocalStorage>) -> Self {
            Self::Dynamic(value.into())
        }
    }

    impl<T> From<RwSignal<T>> for MaybeSignal<T>
    where
        T: Send + Sync,
    {
        fn from(value: RwSignal<T>) -> Self {
            Self::Dynamic(value.into())
        }
    }

    impl<T> From<RwSignal<T, LocalStorage>> for MaybeSignal<T, LocalStorage> {
        fn from(value: RwSignal<T, LocalStorage>) -> Self {
            Self::Dynamic(value.into())
        }
    }

    impl<T> From<Memo<T>> for MaybeSignal<T>
    where
        T: Send + Sync,
    {
        fn from(value: Memo<T>) -> Self {
            Self::Dynamic(value.into())
        }
    }

    impl<T> From<Memo<T, LocalStorage>> for MaybeSignal<T, LocalStorage> {
        fn from(value: Memo<T, LocalStorage>) -> Self {
            Self::Dynamic(value.into())
        }
    }

    impl<T> From<ArcReadSignal<T>> for MaybeSignal<T>
    where
        T: Send + Sync,
    {
        fn from(value: ArcReadSignal<T>) -> Self {
            ReadSignal::from(value).into()
        }
    }

    impl<T> FromLocal<ArcReadSignal<T>> for MaybeSignal<T, LocalStorage> {
        fn from_local(value: ArcReadSignal<T>) -> Self {
            ReadSignal::from_local(value).into()
        }
    }

    impl<T> From<ArcRwSignal<T>> for MaybeSignal<T>
    where
        T: Send + Sync + 'static,
    {
        fn from(value: ArcRwSignal<T>) -> Self {
            RwSignal::from(value).into()
        }
    }

    impl<T> FromLocal<ArcRwSignal<T>> for MaybeSignal<T, LocalStorage>
    where
        T: 'static,
    {
        fn from_local(value: ArcRwSignal<T>) -> Self {
            RwSignal::from_local(value).into()
        }
    }

    impl<T> From<ArcMemo<T>> for MaybeSignal<T>
    where
        T: Send + Sync,
    {
        fn from(value: ArcMemo<T>) -> Self {
            Memo::from(value).into()
        }
    }

    impl<T> FromLocal<ArcMemo<T>> for MaybeSignal<T, LocalStorage> {
        fn from_local(value: ArcMemo<T>) -> Self {
            Memo::from_local(value).into()
        }
    }

    impl<T, S> From<Signal<T, S>> for MaybeSignal<T, S> {
        fn from(value: Signal<T, S>) -> Self {
            Self::Dynamic(value)
        }
    }

    impl<S> From<&str> for MaybeSignal<String, S> {
        fn from(value: &str) -> Self {
            Self::Static(value.to_string())
        }
    }

    /// A wrapping type for an optional component prop, which can either be a signal or a
    /// non-reactive value, and which may or may not have a value. In other words, this is
    /// an `Option<MaybeSignal<Option<T>>>` that automatically flattens its getters.
    ///
    /// This creates an extremely flexible type for component libraries, etc.
    ///
    /// ## Examples
    /// ```rust
    /// # use reactive_graph::signal::*;
    /// # use reactive_graph::wrappers::read::MaybeProp;
    /// # use reactive_graph::computed::Memo;
    /// # use reactive_graph::prelude::*;
    /// let (count, set_count) = signal(Some(2));
    /// let double = |n| n * 2;
    /// let double_count = MaybeProp::derive(move || count.get().map(double));
    /// let memoized_double_count = Memo::new(move |_| count.get().map(double));
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
    /// ```
    #[derive(Debug, PartialEq, Eq)]
    pub struct MaybeProp<T: 'static, S = SyncStorage>(
        pub(crate) Option<MaybeSignal<Option<T>, S>>,
    );

    impl<T: Clone, S> Clone for MaybeProp<T, S> {
        fn clone(&self) -> Self {
            Self(self.0.clone())
        }
    }

    impl<T: Copy, S> Copy for MaybeProp<T, S> {}

    impl<T, S> Default for MaybeProp<T, S> {
        fn default() -> Self {
            Self(None)
        }
    }

    impl<T, S> DefinedAt for MaybeProp<T, S> {
        fn defined_at(&self) -> Option<&'static Location<'static>> {
            // TODO this can be improved by adding a defined_at field
            None
        }
    }

    impl<T, S> WithUntracked for MaybeProp<T, S>
    where
        S: Storage<SignalTypes<Option<T>>>,
    {
        type Value = Option<T>;

        fn try_with_untracked<U>(
            &self,
            fun: impl FnOnce(&Self::Value) -> U,
        ) -> Option<U> {
            match &self.0 {
                None => Some(fun(&None)),
                Some(inner) => inner.try_with_untracked(fun),
            }
        }
    }

    impl<T, S> With for MaybeProp<T, S>
    where
        T: Send + Sync + 'static,
        S: Storage<SignalTypes<Option<T>>>,
    {
        type Value = Option<T>;

        fn try_with<U>(
            &self,
            fun: impl FnOnce(&Self::Value) -> U,
        ) -> Option<U> {
            match &self.0 {
                None => Some(fun(&None)),
                Some(inner) => inner.try_with(fun),
            }
        }
    }

    impl<T> MaybeProp<T>
    where
        T: Send + Sync,
    {
        /// Wraps a derived signal, i.e., any computation that accesses one or more
        /// reactive signals.
        pub fn derive(
            derived_signal: impl Fn() -> Option<T> + Send + Sync + 'static,
        ) -> Self {
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

    impl<T> From<ReadSignal<Option<T>>> for MaybeProp<T>
    where
        T: Send + Sync,
    {
        fn from(value: ReadSignal<Option<T>>) -> Self {
            Self(Some(value.into()))
        }
    }

    impl<T> From<RwSignal<Option<T>>> for MaybeProp<T>
    where
        T: Send + Sync,
    {
        fn from(value: RwSignal<Option<T>>) -> Self {
            Self(Some(value.into()))
        }
    }

    impl<T> From<Memo<Option<T>>> for MaybeProp<T>
    where
        T: Send + Sync,
    {
        fn from(value: Memo<Option<T>>) -> Self {
            Self(Some(value.into()))
        }
    }

    impl<T> From<Signal<Option<T>>> for MaybeProp<T> {
        fn from(value: Signal<Option<T>>) -> Self {
            Self(Some(value.into()))
        }
    }

    impl<T> From<ReadSignal<T>> for MaybeProp<T>
    where
        T: Send + Sync + Clone,
    {
        fn from(value: ReadSignal<T>) -> Self {
            Self(Some(MaybeSignal::derive(move || Some(value.get()))))
        }
    }

    impl<T> From<RwSignal<T>> for MaybeProp<T>
    where
        T: Send + Sync + Clone,
    {
        fn from(value: RwSignal<T>) -> Self {
            Self(Some(MaybeSignal::derive(move || Some(value.get()))))
        }
    }

    impl<T> From<Memo<T>> for MaybeProp<T>
    where
        T: Send + Sync + Clone,
    {
        fn from(value: Memo<T>) -> Self {
            Self(Some(MaybeSignal::derive(move || Some(value.get()))))
        }
    }

    impl<T> From<Signal<T>> for MaybeProp<T>
    where
        T: Send + Sync + Clone,
    {
        fn from(value: Signal<T>) -> Self {
            Self(Some(MaybeSignal::derive(move || Some(value.get()))))
        }
    }

    impl From<&str> for MaybeProp<String> {
        fn from(value: &str) -> Self {
            Self(Some(MaybeSignal::from(Some(value.to_string()))))
        }
    }

    impl<T> MaybeProp<T, LocalStorage> {
        /// Wraps a derived signal, i.e., any computation that accesses one or more
        /// reactive signals.
        pub fn derive_local(
            derived_signal: impl Fn() -> Option<T> + 'static,
        ) -> Self {
            Self(Some(MaybeSignal::derive_local(derived_signal)))
        }
    }

    impl<T> FromLocal<T> for MaybeProp<T, LocalStorage> {
        fn from_local(value: T) -> Self {
            Self(Some(MaybeSignal::from(Some(value))))
        }
    }

    impl<T> FromLocal<Option<T>> for MaybeProp<T, LocalStorage> {
        fn from_local(value: Option<T>) -> Self {
            Self(Some(MaybeSignal::from(value)))
        }
    }

    impl<T> From<MaybeSignal<Option<T>, LocalStorage>>
        for MaybeProp<T, LocalStorage>
    {
        fn from(value: MaybeSignal<Option<T>, LocalStorage>) -> Self {
            Self(Some(value))
        }
    }

    impl<T> From<Option<MaybeSignal<Option<T>, LocalStorage>>>
        for MaybeProp<T, LocalStorage>
    {
        fn from(value: Option<MaybeSignal<Option<T>, LocalStorage>>) -> Self {
            Self(value)
        }
    }

    impl<T> From<ReadSignal<Option<T>, LocalStorage>> for MaybeProp<T, LocalStorage>
    where
        T: Send + Sync,
    {
        fn from(value: ReadSignal<Option<T>, LocalStorage>) -> Self {
            Self(Some(value.into()))
        }
    }

    impl<T> From<RwSignal<Option<T>, LocalStorage>> for MaybeProp<T, LocalStorage>
    where
        T: Send + Sync,
    {
        fn from(value: RwSignal<Option<T>, LocalStorage>) -> Self {
            Self(Some(value.into()))
        }
    }

    impl<T> From<Memo<Option<T>, LocalStorage>> for MaybeProp<T, LocalStorage>
    where
        T: Send + Sync,
    {
        fn from(value: Memo<Option<T>, LocalStorage>) -> Self {
            Self(Some(value.into()))
        }
    }

    impl<T> From<Signal<Option<T>, LocalStorage>> for MaybeProp<T, LocalStorage> {
        fn from(value: Signal<Option<T>, LocalStorage>) -> Self {
            Self(Some(value.into()))
        }
    }

    impl<T> From<ReadSignal<T, LocalStorage>> for MaybeProp<T, LocalStorage>
    where
        T: Send + Sync + Clone,
    {
        fn from(value: ReadSignal<T, LocalStorage>) -> Self {
            Self(Some(MaybeSignal::derive_local(move || Some(value.get()))))
        }
    }

    impl<T> From<RwSignal<T, LocalStorage>> for MaybeProp<T, LocalStorage>
    where
        T: Send + Sync + Clone,
    {
        fn from(value: RwSignal<T, LocalStorage>) -> Self {
            Self(Some(MaybeSignal::derive_local(move || Some(value.get()))))
        }
    }

    impl<T> From<Memo<T, LocalStorage>> for MaybeProp<T, LocalStorage>
    where
        T: Send + Sync + Clone,
    {
        fn from(value: Memo<T, LocalStorage>) -> Self {
            Self(Some(MaybeSignal::derive_local(move || Some(value.get()))))
        }
    }

    impl<T> From<Signal<T, LocalStorage>> for MaybeProp<T, LocalStorage>
    where
        T: Send + Sync + Clone,
    {
        fn from(value: Signal<T, LocalStorage>) -> Self {
            Self(Some(MaybeSignal::derive_local(move || Some(value.get()))))
        }
    }

    impl From<&str> for MaybeProp<String, LocalStorage> {
        fn from(value: &str) -> Self {
            Self(Some(MaybeSignal::from(Some(value.to_string()))))
        }
    }
}

/// Types that abstract over the ability to update a signal.
pub mod write {
    use crate::{
        owner::{Storage, StoredValue, SyncStorage},
        signal::{ArcRwSignal, ArcWriteSignal, RwSignal, WriteSignal},
        traits::Set,
    };

    /// Helper trait for converting `Fn(T)` into [`SignalSetter<T>`].
    pub trait IntoSignalSetter<T, S>: Sized {
        /// Consumes `self`, returning [`SignalSetter<T>`].
        fn into_signal_setter(self) -> SignalSetter<T, S>;
    }

    impl<F, T, S> IntoSignalSetter<T, S> for F
    where
        F: Fn(T) + 'static + Send + Sync,
        S: Storage<Box<dyn Fn(T) + Send + Sync>>,
    {
        fn into_signal_setter(self) -> SignalSetter<T, S> {
            SignalSetter::map(self)
        }
    }

    /// A wrapper for any kind of settable reactive signal: a [`WriteSignal`],
    /// [`RwSignal`], or closure that receives a value and sets a signal depending
    /// on it.
    ///
    /// This allows you to create APIs that take any kind of `SignalSetter<T>` as an argument,
    /// rather than adding a generic `F: Fn(T)`. Values can be set with the same
    /// function call or `set()`, API as other signals.
    ///
    /// ## Core Trait Implementations
    /// - [`.set()`](#impl-SignalSet<T>-for-SignalSetter<T>) (or calling the setter as a function)
    ///   sets the signal’s value, and notifies all subscribers that the signal’s value has changed.
    ///   to subscribe to the signal, and to re-run whenever the value of the signal changes.
    ///
    /// ## Examples
    /// ```rust
    /// # use reactive_graph::prelude::*;
    /// # use reactive_graph::wrappers::write::SignalSetter;
    /// # use reactive_graph::signal::signal;
    /// let (count, set_count) = signal(2);
    /// let set_double_input = SignalSetter::map(move |n| set_count.set(n * 2));
    ///
    /// // this function takes any kind of signal setter
    /// fn set_to_4(setter: &SignalSetter<i32>) {
    ///     // ✅ calling the signal sets the value
    ///     //    can be `setter(4)` on nightly
    ///     setter.set(4);
    /// }
    ///
    /// set_to_4(&set_count.into());
    /// assert_eq!(count.get(), 4);
    /// set_to_4(&set_double_input);
    /// assert_eq!(count.get(), 8);
    /// ```
    #[derive(Debug, PartialEq, Eq)]
    pub struct SignalSetter<T, S = SyncStorage>
    where
        T: 'static,
    {
        inner: SignalSetterTypes<T, S>,
        #[cfg(debug_assertions)]
        defined_at: &'static std::panic::Location<'static>,
    }

    impl<T, S> Clone for SignalSetter<T, S> {
        fn clone(&self) -> Self {
            *self
        }
    }

    impl<T: Default + 'static, S> Default for SignalSetter<T, S> {
        #[track_caller]
        fn default() -> Self {
            Self {
                inner: SignalSetterTypes::Default,
                #[cfg(debug_assertions)]
                defined_at: std::panic::Location::caller(),
            }
        }
    }

    impl<T, S> Copy for SignalSetter<T, S> {}

    impl<T, S> Set for SignalSetter<T, S>
    where
        T: 'static,
        S: Storage<ArcWriteSignal<T>> + Storage<Box<dyn Fn(T) + Send + Sync>>,
    {
        type Value = T;

        fn set(&self, new_value: Self::Value) {
            match self.inner {
                SignalSetterTypes::Default => {}
                SignalSetterTypes::Write(w) => w.set(new_value),
                SignalSetterTypes::Mapped(s) => {
                    s.with_value(|setter| setter(new_value))
                }
            }
        }

        fn try_set(&self, new_value: Self::Value) -> Option<Self::Value> {
            match self.inner {
                SignalSetterTypes::Default => Some(new_value),
                SignalSetterTypes::Write(w) => w.try_set(new_value),
                SignalSetterTypes::Mapped(s) => {
                    let mut new_value = Some(new_value);

                    let _ = s.try_with_value(|setter| {
                        setter(new_value.take().unwrap())
                    });

                    new_value
                }
            }
        }
    }

    impl<T, S> SignalSetter<T, S>
    where
        S: Storage<Box<dyn Fn(T) + Send + Sync>>,
    {
        /// Wraps a signal-setting closure, i.e., any computation that sets one or more reactive signals.
        #[track_caller]
        pub fn map(mapped_setter: impl Fn(T) + Send + Sync + 'static) -> Self {
            Self {
                inner: SignalSetterTypes::Mapped(
                    StoredValue::new_with_storage(Box::new(mapped_setter)),
                ),
                #[cfg(debug_assertions)]
                defined_at: std::panic::Location::caller(),
            }
        }
    }

    impl<T, S> From<WriteSignal<T, S>> for SignalSetter<T, S> {
        #[track_caller]
        fn from(value: WriteSignal<T, S>) -> Self {
            Self {
                inner: SignalSetterTypes::Write(value),
                #[cfg(debug_assertions)]
                defined_at: std::panic::Location::caller(),
            }
        }
    }

    impl<T, S> From<RwSignal<T, S>> for SignalSetter<T, S>
    where
        T: Send + Sync + 'static,
        S: Storage<ArcRwSignal<T>> + Storage<ArcWriteSignal<T>>,
    {
        #[track_caller]
        fn from(value: RwSignal<T, S>) -> Self {
            Self {
                inner: SignalSetterTypes::Write(value.write_only()),
                #[cfg(debug_assertions)]
                defined_at: std::panic::Location::caller(),
            }
        }
    }

    enum SignalSetterTypes<T, S = SyncStorage>
    where
        T: 'static,
    {
        Write(WriteSignal<T, S>),
        Mapped(StoredValue<Box<dyn Fn(T) + Send + Sync>, S>),
        Default,
    }

    impl<T, S> Clone for SignalSetterTypes<T, S> {
        fn clone(&self) -> Self {
            *self
        }
    }

    impl<T, S> Copy for SignalSetterTypes<T, S> {}

    impl<T, S> core::fmt::Debug for SignalSetterTypes<T, S>
    where
        T: core::fmt::Debug,
        S: core::fmt::Debug,
    {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match self {
                Self::Write(arg0) => {
                    f.debug_tuple("WriteSignal").field(arg0).finish()
                }
                Self::Mapped(_) => f.debug_tuple("Mapped").finish(),
                Self::Default => {
                    f.debug_tuple("SignalSetter<Default>").finish()
                }
            }
        }
    }

    impl<T, S> PartialEq for SignalSetterTypes<T, S>
    where
        T: PartialEq,
    {
        fn eq(&self, other: &Self) -> bool {
            match (self, other) {
                (Self::Write(l0), Self::Write(r0)) => l0 == r0,
                (Self::Mapped(l0), Self::Mapped(r0)) => std::ptr::eq(l0, r0),
                _ => false,
            }
        }
    }

    impl<T, S> Eq for SignalSetterTypes<T, S> where T: PartialEq {}
}
