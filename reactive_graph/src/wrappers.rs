pub mod read {
    use crate::{
        computed::{ArcMemo, Memo},
        owner::StoredValue,
        signal::{ArcReadSignal, ArcRwSignal, ReadSignal, RwSignal},
        traits::{DefinedAt, Dispose, Get, With, WithUntracked},
        untrack, unwrap_signal,
    };
    use std::{panic::Location, sync::Arc};

    enum SignalTypes<T: 'static> {
        ReadSignal(ArcReadSignal<T>),
        Memo(ArcMemo<T>),
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
                #[cfg(any(debug_assertions, feature = "ssr"))]
                defined_at: std::panic::Location::caller(),
            }
        }
    }

    impl<T: Send + Sync> From<ArcRwSignal<T>> for ArcSignal<T> {
        #[track_caller]
        fn from(value: ArcRwSignal<T>) -> Self {
            Self {
                inner: SignalTypes::ReadSignal(value.read_only()),
                #[cfg(any(debug_assertions, feature = "ssr"))]
                defined_at: std::panic::Location::caller(),
            }
        }
    }

    impl<T: Send + Sync> From<ArcMemo<T>> for ArcSignal<T> {
        #[track_caller]
        fn from(value: ArcMemo<T>) -> Self {
            Self {
                inner: SignalTypes::Memo(value),
                #[cfg(any(debug_assertions, feature = "ssr"))]
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

    pub struct Signal<T: 'static> {
        #[cfg(debug_assertions)]
        defined_at: &'static Location<'static>,
        inner: StoredValue<SignalTypes<T>>,
    }

    impl<T: Send + Sync + 'static> Dispose for Signal<T> {
        fn dispose(self) {
            self.inner.dispose()
        }
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
            #[cfg(debug_assertions)]
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

    impl<T> DefinedAt for Signal<T> {
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

    impl<T> WithUntracked for Signal<T>
    where
        T: Send + Sync,
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

    impl<T> With for Signal<T>
    where
        T: Send + Sync,
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
                inner: StoredValue::new(SignalTypes::DerivedSignal(Arc::new(
                    derived_signal,
                ))),
                #[cfg(debug_assertions)]
                defined_at: std::panic::Location::caller(),
            }
        }
    }

    impl<T> Default for Signal<T>
    where
        T: Default + Send + Sync + 'static,
    {
        fn default() -> Self {
            Self::derive(|| Default::default())
        }
    }

    impl<T: Send + Sync + 'static> From<ArcSignal<T>> for Signal<T> {
        #[track_caller]
        fn from(value: ArcSignal<T>) -> Self {
            Signal {
                #[cfg(debug_assertions)]
                defined_at: Location::caller(),
                inner: StoredValue::new(value.inner),
            }
        }
    }

    impl<T: Send + Sync + 'static> From<Signal<T>> for ArcSignal<T> {
        #[track_caller]
        fn from(value: Signal<T>) -> Self {
            ArcSignal {
                #[cfg(debug_assertions)]
                defined_at: Location::caller(),
                inner: value.inner.get().unwrap_or_else(unwrap_signal!(value)),
            }
        }
    }

    impl<T: Send + Sync> From<ReadSignal<T>> for Signal<T> {
        #[track_caller]
        fn from(value: ReadSignal<T>) -> Self {
            Self {
                inner: StoredValue::new(SignalTypes::ReadSignal(value.into())),
                #[cfg(any(debug_assertions, feature = "ssr"))]
                defined_at: std::panic::Location::caller(),
            }
        }
    }

    impl<T: Send + Sync> From<RwSignal<T>> for Signal<T> {
        #[track_caller]
        fn from(value: RwSignal<T>) -> Self {
            Self {
                inner: StoredValue::new(SignalTypes::ReadSignal(
                    value.read_only().into(),
                )),
                #[cfg(any(debug_assertions, feature = "ssr"))]
                defined_at: std::panic::Location::caller(),
            }
        }
    }

    impl<T: Send + Sync> From<Memo<T>> for Signal<T> {
        #[track_caller]
        fn from(value: Memo<T>) -> Self {
            Self {
                inner: StoredValue::new(SignalTypes::Memo(value.into())),
                #[cfg(any(debug_assertions, feature = "ssr"))]
                defined_at: std::panic::Location::caller(),
            }
        }
    }

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

    impl<T> DefinedAt for MaybeSignal<T> {
        fn defined_at(&self) -> Option<&'static Location<'static>> {
            // TODO this could be improved, but would require moving from an enum to a struct here.
            // Probably not worth it for relatively small benefits.
            None
        }
    }

    impl<T: Send + Sync> WithUntracked for MaybeSignal<T> {
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

    impl<T: Send + Sync> With for MaybeSignal<T> {
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
        T: Send + Sync + 'static,
    {
        pub fn derive(
            derived_signal: impl Fn() -> T + Send + Sync + 'static,
        ) -> Self {
            Self::Dynamic(Signal::derive(derived_signal))
        }
    }

    impl<T> From<T> for MaybeSignal<T> {
        fn from(value: T) -> Self {
            Self::Static(value)
        }
    }

    impl<T: Send + Sync> From<ReadSignal<T>> for MaybeSignal<T> {
        fn from(value: ReadSignal<T>) -> Self {
            Self::Dynamic(value.into())
        }
    }

    impl<T: Send + Sync> From<RwSignal<T>> for MaybeSignal<T> {
        fn from(value: RwSignal<T>) -> Self {
            Self::Dynamic(value.into())
        }
    }

    impl<T: Send + Sync> From<Memo<T>> for MaybeSignal<T> {
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

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct MaybeProp<T: Send + Sync + 'static>(
        pub(crate) Option<MaybeSignal<Option<T>>>,
    );

    impl<T: Send + Sync + Copy> Copy for MaybeProp<T> {}

    impl<T: Send + Sync> Default for MaybeProp<T> {
        fn default() -> Self {
            Self(None)
        }
    }

    impl<T: Send + Sync> DefinedAt for MaybeProp<T> {
        fn defined_at(&self) -> Option<&'static Location<'static>> {
            // TODO this can be improved by adding a defined_at field
            None
        }
    }

    impl<T: Send + Sync> WithUntracked for MaybeProp<T> {
        type Value = Option<T>;

        fn try_with_untracked<U>(
            &self,
            fun: impl FnOnce(&Self::Value) -> U,
        ) -> Option<U> {
            self.0.as_ref().and_then(|n| n.try_with_untracked(fun))
        }
    }

    impl<T: Send + Sync> With for MaybeProp<T> {
        type Value = Option<T>;

        fn try_with<U>(
            &self,
            fun: impl FnOnce(&Self::Value) -> U,
        ) -> Option<U> {
            self.0.as_ref().and_then(|n| n.try_with(fun))
        }
    }

    impl<T> MaybeProp<T>
    where
        T: Send + Sync + 'static,
    {
        pub fn derive(
            derived_signal: impl Fn() -> Option<T> + Send + Sync + 'static,
        ) -> Self {
            Self(Some(MaybeSignal::derive(derived_signal)))
        }
    }

    impl<T: Send + Sync> From<T> for MaybeProp<T> {
        fn from(value: T) -> Self {
            Self(Some(MaybeSignal::from(Some(value))))
        }
    }

    impl<T: Send + Sync> From<Option<T>> for MaybeProp<T> {
        fn from(value: Option<T>) -> Self {
            Self(Some(MaybeSignal::from(value)))
        }
    }

    impl<T: Send + Sync> From<MaybeSignal<Option<T>>> for MaybeProp<T> {
        fn from(value: MaybeSignal<Option<T>>) -> Self {
            Self(Some(value))
        }
    }

    impl<T: Send + Sync> From<Option<MaybeSignal<Option<T>>>> for MaybeProp<T> {
        fn from(value: Option<MaybeSignal<Option<T>>>) -> Self {
            Self(value)
        }
    }

    impl<T: Send + Sync> From<ReadSignal<Option<T>>> for MaybeProp<T> {
        fn from(value: ReadSignal<Option<T>>) -> Self {
            Self(Some(value.into()))
        }
    }

    impl<T: Send + Sync> From<RwSignal<Option<T>>> for MaybeProp<T> {
        fn from(value: RwSignal<Option<T>>) -> Self {
            Self(Some(value.into()))
        }
    }

    impl<T: Send + Sync> From<Memo<Option<T>>> for MaybeProp<T> {
        fn from(value: Memo<Option<T>>) -> Self {
            Self(Some(value.into()))
        }
    }

    impl<T: Send + Sync> From<Signal<Option<T>>> for MaybeProp<T> {
        fn from(value: Signal<Option<T>>) -> Self {
            Self(Some(value.into()))
        }
    }

    impl<T: Send + Sync + Clone> From<ReadSignal<T>> for MaybeProp<T> {
        fn from(value: ReadSignal<T>) -> Self {
            Self(Some(MaybeSignal::derive(move || Some(value.get()))))
        }
    }

    impl<T: Send + Sync + Clone> From<RwSignal<T>> for MaybeProp<T> {
        fn from(value: RwSignal<T>) -> Self {
            Self(Some(MaybeSignal::derive(move || Some(value.get()))))
        }
    }

    impl<T: Send + Sync + Clone> From<Memo<T>> for MaybeProp<T> {
        fn from(value: Memo<T>) -> Self {
            Self(Some(MaybeSignal::derive(move || Some(value.get()))))
        }
    }

    impl<T: Send + Sync + Clone> From<Signal<T>> for MaybeProp<T> {
        fn from(value: Signal<T>) -> Self {
            Self(Some(MaybeSignal::derive(move || Some(value.get()))))
        }
    }

    impl From<&str> for MaybeProp<String> {
        fn from(value: &str) -> Self {
            Self(Some(MaybeSignal::from(Some(value.to_string()))))
        }
    }
}

pub mod write {
    use crate::{
        owner::StoredValue,
        signal::{RwSignal, WriteSignal},
        traits::Set,
    };

    /// Helper trait for converting `Fn(T)` into [`SignalSetter<T>`].
    pub trait IntoSignalSetter<T>: Sized {
        /// Consumes `self`, returning [`SignalSetter<T>`].
        fn into_signal_setter(self) -> SignalSetter<T>;
    }

    impl<F, T> IntoSignalSetter<T> for F
    where
        F: Fn(T) + 'static + Send + Sync,
        T: Send + Sync,
    {
        fn into_signal_setter(self) -> SignalSetter<T> {
            SignalSetter::map(self)
        }
    }

    /// A wrapper for any kind of settable reactive signal: a [`WriteSignal`](crate::WriteSignal),
    /// [`RwSignal`](crate::RwSignal), or closure that receives a value and sets a signal depending
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
    /// # tokio_test::block_on(async move {
    /// # any_spawner::Executor::init_tokio();
    /// # let _guard = reactive_graph::diagnostics::SpecialNonReactiveZone::enter();
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
    /// # });
    /// ```
    #[derive(Debug, PartialEq, Eq)]
    pub struct SignalSetter<T>
    where
        T: 'static,
    {
        inner: SignalSetterTypes<T>,
        #[cfg(any(debug_assertions, feature = "ssr"))]
        defined_at: &'static std::panic::Location<'static>,
    }

    impl<T> Clone for SignalSetter<T> {
        fn clone(&self) -> Self {
            *self
        }
    }

    impl<T: Default + 'static> Default for SignalSetter<T> {
        #[track_caller]
        fn default() -> Self {
            Self {
                inner: SignalSetterTypes::Default,
                #[cfg(any(debug_assertions, feature = "ssr"))]
                defined_at: std::panic::Location::caller(),
            }
        }
    }

    impl<T> Copy for SignalSetter<T> {}

    impl<T> Set for SignalSetter<T> {
        type Value = T;

        fn set(&self, new_value: impl Into<Self::Value>) {
            let new_value = new_value.into();
            match self.inner {
                SignalSetterTypes::Default => {}
                SignalSetterTypes::Write(w) => w.set(new_value),
                SignalSetterTypes::Mapped(s) => {
                    s.with_value(|setter| setter(new_value))
                }
            }
        }

        fn try_set(
            &self,
            new_value: impl Into<Self::Value>,
        ) -> Option<Self::Value> {
            let new_value = new_value.into();
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

    impl<T> SignalSetter<T>
    where
        T: Send + Sync + 'static,
    {
        #[track_caller]
        pub fn map(mapped_setter: impl Fn(T) + Send + Sync + 'static) -> Self {
            Self {
                inner: SignalSetterTypes::Mapped(StoredValue::new(Box::new(
                    mapped_setter,
                ))),
                #[cfg(any(debug_assertions, feature = "ssr"))]
                defined_at: std::panic::Location::caller(),
            }
        }
    }

    impl<T> SignalSetter<T>
    where
        T: 'static,
    {
        #[track_caller]
        pub fn set(&self, value: T) {
            match &self.inner {
                SignalSetterTypes::Write(s) => s.set(value),
                SignalSetterTypes::Mapped(s) => s.with_value(|s| s(value)),
                SignalSetterTypes::Default => {}
            }
        }
    }

    impl<T> From<WriteSignal<T>> for SignalSetter<T> {
        #[track_caller]
        fn from(value: WriteSignal<T>) -> Self {
            Self {
                inner: SignalSetterTypes::Write(value),
                #[cfg(any(debug_assertions, feature = "ssr"))]
                defined_at: std::panic::Location::caller(),
            }
        }
    }

    impl<T> From<RwSignal<T>> for SignalSetter<T>
    where
        T: Send + Sync + 'static,
    {
        #[track_caller]
        fn from(value: RwSignal<T>) -> Self {
            Self {
                inner: SignalSetterTypes::Write(value.write_only()),
                #[cfg(any(debug_assertions, feature = "ssr"))]
                defined_at: std::panic::Location::caller(),
            }
        }
    }

    enum SignalSetterTypes<T>
    where
        T: 'static,
    {
        Write(WriteSignal<T>),
        Mapped(StoredValue<Box<dyn Fn(T) + Send + Sync>>),
        Default,
    }

    impl<T> Clone for SignalSetterTypes<T> {
        fn clone(&self) -> Self {
            *self
        }
    }

    impl<T> Copy for SignalSetterTypes<T> {}

    impl<T> core::fmt::Debug for SignalSetterTypes<T>
    where
        T: core::fmt::Debug,
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

    impl<T> PartialEq for SignalSetterTypes<T>
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

    impl<T> Eq for SignalSetterTypes<T> where T: PartialEq {}
}
