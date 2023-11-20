use crate::{with_runtime, Runtime, ScopeProperty};

/// A version of [`create_effect`](crate::create_effect) that listens to any dependency
/// that is accessed inside `deps` and returns a stop handler.
///
/// The return value of `deps` is passed into `callback` as an argument together with the previous value.
/// Additionally the last return value of `callback` is provided as a third argument as is done in [`create_effect`](crate::create_effect).
///
/// ## Usage
///
/// ```
/// # use leptos_reactive::*;
/// # use log;
/// # let runtime = create_runtime();
/// let (num, set_num) = create_signal(0);
///
/// let stop = watch(
///     move || num.get(),
///     move |num, prev_num, _| {
///         log::debug!("Number: {}; Prev: {:?}", num, prev_num);
///     },
///     false,
/// );
///
/// set_num.set(1); // > "Number: 1; Prev: Some(0)"
///
/// stop(); // stop watching
///
/// set_num.set(2); // (nothing happens)
/// # runtime.dispose();
/// ```
///
/// The callback itself doesn't track any signal that is accessed within it.
///
/// ```
/// # use leptos_reactive::*;
/// # use log;
/// # let runtime = create_runtime();
/// let (num, set_num) = create_signal(0);
/// let (cb_num, set_cb_num) = create_signal(0);
///
/// watch(
///     move || num.get(),
///     move |num, _, _| {
///         log::debug!("Number: {}; Cb: {}", num, cb_num.get());
///     },
///     false,
/// );
///
/// set_num.set(1); // > "Number: 1; Cb: 0"
///
/// set_cb_num.set(1); // (nothing happens)
///
/// set_num.set(2); // > "Number: 2; Cb: 1"
/// # runtime.dispose();
/// ```
///
/// ## Immediate
///
/// If the final parameter `immediate` is true, the `callback` will run immediately.
/// If it's `false`, the `callback` will run only after
/// the first change is detected of any signal that is accessed in `deps`.
///
/// ```
/// # use leptos_reactive::*;
/// # use log;
/// # let runtime = create_runtime();
/// let (num, set_num) = create_signal(0);
///
/// watch(
///     move || num.get(),
///     move |num, prev_num, _| {
///         log::debug!("Number: {}; Prev: {:?}", num, prev_num);
///     },
///     true,
/// ); // > "Number: 0; Prev: None"
///
/// set_num.set(1); // > "Number: 1; Prev: Some(0)"
/// # runtime.dispose();
/// ```
#[cfg_attr(
    any(debug_assertions, feature="ssr"),
    instrument(
        level = "trace",
        skip_all,
        fields(
            ty = %std::any::type_name::<T>()
        )
    )
)]
#[track_caller]
#[inline(always)]
pub fn watch<W, T>(
    deps: impl Fn() -> W + 'static,
    callback: impl Fn(&W, Option<&W>, Option<T>) -> T + Clone + 'static,
    immediate: bool,
) -> impl Fn() + Clone
where
    W: Clone + 'static,
    T: 'static,
{
    let runtime = Runtime::current();
    let (e, stop) = runtime.watch(deps, callback, immediate);
    let prop = ScopeProperty::Effect(e);
    let owner = crate::Owner::current();
    _ = with_runtime(|runtime| {
        runtime.update_if_necessary(e);
    });

    move || {
        stop();
        if let Some(owner) = owner {
            _ = with_runtime(|runtime| {
                runtime.remove_scope_property(owner.0, prop)
            });
        }
    }
}
