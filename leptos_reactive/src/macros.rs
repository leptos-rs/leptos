macro_rules! debug_warn {
    ($($x:tt)*) => {
        {
            #[cfg(debug_assertions)]
            {
                ($crate::console_warn(&format_args!($($x)*).to_string()))
            }
            #[cfg(not(debug_assertions))]
            {
                ($($x)*)
            }
        }
    }
}

pub(crate) use debug_warn;

/// Provides a simpler way to use [`SignalWith::with`](crate::SignalWith::with).
///
/// This macro also supports [stored values](crate::StoredValue). If you would
/// like to distinguish between the two, you can also use [`with_value`](crate::with_value)
/// for stored values only.
///
/// The general syntax looks like:
/// ```ignore
/// with!(|capture1, capture2, ...| body);
/// ```
/// The variables within the 'closure' arguments are captured from the
/// environment, and can be used within the body with the same name.
///
/// `move` can also be added before the closure arguments to add `move` to all
/// expanded closures.
///
/// # Examples
/// ```
/// # use leptos_reactive::*;
/// # let runtime = create_runtime();
/// let (first, _) = create_signal("Bob".to_string());
/// let (middle, _) = create_signal("J.".to_string());
/// let (last, _) = create_signal("Smith".to_string());
/// let name = with!(|first, middle, last| format!("{first} {middle} {last}"));
/// assert_eq!(name, "Bob J. Smith");
/// # runtime.dispose();
/// ```
///
/// The `with!` macro in the above example expands to:
/// ```ignore
/// first.with(|first| {
///     middle.with(|middle| {
///         last.with(|last| format!("{first} {middle} {last}"))
///     })
/// })
/// ```
///
/// If `move` is added:
/// ```ignore
/// with!(move |first, last| format!("{first} {last}"))
/// ```
///
/// Then all closures are also `move`.
/// ```ignore
/// first.with(move |first| {
///     last.with(move |last| format!("{first} {last}"))
/// })
/// ```
#[macro_export]
macro_rules! with {
    (|$ident:ident $(,)?| $body:expr) => {
        $crate::macros::__private::Withable::call_with(&$ident, |$ident| $body)
    };
    (move |$ident:ident $(,)?| $body:expr) => {
        $crate::macros::__private::Withable::call_with(&$ident, move |$ident| $body)
    };
    (|$first:ident, $($rest:ident),+ $(,)? | $body:expr) => {
        $crate::macros::__private::Withable::call_with(
            &$first,
            |$first| with!(|$($rest),+| $body)
        )
    };
    (move |$first:ident, $($rest:ident),+ $(,)? | $body:expr) => {
        $crate::macros::__private::Withable::call_with(
            &$first,
            move |$first| with!(|$($rest),+| $body)
        )
    };
}

/// Provides a simpler way to use
/// [`StoredValue::with_value`](crate::StoredValue::with_value).
///
/// To use with [signals](crate::SignalWith::with), see the [`with!`] macro
/// instead.
///
/// Note that the [`with!`] macro also works with
/// [`StoredValue`](crate::StoredValue). Use this macro if you would like to
/// distinguish between signals and stored values.
///
/// The general syntax looks like:
/// ```ignore
/// with_value!(|capture1, capture2, ...| body);
/// ```
/// The variables within the 'closure' arguments are captured from the
/// environment, and can be used within the body with the same name.
///
/// `move` can also be added before the closure arguments to add `move` to all
/// expanded closures.
///
/// # Examples
/// ```
/// # use leptos_reactive::*;
/// # let runtime = create_runtime();
/// let first = store_value("Bob".to_string());
/// let middle = store_value("J.".to_string());
/// let last = store_value("Smith".to_string());
/// let name = with_value!(|first, middle, last| {
///     format!("{first} {middle} {last}")
/// });
/// assert_eq!(name, "Bob J. Smith");
/// # runtime.dispose();
/// ```
/// The `with_value!` macro in the above example expands to:
/// ```ignore
/// first.with_value(|first| {
///     middle.with_value(|middle| {
///         last.with_value(|last| format!("{first} {middle} {last}"))
///     })
/// })
/// ```
///
/// If `move` is added:
/// ```ignore
/// with_value!(move |first, last| format!("{first} {last}"))
/// ```
///
/// Then all closures are also `move`.
/// ```ignore
/// first.with_value(move |first| {
///     last.with_value(move |last| format!("{first} {last}"))
/// })
/// ```
#[macro_export]
macro_rules! with_value {
    (|$ident:ident $(,)?| $body:expr) => {
        $ident.with_value(|$ident| $body)
    };
    (move |$ident:ident $(,)?| $body:expr) => {
        $ident.with_value(move |$ident| $body)
    };
    (|$first:ident, $($rest:ident),+ $(,)? | $body:expr) => {
        $first.with_value(|$first| with_value!(|$($rest),+| $body))
    };
    (move |$first:ident, $($rest:ident),+ $(,)? | $body:expr) => {
        $first.with_value(move |$first| with_value!(move |$($rest),+| $body))
    };
}

