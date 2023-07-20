#![forbid(unsafe_code)]

use crate::{runtime::with_runtime, Runtime};
use std::any::{Any, TypeId};

/// Provides a context value of type `T` to the current reactive [`Scope`](crate::Scope)
/// and all of its descendants. This can be consumed using [`use_context`](crate::use_context).
///
/// This is useful for passing values down to components or functions lower in a
/// hierarchy without needs to “prop drill” by passing them through each layer as
/// arguments to a function or properties of a component.
///
/// Context works similarly to variable scope: a context that is provided higher in
/// the component tree can be used lower down, but a context that is provided lower
/// in the tree cannot be used higher up.
///
/// ```
/// use leptos::*;
///
/// // define a newtype we'll provide as context
/// // contexts are stored by their types, so it can be useful to create
/// // a new type to avoid confusion with other `WriteSignal<i32>`s we may have
/// // all types to be shared via context should implement `Clone`
/// #[derive(Copy, Clone)]
/// struct ValueSetter(WriteSignal<i32>);
///
/// #[component]
/// pub fn Provider(cx: Scope) -> impl IntoView {
///     let (value, set_value) = create_signal(cx, 0);
///
///     // the newtype pattern isn't *necessary* here but is a good practice
///     // it avoids confusion with other possible future `WriteSignal<bool>` contexts
///     // and makes it easier to refer to it in ButtonD
///     provide_context(cx, ValueSetter(set_value));
///
///     // because <Consumer/> is nested inside <Provider/>,
///     // it has access to the provided context
///     view! { cx, <div><Consumer/></div> }
/// }
///
/// #[component]
/// pub fn Consumer(cx: Scope) -> impl IntoView {
///     // consume the provided context of type `ValueSetter` using `use_context`
///     // this traverses up the tree of `Scope`s and gets the nearest provided `ValueSetter`
///     let set_value = use_context::<ValueSetter>(cx).unwrap().0;
/// }
/// ```
#[cfg_attr(
    any(debug_assertions, feature = "ssr"),
    instrument(level = "info", skip_all,)
)]
#[track_caller]
pub fn provide_context<T>(value: T)
where
    T: Clone + 'static,
{
    let id = value.type_id();
    #[cfg(debug_assertions)]
    let defined_at = std::panic::Location::caller();

    with_runtime(Runtime::current(), |runtime| {
        let mut contexts = runtime.contexts.borrow_mut();
        let owner = runtime.owner.get();
        if let Some(owner) = owner {
            let context = contexts.entry(owner).unwrap().or_default();
            context.insert(id, Box::new(value) as Box<dyn Any>);
        } else {
            crate::macros::debug_warn!(
                "At {defined_at}, you are calling provide_context() outside \
                 the reactive system.",
            );
        }
    })
    .expect("provide_context failed");
}

/// Extracts a context value of type `T` from the reactive system by traversing
/// it upwards, beginning from the current [`Scope`](crate::Scope) and iterating
/// through its parents, if any. The context value should have been provided elsewhere
/// using [`provide_context`](crate::provide_context).
///
/// This is useful for passing values down to components or functions lower in a
/// hierarchy without needs to “prop drill” by passing them through each layer as
/// arguments to a function or properties of a component.
///
/// Context works similarly to variable scope: a context that is provided higher in
/// the component tree can be used lower down, but a context that is provided lower
/// in the tree cannot be used higher up.
///
/// ```
/// use leptos::*;
///
/// // define a newtype we'll provide as context
/// // contexts are stored by their types, so it can be useful to create
/// // a new type to avoid confusion with other `WriteSignal<i32>`s we may have
/// // all types to be shared via context should implement `Clone`
/// #[derive(Copy, Clone)]
/// struct ValueSetter(WriteSignal<i32>);
///
/// #[component]
/// pub fn Provider(cx: Scope) -> impl IntoView {
///     let (value, set_value) = create_signal(cx, 0);
///
///     // the newtype pattern isn't *necessary* here but is a good practice
///     // it avoids confusion with other possible future `WriteSignal<bool>` contexts
///     // and makes it easier to refer to it in ButtonD
///     provide_context(cx, ValueSetter(set_value));
///
///     // because <Consumer/> is nested inside <Provider/>,
///     // it has access to the provided context
///     view! { cx, <div><Consumer/></div> }
/// }
///
/// #[component]
/// pub fn Consumer(cx: Scope) -> impl IntoView {
///     // consume the provided context of type `ValueSetter` using `use_context`
///     // this traverses up the tree of `Scope`s and gets the nearest provided `ValueSetter`
///     let set_value = use_context::<ValueSetter>(cx).unwrap().0;
///
/// }
/// ```
#[cfg_attr(
    any(debug_assertions, feature = "ssr"),
    instrument(level = "info", skip_all,)
)]
pub fn use_context<T>() -> Option<T>
where
    T: Clone + 'static,
{
    let ty = TypeId::of::<T>();

    with_runtime(Runtime::current(), |runtime| {
        let owner = runtime.owner.get();
        if let Some(owner) = owner {
            runtime.get_context(owner, ty)
        } else {
            crate::macros::debug_warn!(
                "At {}, you are calling use_context() outside the reactive \
                 system.",
                std::panic::Location::caller()
            );
            None
        }
    })
    .ok()
    .flatten()
}

/// Extracts a context value of type `T` from the reactive system by traversing
/// it upwards, beginning from the current [Scope](crate::Scope) and iterating
/// through its parents, if any. The context value should have been provided elsewhere
/// using [provide_context](crate::provide_context).
///
/// This is useful for passing values down to components or functions lower in a
/// hierarchy without needs to “prop drill” by passing them through each layer as
/// arguments to a function or properties of a component.
///
/// Context works similarly to variable scope: a context that is provided higher in
/// the component tree can be used lower down, but a context that is provided lower
/// in the tree cannot be used higher up.
///
/// ```
/// use leptos::*;
///
/// // define a newtype we'll provide as context
/// // contexts are stored by their types, so it can be useful to create
/// // a new type to avoid confusion with other `WriteSignal<i32>`s we may have
/// // all types to be shared via context should implement `Clone`
/// #[derive(Copy, Clone)]
/// struct ValueSetter(WriteSignal<i32>);
///
/// #[component]
/// pub fn Provider(cx: Scope) -> impl IntoView {
///     let (value, set_value) = create_signal(cx, 0);
///
///     // the newtype pattern isn't *necessary* here but is a good practice
///     // it avoids confusion with other possible future `WriteSignal<bool>` contexts
///     // and makes it easier to refer to it in ButtonD
///     provide_context(cx, ValueSetter(set_value));
///
///     // because <Consumer/> is nested inside <Provider/>,
///     // it has access to the provided context
///     view! { cx, <div><Consumer/></div> }
/// }
///
/// #[component]
/// pub fn Consumer(cx: Scope) -> impl IntoView {
///     // consume the provided context of type `ValueSetter` using `use_context`
///     // this traverses up the tree of `Scope`s and gets the nearest provided `ValueSetter`
///     let set_value = expect_context::<ValueSetter>(cx).0;
///
///     todo!()
/// }
/// ```
pub fn expect_context<T>() -> T
where
    T: Clone + 'static,
{
    use_context().unwrap_or_else(|| {
        panic!(
            "context of type {:?} to be present",
            std::any::type_name::<T>()
        )
    })
}
