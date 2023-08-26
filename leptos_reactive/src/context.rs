use crate::runtime::with_runtime;
use std::any::{Any, TypeId};

/// Provides a context value of type `T` to the current reactive node
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
/// pub fn Provider() -> impl IntoView {
///     let (value, set_value) = create_signal(0);
///
///     // the newtype pattern isn't *necessary* here but is a good practice
///     // it avoids confusion with other possible future `WriteSignal<bool>` contexts
///     // and makes it easier to refer to it in ButtonD
///     provide_context(ValueSetter(set_value));
///
///     // because <Consumer/> is nested inside <Provider/>,
///     // it has access to the provided context
///     view! { <div><Consumer/></div> }
/// }
///
/// #[component]
/// pub fn Consumer() -> impl IntoView {
///     // consume the provided context of type `ValueSetter` using `use_context`
///     // this traverses up the reactive graph and gets the nearest provided `ValueSetter`
///     let set_value = use_context::<ValueSetter>().unwrap().0;
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

    with_runtime(|runtime| {
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
/// it upwards, beginning from the current reactive owner and iterating
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
/// pub fn Provider() -> impl IntoView {
///     let (value, set_value) = create_signal(0);
///
///     // the newtype pattern isn't *necessary* here but is a good practice
///     // it avoids confusion with other possible future `WriteSignal<bool>` contexts
///     // and makes it easier to refer to it in ButtonD
///     provide_context(ValueSetter(set_value));
///
///     // because <Consumer/> is nested inside <Provider/>,
///     // it has access to the provided context
///     view! { <div><Consumer/></div> }
/// }
///
/// #[component]
/// pub fn Consumer() -> impl IntoView {
///     // consume the provided context of type `ValueSetter` using `use_context`
///     // this traverses up the reactive graph and gets the nearest provided `ValueSetter`
///     let set_value = use_context::<ValueSetter>().unwrap().0;
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

    with_runtime(|runtime| {
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
/// it upwards, beginning from the current reactive owner and iterating
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
/// pub fn Provider() -> impl IntoView {
///     let (value, set_value) = create_signal(0);
///
///     // the newtype pattern isn't *necessary* here but is a good practice
///     // it avoids confusion with other possible future `WriteSignal<bool>` contexts
///     // and makes it easier to refer to it in ButtonD
///     provide_context(ValueSetter(set_value));
///
///     // because <Consumer/> is nested inside <Provider/>,
///     // it has access to the provided context
///     view! { <div><Consumer/></div> }
/// }
///
/// #[component]
/// pub fn Consumer() -> impl IntoView {
///     // consume the provided context of type `ValueSetter` using `use_context`
///     // this traverses up the reactive graph and gets the nearest provided `ValueSetter`
///     let set_value = expect_context::<ValueSetter>().0;
///
///     todo!()
/// }
/// ```
///
/// ## Panics
/// Panics if a context of this type is not found in the current reactive
/// owner or its ancestors.
#[track_caller]
pub fn expect_context<T>() -> T
where
    T: Clone + 'static,
{
    let location = std::panic::Location::caller();

    use_context().unwrap_or_else(|| {
        panic!(
            "{:?} expected context of type {:?} to be present",
            location,
            std::any::type_name::<T>()
        )
    })
}
