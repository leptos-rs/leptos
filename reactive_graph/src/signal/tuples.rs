use crate::traits::{
    DefinedAt, Dispose, IsDisposed, Notify, ReadUntracked, Set, Track,
};
use std::{fmt, ops::Deref, panic::Location, sync::Arc};

const DEFINED_AT_MSG: &str = "`DefinedAt::defined_at` called on a tuple. Only \
                              the first value will be reported.";
const PARTIAL_SET_MSG: &str =
    "Tried to fallibly set a tuple, but only some values succeeded.";

macro_rules! impl_tuple {
    ($($T:ident, $N:tt,)*) => {
        impl<$($T,)*> Dispose for ($($T,)*)
        where
            $($T: Dispose,)*
        {
            fn dispose(self) {
                $(
                    self.$N.dispose();
                )*
            }
        }

        impl<$($T,)*> Track for ($($T,)*)
        where
            $($T: Track,)*
        {
            fn track(&self) {
                $(
                    self.$N.track();
                )*
            }
        }

        impl<$($T,)*> DefinedAt for ($($T,)*)
        where
            $($T: DefinedAt,)*
        {
            fn defined_at(&self) -> Option<&'static Location<'static>> {
                let locs = [$(self.$N.defined_at()),*];

                crate::log_warning(format_args!(
                    "{DEFINED_AT_MSG} All locations: [{}]",
                    MaybeLocations { locs: &locs },
                ));

                locs[0]
            }
        }

        impl<$($T,)*> ReadUntracked for ($($T,)*)
        where
            $($T: ReadUntracked,)*
        {
            type Value = TupleReadWrapper<($(TupleReadFieldWrapper<$T>,)*)>;

            fn try_read_untracked(&self) -> Option<Self::Value> {
                Some(TupleReadWrapper((
                    $(TupleReadFieldWrapper::new(&self.$N)?,)*
                )))
            }
        }

        impl<$($T,)*> Notify for ($($T,)*)
        where
            $($T: Notify,)*
        {
            fn notify(&self) {
                $(
                    self.$N.notify();
                )*
            }
        }

        impl<$($T,)*> Set for ($($T,)*)
        where
            $($T: Set,)*
        {
            type Value = ($($T::Value,)*);

            fn set(&self, value: Self::Value) {
                $(
                    self.$N.set(value.$N);
                )*
            }

            fn try_set(&self, value: Self::Value) -> Option<Self::Value> {
                let values = ($(
                    self.$N.try_set(value.$N),
                )*);

                let all_none = $(values.$N.is_none() &&)* true;
                let all_some = $(values.$N.is_some() &&)* true;
                assert!(all_none || all_some, "{PARTIAL_SET_MSG}");

                Some(($(values.$N?,)*))
            }
        }

        impl<$($T,)*> IsDisposed for ($($T,)*)
        where
            $($T: IsDisposed,)*
        {
            fn is_disposed(&self) -> bool {
                $(
                    self.$N.is_disposed() &&
                )*
                true
            }
        }
    };
}

#[rustfmt::skip]
mod impl_tuples {
    use super::*;

    impl_tuple!(T0, 0,);
    impl_tuple!(T0, 0, T1, 1,);
    impl_tuple!(T0, 0, T1, 1, T2, 2,);
    impl_tuple!(T0, 0, T1, 1, T2, 2, T3, 3,);
    impl_tuple!(T0, 0, T1, 1, T2, 2, T3, 3, T4, 4,);
    impl_tuple!(T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5,);
    impl_tuple!(T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6,);
    impl_tuple!(T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6, T7, 7,);
    impl_tuple!(T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6, T7, 7, T8, 8,);
    impl_tuple!(T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6, T7, 7, T8, 8, T9, 9,);
    impl_tuple!(T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6, T7, 7, T8, 8, T9, 9, T10, 10,);
    impl_tuple!(T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6, T7, 7, T8, 8, T9, 9, T10, 10, T11, 11,);
    impl_tuple!(T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6, T7, 7, T8, 8, T9, 9, T10, 10, T11, 11, T12, 12,);
    impl_tuple!(T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6, T7, 7, T8, 8, T9, 9, T10, 10, T11, 11, T12, 12, T13, 13,);
    impl_tuple!(T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6, T7, 7, T8, 8, T9, 9, T10, 10, T11, 11, T12, 12, T13, 13, T14, 14,);
    impl_tuple!(T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6, T7, 7, T8, 8, T9, 9, T10, 10, T11, 11, T12, 12, T13, 13, T14, 14, T15, 15,);
}

/// A wrapper around a tuple that simply [Deref]s to the tuple.
/// This is needed because of the [Deref] bound on [ReadUntracked::Value].
pub struct TupleReadWrapper<T>(T);
impl<T> Deref for TupleReadWrapper<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<T: fmt::Debug> fmt::Debug for TupleReadWrapper<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("TupleReadWrapper").finish()
    }
}

/// A wrapper around an [ReadUntracked::Value] that allows us to implement [Clone]
/// on the [Deref] guard and sattisfy the blanket implementation of
/// [GetUntracked](crate::traits::GetUntracked).
pub struct TupleReadFieldWrapper<T: ReadUntracked> {
    value: Arc<T::Value>,
}
impl<T: ReadUntracked> Deref for TupleReadFieldWrapper<T> {
    type Target = <T::Value as Deref>::Target;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}
impl<T: ReadUntracked> TupleReadFieldWrapper<T> {
    fn new(signal: &T) -> Option<Self> {
        let value = signal.try_read_untracked()?;
        Some(Self {
            value: Arc::new(value),
        })
    }
}
impl<T: ReadUntracked> fmt::Debug for TupleReadFieldWrapper<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("TupleReadFieldWrapper").finish()
    }
}
impl<T: ReadUntracked + Clone> Clone for TupleReadFieldWrapper<T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
        }
    }
}

struct MaybeLocations<'a, const N: usize> {
    locs: &'a [Option<&'static Location<'static>>; N],
}
impl<const N: usize> fmt::Display for MaybeLocations<'_, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for loc in self.locs {
            if let Some(loc) = loc {
                write!(f, "{}", loc)?;
            } else {
                f.write_str("unknown")?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        signal::ArcRwSignal,
        traits::{GetUntracked, WithUntracked},
    };

    #[test]
    fn compile_test() {
        let a = ArcRwSignal::new(1);
        let b = ArcRwSignal::new(2);
        {
            let tuple = (a.clone(), b.clone()).read_untracked();
            assert_eq!(*tuple.0 .0, 1); // .0 is ambiguous in this file only as it's not public.
            assert_eq!(*tuple.1, 2);
        }
        {
            let (a, b) = (a.clone(), b.clone()).get_untracked();
            assert_eq!(*a, 1);
            assert_eq!(*b, 2);
        }

        (a.clone(), b.clone()).with_untracked(
            |(a, b): &(
                TupleReadFieldWrapper<ArcRwSignal<i32>>,
                TupleReadFieldWrapper<ArcRwSignal<i32>>,
            )| {
                assert_eq!(**a, 1);
                assert_eq!(**b, 2);
            },
        );
    }
}
