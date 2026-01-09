#[allow(deprecated)]
use crate::wrappers::read::{MaybeProp, MaybeSignal};
use crate::{
    computed::{ArcMemo, Memo},
    owner::Storage,
    signal::{
        ArcReadSignal, ArcRwSignal, ArcWriteSignal, ReadSignal, RwSignal,
        WriteSignal,
    },
    traits::{Get, Set},
    wrappers::{
        read::{ArcSignal, Signal, SignalTypes},
        write::SignalSetter,
    },
};

macro_rules! impl_set_fn_traits {
    ($($ty:ident),*) => {
        $(
            #[cfg(feature = "nightly")]
            impl<T> FnOnce<(T,)> for $ty<T> where $ty<T>: Set<Value = T> {
                type Output = ();

                #[inline(always)]
                extern "rust-call" fn call_once(self, args: (T,)) -> Self::Output {
                    self.set(args.0);
                }
            }

            #[cfg(feature = "nightly")]
            impl<T> FnMut<(T,)> for $ty<T> where $ty<T>: Set<Value = T> {
                #[inline(always)]
                extern "rust-call" fn call_mut(&mut self, args: (T,)) -> Self::Output {
                    self.set(args.0);
                }
            }

            #[cfg(feature = "nightly")]
            impl<T> Fn<(T,)> for $ty<T> where $ty<T>: Set<Value = T> {
                #[inline(always)]
                extern "rust-call" fn call(&self, args: (T,)) -> Self::Output {
                    self.set(args.0);
                }
            }
        )*
    };
}

macro_rules! impl_set_fn_traits_arena {
    ($($ty:ident),*) => {
        $(
            #[cfg(feature = "nightly")]
            impl<T, S> FnOnce<(T,)> for $ty<T, S> where $ty<T, S>: Set<Value = T> {
                type Output = ();

                #[inline(always)]
                extern "rust-call" fn call_once(self, args: (T,)) -> Self::Output {
                    self.set(args.0);
                }
            }

            #[cfg(feature = "nightly")]
            impl<T, S> FnMut<(T,)> for $ty<T, S> where $ty<T, S>: Set<Value = T> {
                #[inline(always)]
                extern "rust-call" fn call_mut(&mut self, args: (T,)) -> Self::Output {
                    self.set(args.0);
                }
            }

            #[cfg(feature = "nightly")]
            impl<T, S> Fn<(T,)> for $ty<T, S> where $ty<T, S>: Set<Value = T> {
                #[inline(always)]
                extern "rust-call" fn call(&self, args: (T,)) -> Self::Output {
                    self.set(args.0);
                }
            }
        )*
    };
}

macro_rules! impl_get_fn_traits_get {
    ($($ty:ident),*) => {
        $(
            #[cfg(feature = "nightly")]
            impl<T> FnOnce<()> for $ty<T> where $ty<T>: Get {
                type Output = <Self as Get>::Value;

                #[inline(always)]
                extern "rust-call" fn call_once(self, _args: ()) -> Self::Output {
                    self.get()
                }
            }

            #[cfg(feature = "nightly")]
            impl<T> FnMut<()> for $ty<T> where $ty<T>: Get {
                #[inline(always)]
                extern "rust-call" fn call_mut(&mut self, _args: ()) -> Self::Output {
                    self.get()
                }
            }

            #[cfg(feature = "nightly")]
            impl<T> Fn<()> for $ty<T> where $ty<T>: Get {
                #[inline(always)]
                extern "rust-call" fn call(&self, _args: ()) -> Self::Output {
                    self.get()
                }
            }
        )*
    };
}

macro_rules! impl_get_fn_traits_get_arena {
    ($($ty:ident),*) => {
        $(
            #[cfg(feature = "nightly")]
            #[allow(deprecated)]
            impl<T, S> FnOnce<()> for $ty<T, S> where $ty<T, S>: Get, S: Storage<T> + Storage<Option<T>> + Storage<SignalTypes<Option<T>, S>> {
                type Output = <Self as Get>::Value;

                #[inline(always)]
                extern "rust-call" fn call_once(self, _args: ()) -> Self::Output {
                    self.get()
                }
            }

            #[cfg(feature = "nightly")]
            #[allow(deprecated)]
            impl<T, S> FnMut<()> for $ty<T, S> where $ty<T, S>: Get, S: Storage<T> + Storage<Option<T>> + Storage<SignalTypes<Option<T>, S>> {
                #[inline(always)]
                extern "rust-call" fn call_mut(&mut self, _args: ()) -> Self::Output {
                    self.get()
                }
            }

            #[cfg(feature = "nightly")]
            #[allow(deprecated)]
            impl<T, S> Fn<()> for $ty<T, S> where $ty<T, S>: Get, S: Storage<T> + Storage<Option<T>> + Storage<SignalTypes<Option<T>, S>> {
                #[inline(always)]
                extern "rust-call" fn call(&self, _args: ()) -> Self::Output {
                    self.get()
                }
            }
        )*
    };
}

macro_rules! impl_get_fn_traits_get_arena_with_readable_deref_impl {
    ($($ty:ident),*) => {
        $(
            /// Allow calling $ty() syntax
            impl<T: Clone + 'static, S: Storage<T> + 'static> std::ops::Deref
                for $ty<T, S>
            where
                $ty<T, S>: crate::traits::Get<Value = T>,
                $ty<T, S>: Get, S: Storage<T> + Storage<Option<T>> + Storage<SignalTypes<Option<T>, S>>
            {
                type Target = dyn Fn() -> T;

                fn deref(&self) -> &Self::Target {
                    unsafe { readable_deref_impl::ReadableDerefImpl::deref_impl(self) }
                }
            }

            impl<T: Clone + 'static, S: Storage<T> + 'static> readable_deref_impl::ReadableDerefImpl for $ty<T, S>
            where
                $ty<T, S>: crate::traits::Get<Value = T>,
                $ty<T, S>: Get, S: Storage<T> + Storage<Option<T>> + Storage<SignalTypes<Option<T>, S>>
            {
            }

        )*
    };
}

impl_get_fn_traits_get![ArcReadSignal, ArcRwSignal];
impl_get_fn_traits_get_arena![
    ReadSignal,
    RwSignal,
    ArcMemo,
    MaybeSignal,
    Memo,
    MaybeProp
];
impl_get_fn_traits_get_arena_with_readable_deref_impl![ArcSignal, Signal];
impl_set_fn_traits![ArcRwSignal, ArcWriteSignal];
impl_set_fn_traits_arena![RwSignal, WriteSignal, SignalSetter];

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
