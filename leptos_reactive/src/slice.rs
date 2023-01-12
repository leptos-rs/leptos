use crate::{create_memo, IntoSignalSetter, RwSignal, Scope, Signal, SignalSetter};

/// derives a reactive slice from an [RwSignal](crate::RwSignal)
///
/// Slices have the same guarantees as [Memos](crate::Memo),
/// they only emit their value when it has actually been changed.
///
/// slices need a getter and a setter, and you must make sure that
/// the setter and getter only touch their respective field and nothing else.
/// They optimally should not have any side effects.
///
/// you can use slices whenever you want to react to only parts
/// of a bigger signal, the prime example would be state management
/// where you want all state variables grouped up but also need
/// fine-grained signals for each or some of these variables.
/// In the example below, setting an auth token will only trigger
/// the token signal, but none of the other derived signals.
///
/// ```
/// # use leptos_reactive::*;
/// # let (cx, disposer) = raw_scope_and_disposer(create_runtime());
///
/// // this could be serialized to and from localstorage with miniserde
/// pub struct State {
///     token: String,
///     dark_mode: bool,
/// }
///
/// let state = create_rw_signal(
///     cx,
///     State {
///         token: "".into(),
///         // this would cause flickering on reload,
///         // use a cookie for the initial value in real projects
///         dark_mode: false,
///     },
/// );
/// let (token, set_token) = create_slice(
///     cx,
///     state,
///     |state| state.token.clone(),
///     |state, value| state.token = value,
/// );
/// let (dark_mode, set_dark_mode) = create_slice(
///     cx,
///     state,
///     |state| state.dark_mode,
///     |state, value| state.dark_mode = value,
/// );
/// let count_token_updates = create_rw_signal(cx, 0);
/// count_token_updates.with(|counter| assert_eq!(counter, &0));
/// create_effect(cx, move |_| {
///     token.with(|_| {});
///     count_token_updates.update(|counter| *counter += 1)
/// });
/// count_token_updates.with(|counter| assert_eq!(counter, &1));
/// set_token.set("this is not a token!".into());
/// // token was updated with the new token
/// count_token_updates.with(|counter| assert_eq!(counter, &2));
/// set_dark_mode.set(true);
/// // since token didn't change, there was also no update emitted
/// count_token_updates.with(|counter| assert_eq!(counter, &2));
/// ```
pub fn create_slice<T, O>(
    cx: Scope,
    signal: RwSignal<T>,
    getter: impl Fn(&T) -> O + Clone + Copy + 'static,
    setter: impl Fn(&mut T, O) + Clone + Copy + 'static,
) -> (Signal<O>, SignalSetter<O>)
where
    O: Eq,
{
    let getter = create_memo(cx, move |_| signal.with(getter));
    let setter = move |value| signal.update(|x| setter(x, value));
    (getter.into(), setter.mapped_signal_setter(cx))
}
