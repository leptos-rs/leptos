use crate::{
    create_memo, IntoSignalSetter, RwSignal, Scope, Signal, SignalSetter,
    SignalUpdate, SignalWith,
};

/// Derives a reactive slice of an [`RwSignal`](crate::RwSignal).
///
/// Slices have the same guarantees as [`Memo`s](crate::Memo):
/// they only emit their value when it has actually been changed.
///
/// Slices need a getter and a setter, and you must make sure that
/// the setter and getter only touch their respective field and nothing else.
/// They optimally should not have any side effects.
///
/// You can use slices whenever you want to react to only parts
/// of a bigger signal. The prime example would be state management,
/// where you want all state variables grouped together, but also need
/// fine-grained signals for each or some of these variables.
/// In the example below, setting an auth token will only trigger
/// the token signal, but none of the other derived signals.
/// ```
/// # use leptos_reactive::*;
/// # let (cx, disposer) = raw_scope_and_disposer(create_runtime());
///
/// // some global state with independent fields
/// #[derive(Default, Clone, Debug)]
/// struct GlobalState {
///     count: u32,
///     name: String,
/// }
///
/// let state = create_rw_signal(cx, GlobalState::default());
///
/// // `create_slice` lets us create a "lens" into the data
/// let (count, set_count) = create_slice(
///     cx,
///     // we take a slice *from* `state`
///     state,
///     // our getter returns a "slice" of the data
///     |state| state.count,
///     // our setter describes how to mutate that slice, given a new value
///     |state, n| state.count = n,
/// );
///
/// // this slice is completely independent of the `count` slice
/// // neither of them will cause the other to rerun
/// let (name, set_name) = create_slice(
///     cx,
///     // we take a slice *from* `state`
///     state,
///     // our getter returns a "slice" of the data
///     |state| state.name.clone(),
///     // our setter describes how to mutate that slice, given a new value
///     |state, n| state.name = n,
/// );
///
/// create_effect(cx, move |_| {
///     // note: in the browser, use leptos::log! instead
///     println!("name is {}", name.get());
/// });
/// create_effect(cx, move |_| {
///     println!("count is {}", count.get());
/// });
///
/// // setting count only causes count to log, not name
/// set_count.set(42);
///
/// // setting name only causes name to log, not count
/// set_name.set("Bob".into());
/// ```
#[track_caller]
pub fn create_slice<T, O, S>(
    cx: Scope,
    signal: RwSignal<T>,
    getter: impl Fn(&T) -> O + Clone + Copy + 'static,
    setter: impl Fn(&mut T, S) + Clone + Copy + 'static,
) -> (Signal<O>, SignalSetter<S>)
where
    O: PartialEq,
{
    (
        create_read_slice(cx, signal, getter),
        create_write_slice(cx, signal, setter),
    )
}

/// Takes a memoized, read-only slice of a signal. This is equivalent to the
/// read-only half of [`create_slice`].
#[track_caller]
pub fn create_read_slice<T, O>(
    cx: Scope,
    signal: RwSignal<T>,
    getter: impl Fn(&T) -> O + Clone + Copy + 'static,
) -> Signal<O>
where
    O: PartialEq,
{
    create_memo(cx, move |_| signal.with(getter)).into()
}

/// Creates a setter to access one slice of a signal. This is equivalent to the
/// write-only half of [`create_slice`].
#[track_caller]
pub fn create_write_slice<T, O>(
    cx: Scope,
    signal: RwSignal<T>,
    setter: impl Fn(&mut T, O) + Clone + Copy + 'static,
) -> SignalSetter<O> {
    let setter = move |value| signal.update(|x| setter(x, value));
    setter.mapped_signal_setter(cx)
}
