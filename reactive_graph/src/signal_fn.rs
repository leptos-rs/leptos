#[allow(deprecated)]
use crate::wrappers::read::{MaybeProp, MaybeSignal};
use crate::{
    computed::{ArcMemo, Memo},
    owner::Storage,
    signal::{
        ArcReadSignal, ArcRwSignal, ArcWriteSignal, ReadSignal, RwSignal,
        WriteSignal,
    },
    wrappers::{
        read::{ArcSignal, Signal, SignalTypes},
        write::SignalSetter,
    },
};

mod readable_deref_impl {
    /// Derived from the implementation and original creaters at dioxus
    /// https://docs.rs/dioxus-signals/0.6.3/src/dioxus_signals/signal.rs.html#485-494
    pub trait ReadableDerefImpl: crate::traits::Get {
        /// SAFETY: You must call this function directly with `self` as the argument.
        /// This function relies on the size of the object you return from the deref
        /// being the same as the object you pass in
        #[doc(hidden)]
        unsafe fn deref_impl<'a>(&self) -> &'a dyn Fn() -> Self::Value
        where
            Self: Sized + 'a,
            Self::Value: Clone + 'static,
        {
            // https://github.com/dtolnay/case-studies/tree/master/callable-types

            // First we create a closure that captures something with the Same in memory layout as Self (MaybeUninit<Self>).
            let uninit_callable = std::mem::MaybeUninit::<Self>::uninit();
            // Then move that value into the closure. We assume that the closure now has a in memory layout of Self.
            let uninit_closure =
                move || Self::get(unsafe { &*uninit_callable.as_ptr() });

            // Check that the size of the closure is the same as the size of Self in case the compiler changed the layout of the closure.
            let size_of_closure = std::mem::size_of_val(&uninit_closure);
            assert_eq!(size_of_closure, std::mem::size_of::<Self>());

            // Then cast the lifetime of the closure to the lifetime of &self.
            fn cast_lifetime<'a, T>(_a: &T, b: &'a T) -> &'a T {
                b
            }
            let reference_to_closure = cast_lifetime(
                {
                    // The real closure that we will never use.
                    &uninit_closure
                },
                #[allow(clippy::missing_transmute_annotations)]
                // We transmute self into a reference to the closure. This is safe because we know that the closure has the same memory layout as Self so &Closure == &Self.
                unsafe {
                    std::mem::transmute(self)
                },
            );

            // Cast the closure to a trait object.
            reference_to_closure as &_
        }
    }
}

mod writable_deref_impl {
    /// Analogous to ReadableDerefImpl, but for signal types that implement Set.
    pub trait WritableDerefImpl: crate::traits::Set {
        /// SAFETY: You must call this function directly with `self` as the argument.
        /// This function relies on the size of the object you return from the deref
        /// being the same as the object you pass in
        #[doc(hidden)]
        unsafe fn deref_impl<'a>(&self) -> &'a dyn Fn(Self::Value)
        where
            Self: Sized + 'a,
        {
            let uninit_callable = std::mem::MaybeUninit::<Self>::uninit();
            let uninit_closure = move |value| {
                Self::set(unsafe { &*uninit_callable.as_ptr() }, value);
            };

            let size_of_closure = std::mem::size_of_val(&uninit_closure);
            assert_eq!(size_of_closure, std::mem::size_of::<Self>());

            fn cast_lifetime<'a, T>(_a: &T, b: &'a T) -> &'a T {
                b
            }
            let reference_to_closure = cast_lifetime(
                &uninit_closure,
                #[allow(clippy::missing_transmute_annotations)]
                unsafe {
                    std::mem::transmute(self)
                },
            );

            reference_to_closure as &_
        }
    }
}

// =============================================================================
// Read-side Deref impls: Deref<Target = dyn Fn() -> T>
// =============================================================================

/// Impl Deref for Arc signal types (no S generic)
macro_rules! impl_readable_deref_arc {
    ($($ty:ident),*) => {
        $(
            impl<T: Clone + 'static> std::ops::Deref for $ty<T>
            where
                $ty<T>: crate::traits::Get<Value = T>,
            {
                type Target = dyn Fn() -> T;

                fn deref(&self) -> &Self::Target {
                    unsafe { readable_deref_impl::ReadableDerefImpl::deref_impl(self) }
                }
            }

            impl<T: Clone + 'static> readable_deref_impl::ReadableDerefImpl for $ty<T>
            where
                $ty<T>: crate::traits::Get<Value = T>,
            {
            }
        )*
    };
}

