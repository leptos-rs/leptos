use crate::{
    computed::{ArcMemo, Memo},
    signal::{
        ArcReadSignal, ArcRwSignal, ArcWriteSignal, ReadSignal, RwSignal,
        WriteSignal,
    },
    traits::{Get, Read, Set},
    wrappers::read::{ArcSignal, Signal},
};

macro_rules! impl_get_fn_traits_read {
    ($($ty:ident $(($method_name:ident))?),*) => {
        $(
            #[cfg(feature = "nightly")]
            impl<T: 'static> FnOnce<()> for $ty<T> {
                type Output = <Self as Read>::Value;

                #[inline(always)]
                extern "rust-call" fn call_once(self, _args: ()) -> Self::Output {
                    impl_get_fn_traits_read!(@method_name self $($method_name)?)
                }
            }

            #[cfg(feature = "nightly")]
            impl<T: 'static> FnMut<()> for $ty<T> {
                #[inline(always)]
                extern "rust-call" fn call_mut(&mut self, _args: ()) -> Self::Output {
                    impl_get_fn_traits_read!(@method_name self $($method_name)?)
                }
            }

            #[cfg(feature = "nightly")]
            impl<T: 'static> Fn<()> for $ty<T> {
                #[inline(always)]
                extern "rust-call" fn call(&self, _args: ()) -> Self::Output {
                    impl_get_fn_traits_read!(@method_name self $($method_name)?)
                }
            }
        )*
    };
    (@method_name $self:ident) => {
        $self.read()
    };
    (@method_name $self:ident $ident:ident) => {
        $self.$ident()
    };
}

macro_rules! impl_set_fn_traits {
    ($($ty:ident $($method_name:ident)?),*) => {
        $(
            #[cfg(feature = "nightly")]
            impl<T> FnOnce<(T,)> for $ty<T> {
                type Output = ();

                #[inline(always)]
                extern "rust-call" fn call_once(self, args: (T,)) -> Self::Output {
                    impl_set_fn_traits!(@method_name self $($method_name)? args)
                }
            }

            #[cfg(feature = "nightly")]
            impl<T> FnMut<(T,)> for $ty<T> {
                #[inline(always)]
                extern "rust-call" fn call_mut(&mut self, args: (T,)) -> Self::Output {
                    impl_set_fn_traits!(@method_name self $($method_name)? args)
                }
            }

            #[cfg(feature = "nightly")]
            impl<T> Fn<(T,)> for $ty<T> {
                #[inline(always)]
                extern "rust-call" fn call(&self, args: (T,)) -> Self::Output {
                    impl_set_fn_traits!(@method_name self $($method_name)? args)
                }
            }
        )*
    };
    (@method_name $self:ident $args:ident) => {
        $self.set($args.0)
    };
    (@method_name $self:ident $ident:ident $args:ident) => {
        $self.$ident($args.0)
    };
}

macro_rules! impl_get_fn_traits_read_send {
    ($($ty:ident $(($method_name:ident))?),*) => {
        $(
            #[cfg(feature = "nightly")]
            impl<T: Send + Sync + 'static> FnOnce<()> for $ty<T> {
                type Output = <Self as Read>::Value;

                #[inline(always)]
                extern "rust-call" fn call_once(self, _args: ()) -> Self::Output {
                    impl_get_fn_traits_read_send!(@method_name self $($method_name)?)
                }
            }

            #[cfg(feature = "nightly")]
            impl<T: Send + Sync + 'static> FnMut<()> for $ty<T> {
                #[inline(always)]
                extern "rust-call" fn call_mut(&mut self, _args: ()) -> Self::Output {
                    impl_get_fn_traits_read_send!(@method_name self $($method_name)?)
                }
            }

            #[cfg(feature = "nightly")]
            impl<T: Send + Sync + 'static> Fn<()> for $ty<T> {
                #[inline(always)]
                extern "rust-call" fn call(&self, _args: ()) -> Self::Output {
                    impl_get_fn_traits_read_send!(@method_name self $($method_name)?)
                }
            }
        )*
    };
    (@method_name $self:ident) => {
        $self.read()
    };
    (@method_name $self:ident $ident:ident) => {
        $self.$ident()
    };
}

macro_rules! impl_get_fn_traits_get_send {
    ($($ty:ident $(($method_name:ident))?),*) => {
        $(
            #[cfg(feature = "nightly")]
            impl<T: Send + Sync + Clone + 'static> FnOnce<()> for $ty<T> {
                type Output = <Self as Get>::Value;

                #[inline(always)]
                extern "rust-call" fn call_once(self, _args: ()) -> Self::Output {
                    impl_get_fn_traits_get_send!(@method_name self $($method_name)?)
                }
            }

            #[cfg(feature = "nightly")]
            impl<T: Send + Sync + Clone + 'static> FnMut<()> for $ty<T> {
                #[inline(always)]
                extern "rust-call" fn call_mut(&mut self, _args: ()) -> Self::Output {
                    impl_get_fn_traits_get_send!(@method_name self $($method_name)?)
                }
            }

            #[cfg(feature = "nightly")]
            impl<T: Send + Sync + Clone + 'static> Fn<()> for $ty<T> {
                #[inline(always)]
                extern "rust-call" fn call(&self, _args: ()) -> Self::Output {
                    impl_get_fn_traits_get_send!(@method_name self $($method_name)?)
                }
            }
        )*
    };
    (@method_name $self:ident) => {
        $self.get()
    };
    (@method_name $self:ident $ident:ident) => {
        $self.$ident()
    };
}
macro_rules! impl_set_fn_traits_send {
    ($($ty:ident $($method_name:ident)?),*) => {
        $(
            #[cfg(feature = "nightly")]
            impl<T: Send + Sync + 'static> FnOnce<(T,)> for $ty<T> {
                type Output = ();

                #[inline(always)]
                extern "rust-call" fn call_once(self, args: (T,)) -> Self::Output {
                    impl_set_fn_traits_send!(@method_name self $($method_name)? args)
                }
            }

            #[cfg(feature = "nightly")]
            impl<T: Send + Sync + 'static> FnMut<(T,)> for $ty<T> {
                #[inline(always)]
                extern "rust-call" fn call_mut(&mut self, args: (T,)) -> Self::Output {
                    impl_set_fn_traits_send!(@method_name self $($method_name)? args)
                }
            }

            #[cfg(feature = "nightly")]
            impl<T: Send + Sync + 'static> Fn<(T,)> for $ty<T> {
                #[inline(always)]
                extern "rust-call" fn call(&self, args: (T,)) -> Self::Output {
                    impl_set_fn_traits_send!(@method_name self $($method_name)? args)
                }
            }
        )*
    };
    (@method_name $self:ident $args:ident) => {
        $self.set($args.0)
    };
    (@method_name $self:ident $ident:ident $args:ident) => {
        $self.$ident($args.0)
    };
}

impl_get_fn_traits_read![ArcReadSignal, ArcRwSignal];
impl_get_fn_traits_get_send![ArcSignal, Signal];
impl_get_fn_traits_read_send![ReadSignal, RwSignal, Memo, ArcMemo];
impl_set_fn_traits![ArcWriteSignal];
impl_set_fn_traits_send![WriteSignal];
