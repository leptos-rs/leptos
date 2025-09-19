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

impl_get_fn_traits_get![ArcReadSignal, ArcRwSignal];
impl_get_fn_traits_get_arena![
    ReadSignal,
    RwSignal,
    ArcMemo,
    ArcSignal,
    Signal,
    MaybeSignal,
    Memo,
    MaybeProp
];
impl_set_fn_traits![ArcRwSignal, ArcWriteSignal];
impl_set_fn_traits_arena![RwSignal, WriteSignal, SignalSetter];
