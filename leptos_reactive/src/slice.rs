use crate::{create_memo, IntoSignalSetter, RwSignal, Scope, Signal, SignalSetter};

/// create a slice of a Signal
///
///
/// ```
/// # use leptos_reactive::*;
///
/// # let (cx, disposer) = raw_scope_and_disposer(create_runtime());
///
/// pub struct Stuff {
///     inner: String,
///     other: i32,
/// }
///
/// let complex = create_rw_signal(
///     cx,
///     Stuff {
///         inner: "".into(),
///         other: 0,
///     },
/// );
/// let (inner, set_inner) = create_slice(cx, complex, |c| c.inner.clone(), |c, s| c.inner = s);
/// let logs = create_rw_signal(cx, 0);
/// logs.with(|logs| assert_eq!(logs, &0));
/// create_effect(cx, move |_| { inner.with(|inner| {}); logs.update(|logs| *logs += 1) });
/// logs.with(|logs| assert_eq!(logs, &1));
/// set_inner.set("Hello World".into());
/// logs.with(|logs| assert_eq!(logs, &2));
/// complex.update(|complex| complex.other = 10);
/// logs.with(|logs| assert_eq!(logs, &2));
/// ```
pub fn create_slice<T, O, G, S>(
    cx: Scope,
    signal: RwSignal<T>,
    getter: G,
    setter: S,
) -> (Signal<O>, SignalSetter<O>)
where
    G: Fn(&T) -> O + Clone + Copy + 'static,
    S: Fn(&mut T, O) + Clone + Copy + 'static,
    O: Eq,
{
    let getter = create_memo(cx, move |_| signal.with(getter));
    let setter = move |value| signal.update(|x| setter(x, value));
    (getter.into(), setter.mapped_signal_setter(cx))
}