/// Provides a simpler way to use
/// [`SignalUpdate::update`](crate::SignalUpdate::update).
///
/// This macro also supports [stored values](crate::StoredValue). If you would
/// like to distinguish between the two, you can also use [`update_value`](crate::update_value)
/// for stored values only.
///
/// The general syntax looks like:
/// ```ignore
/// update!(|capture1, capture2, ...| body);
/// ```
/// The variables within the 'closure' arguments are captured from the
/// environment, and can be used within the body with the same name.
///
/// `move` can also be added before the closure arguments to add `move` to all
/// expanded closures.
///
/// # Examples
/// ```
/// # use leptos_reactive::*;
/// # let runtime = create_runtime();
/// let a = create_rw_signal(1);
/// let b = create_rw_signal(2);
/// update!(|a, b| *a = *a + *b);
/// assert_eq!(a.get(), 3);
/// # runtime.dispose();
/// ```
/// The `update!` macro in the above example expands to:
/// ```ignore
/// a.update(|a| {
///     b.update(|b| *a = *a + *b)
/// })
/// ```
///
/// If `move` is added:
/// ```ignore
/// update!(move |a, b| *a = *a + *b + something_else)
/// ```
///
/// Then all closures are also `move`.
/// ```ignore
/// first.update(move |a| {
///     last.update(move |b| *a = *a + *b + something_else)
/// })
/// ```
#[macro_export]
macro_rules! update {
    (|$ident:ident $(,)?| $body:expr) => {
        $crate::macros::__private::Updatable::call_update(&$ident, |$ident| $body)
    };
    (move |$ident:ident $(,)?| $body:expr) => {
        $crate::macros::__private::Updatable::call_update(&$ident, move |$ident| $body)
    };
    (|$first:ident, $($rest:ident),+ $(,)? | $body:expr) => {
        $crate::macros::__private::Updatable::call_update(
            &$first,
            |$first| update!(|$($rest),+| $body)
        )
    };
    (move |$first:ident, $($rest:ident),+ $(,)? | $body:expr) => {
        $crate::macros::__private::Updatable::call_update(
            &$first,
            move |$first| update!(|$($rest),+| $body)
        )
    };
}

/// Provides a simpler way to use
/// [`StoredValue::update_value`](crate::StoredValue::update_value).
///
/// To use with [signals](crate::SignalUpdate::update), see the [`update`]
/// macro instead.
///
/// Note that the [`update!`] macro also works with
/// [`StoredValue`](crate::StoredValue). Use this macro if you would like to
/// distinguish between signals and stored values.
///
/// The general syntax looks like:
/// ```ignore
/// update_value!(|capture1, capture2, ...| body);
/// ```
/// The variables within the 'closure' arguments are captured from the
/// environment, and can be used within the body with the same name.
///
/// `move` can also be added before the closure arguments to add `move` to all
/// expanded closures.
///
/// # Examples
/// ```
/// # use leptos_reactive::*;
/// # let runtime = create_runtime();
/// let a = store_value(1);
/// let b = store_value(2);
/// update_value!(|a, b| *a = *a + *b);
/// assert_eq!(a.get_value(), 3);
/// # runtime.dispose();
/// ```
/// The `update_value!` macro in the above example expands to:
/// ```ignore
/// a.update_value(|a| {
///     b.update_value(|b| *a = *a + *b)
/// })
/// ```
/// If `move` is added:
/// ```ignore
/// update_value!(move |a, b| *a = *a + *b + something_else)
/// ```
///
/// Then all closures are also `move`.
/// ```ignore
/// first.update_value(move |a| {
///     last.update_value(move |b| *a = *a + *b + something_else)
/// })
/// ```
#[macro_export]
macro_rules! update_value {
    (|$ident:ident $(,)?| $body:expr) => {
        $ident.update_value(|$ident| $body)
    };
    (move |$ident:ident $(,)?| $body:expr) => {
        $ident.update_value(move |$ident| $body)
    };
    (|$first:ident, $($rest:ident),+ $(,)? | $body:expr) => {
        $first.update_value(|$first| update_value!(|$($rest),+| $body))
    };
    (move |$first:ident, $($rest:ident),+ $(,)? | $body:expr) => {
        $first.update_value(move |$first| update_value!(move |$($rest),+| $body))
    };
}

/// This is a private module intended to only be used by macros. Do not access
/// this directly!
#[doc(hidden)]
pub mod __private {
    use crate::{SignalUpdate, SignalWith, StoredValue};

    pub trait Withable {
        type Value;

        // don't use `&self` or r-a will suggest importing this trait
        // and using it as a method
        #[track_caller]
        fn call_with<O>(item: &Self, f: impl FnOnce(&Self::Value) -> O) -> O;
    }

    impl<T> Withable for StoredValue<T> {
        type Value = T;

        #[inline(always)]
        fn call_with<O>(item: &Self, f: impl FnOnce(&Self::Value) -> O) -> O {
            item.with_value(f)
        }
    }

    impl<S: SignalWith> Withable for S {
        type Value = S::Value;

        #[inline(always)]
        fn call_with<O>(item: &Self, f: impl FnOnce(&Self::Value) -> O) -> O {
            item.with(f)
        }
    }

    pub trait Updatable {
        type Value;

        #[track_caller]
        fn call_update(item: &Self, f: impl FnOnce(&mut Self::Value));
    }

    impl<T> Updatable for StoredValue<T> {
        type Value = T;

        #[inline(always)]
        fn call_update(item: &Self, f: impl FnOnce(&mut Self::Value)) {
            item.update_value(f)
        }
    }

    impl<S: SignalUpdate> Updatable for S {
        type Value = S::Value;

        #[inline(always)]
        fn call_update(item: &Self, f: impl FnOnce(&mut Self::Value)) {
            item.update(f)
        }
    }
}