/// Impl Deref for arena signal types (with S generic)
macro_rules! impl_readable_deref_arena {
    ($($ty:ident),*) => {
        $(
            impl<T: Clone + 'static, S: Storage<T> + 'static> std::ops::Deref for $ty<T, S>
            where
                $ty<T, S>: crate::traits::Get<Value = T>,
            {
                type Target = dyn Fn() -> T;

                fn deref(&self) -> &Self::Target {
                    unsafe { readable_deref_impl::ReadableDerefImpl::deref_impl(self) }
                }
            }

            impl<T: Clone + 'static, S: Storage<T> + 'static> readable_deref_impl::ReadableDerefImpl for $ty<T, S>
            where
                $ty<T, S>: crate::traits::Get<Value = T>,
            {
            }
        )*
    };
}

/// Impl Deref for arena signal types that require SignalTypes storage bounds
macro_rules! impl_readable_deref_arena_signal_types {
    ($($ty:ident),*) => {
        $(
            #[allow(deprecated)]
            impl<T: Clone + 'static, S: Storage<T> + Storage<Option<T>> + Storage<SignalTypes<Option<T>, S>> + 'static> std::ops::Deref
                for $ty<T, S>
            where
                $ty<T, S>: crate::traits::Get<Value = T>,
            {
                type Target = dyn Fn() -> T;

                fn deref(&self) -> &Self::Target {
                    unsafe { readable_deref_impl::ReadableDerefImpl::deref_impl(self) }
                }
            }

            #[allow(deprecated)]
            impl<T: Clone + 'static, S: Storage<T> + Storage<Option<T>> + Storage<SignalTypes<Option<T>, S>> + 'static> readable_deref_impl::ReadableDerefImpl
                for $ty<T, S>
            where
                $ty<T, S>: crate::traits::Get<Value = T>,
            {
            }
        )*
    };
}

impl_readable_deref_arc![ArcReadSignal, ArcRwSignal];
impl_readable_deref_arena![ReadSignal, RwSignal, Memo];
impl_readable_deref_arena_signal_types![Signal, ArcSignal, ArcMemo, MaybeSignal, MaybeProp];

// =============================================================================
// Write-side Deref impls: Deref<Target = dyn Fn(T)>
// =============================================================================

/// Impl Deref for Arc writable signal types (no S generic)
macro_rules! impl_writable_deref_arc {
    ($($ty:ident),*) => {
        $(
            impl<T: 'static> std::ops::Deref for $ty<T>
            where
                $ty<T>: crate::traits::Set<Value = T>,
            {
                type Target = dyn Fn(T);

                fn deref(&self) -> &Self::Target {
                    unsafe { writable_deref_impl::WritableDerefImpl::deref_impl(self) }
                }
            }

            impl<T: 'static> writable_deref_impl::WritableDerefImpl for $ty<T>
            where
                $ty<T>: crate::traits::Set<Value = T>,
            {
            }
        )*
    };
}

impl_writable_deref_arc![ArcWriteSignal];

// WriteSignal: needs Storage<ArcWriteSignal<T>>
impl<T: 'static, S: Storage<ArcWriteSignal<T>> + 'static> std::ops::Deref for WriteSignal<T, S>
where
    WriteSignal<T, S>: crate::traits::Set<Value = T>,
{
    type Target = dyn Fn(T);

    fn deref(&self) -> &Self::Target {
        unsafe { writable_deref_impl::WritableDerefImpl::deref_impl(self) }
    }
}

impl<T: 'static, S: Storage<ArcWriteSignal<T>> + 'static> writable_deref_impl::WritableDerefImpl for WriteSignal<T, S>
where
    WriteSignal<T, S>: crate::traits::Set<Value = T>,
{
}

// SignalSetter: needs Storage<ArcWriteSignal<T>> + Storage<Box<dyn Fn(T) + Send + Sync>>
impl<T: 'static, S: Storage<ArcWriteSignal<T>> + Storage<Box<dyn Fn(T) + Send + Sync>> + 'static> std::ops::Deref for SignalSetter<T, S>
where
    SignalSetter<T, S>: crate::traits::Set<Value = T>,
{
    type Target = dyn Fn(T);

    fn deref(&self) -> &Self::Target {
        unsafe { writable_deref_impl::WritableDerefImpl::deref_impl(self) }
    }
}

impl<T: 'static, S: Storage<ArcWriteSignal<T>> + Storage<Box<dyn Fn(T) + Send + Sync>> + 'static> writable_deref_impl::WritableDerefImpl for SignalSetter<T, S>
where
    SignalSetter<T, S>: crate::traits::Set<Value = T>,
{
}
