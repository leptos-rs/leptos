pub mod read {
    use crate::{
        computed::ArcMemo,
        owner::StoredValue,
        signal::ArcReadSignal,
        traits::{DefinedAt, Get, GetUntracked},
        untrack,
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

    impl<T> GetUntracked for ArcSignal<T>
    where
        T: Send + Sync + Clone,
    {
        type Value = T;

        fn try_get_untracked(&self) -> Option<Self::Value> {
            match &self.inner {
                SignalTypes::ReadSignal(i) => i.try_get_untracked(),
                SignalTypes::Memo(i) => i.try_get_untracked(),
                SignalTypes::DerivedSignal(i) => Some(untrack(|| i())),
            }
        }
    }

    impl<T> Get for ArcSignal<T>
    where
        T: Send + Sync + Clone,
    {
        type Value = T;

        fn try_get(&self) -> Option<Self::Value> {
            match &self.inner {
                SignalTypes::ReadSignal(i) => i.try_get(),
                SignalTypes::Memo(i) => i.try_get(),
                SignalTypes::DerivedSignal(i) => Some(i()),
            }
        }
    }

    pub struct Signal<T: 'static> {
        #[cfg(debug_assertions)]
        defined_at: &'static Location<'static>,
        inner: StoredValue<SignalTypes<T>>,
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

    impl<T> GetUntracked for Signal<T>
    where
        T: Send + Sync + Clone,
    {
        type Value = T;

        fn try_get_untracked(&self) -> Option<Self::Value> {
            self.inner
                // cloning here clones the inner Arc and releases the lock, in case anything inside needs to take it
                // this happens particularly in derived signals, because they need to to access the
                // global arena again
                //
                // note that .read() multiple times in the same thread on a std RwLock can deadlock
                // to avoid writer starvation, which is why this happens
                .with_value(|inner| inner.clone())
                .and_then(|inner| match &inner {
                    SignalTypes::ReadSignal(i) => i.try_get_untracked(),
                    SignalTypes::Memo(i) => i.try_get_untracked(),
                    SignalTypes::DerivedSignal(i) => Some(untrack(|| i())),
                })
        }
    }

    impl<T> Get for Signal<T>
    where
        T: Send + Sync + Clone,
    {
        type Value = T;

        fn try_get(&self) -> Option<Self::Value> {
            self.inner
                .with_value(|inner| match &inner {
                    SignalTypes::ReadSignal(i) => i.try_get(),
                    SignalTypes::Memo(i) => i.try_get(),
                    SignalTypes::DerivedSignal(i) => Some(i()),
                })
                .flatten()
        }
    }

    impl<T> Signal<T>
    where
        T: Send + Sync + 'static,
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
}
