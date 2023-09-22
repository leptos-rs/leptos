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
/// To use with [stored values](crate::StoredValue), see the [`with_value`](crate::with_value)
/// macro instead.
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
/// # use leptos::*;
/// # let runtime = create_runtime();
/// # if !cfg!(any(feature = "csr", feature = "hydrate")) {
/// let (first, _) = create_signal("Bob".to_string());
/// let (middle, _) = create_signal("J.".to_string());
/// let (last, _) = create_signal("Smith".to_string());
/// let name = with!(|first, middle, last| format!("{first} {middle} {last}"));
/// assert_eq!(name, "Bob J. Smith");
/// # };
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
        $ident.with(|$ident| $body)
    };
    (move |$ident:ident $(,)?| $body:expr) => {
        $ident.with(move |$ident| $body)
    };
    (|$first:ident, $($rest:ident),+ $(,)? | $body:expr) => {
        $first.with(|$first| with!(|$($rest),+| $body))
    };
    (move |$first:ident, $($rest:ident),+ $(,)? | $body:expr) => {
        $first.with(move |$first| with!(move |$($rest),+| $body))
    };
}

/// Provides a simpler way to use
/// [`StoredValue::with_value`](crate::StoredValue::with_value).
///
/// To use with [signals](crate::SignalWith::with), see the [`with!`] macro
/// instead.
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
/// # use leptos::*;
/// # let runtime = create_runtime();
/// # if !cfg!(any(feature = "csr", feature = "hydrate")) {
/// let first = store_value("Bob".to_string());
/// let middle = store_value("J.".to_string());
/// let last = store_value("Smith".to_string());
/// let name = with_value!(|first, middle, last| {
///     format!("{first} {middle} {last}")
/// });
/// assert_eq!(name, "Bob J. Smith");
/// # };
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
/// To use with [stored values](crate::StoredValue), see the [`update_value`](crate::update_value)
/// macro instead.
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
/// # use leptos::*;
/// # let runtime = create_runtime();
/// # if !cfg!(any(feature = "csr", feature = "hydrate")) {
/// let a = create_rw_signal(1);
/// let b = create_rw_signal(2);
/// update!(|a, b| *a = *a + *b);
/// assert_eq!(a.get(), 3);
/// # };
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
        $ident.update(|$ident| $body)
    };
    (move |$ident:ident $(,)?| $body:expr) => {
        $ident.update(move |$ident| $body)
    };
    (|$first:ident, $($rest:ident),+ $(,)? | $body:expr) => {
        $first.update(|$first| update!(|$($rest),+| $body))
    };
    (move |$first:ident, $($rest:ident),+ $(,)? | $body:expr) => {
        $first.update(move |$first| update!(move |$($rest),+| $body))
    };
}

/// Provides a simpler way to use
/// [`StoredValue::update_value`](crate::StoredValue::update_value).
///
/// To use with [signals](crate::SignalUpdate::update), see the [`update`]
/// macro instead.
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
/// # use leptos::*;
/// # let runtime = create_runtime();
/// # if !cfg!(any(feature = "csr", feature = "hydrate")) {
/// let a = store_value(1);
/// let b = store_value(2);
/// update_value!(|a, b| *a = *a + *b);
/// assert_eq!(a.get_value(), 3);
/// # };
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
