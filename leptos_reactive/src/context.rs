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
/// the reactive graph can be used lower down, but a context that is provided lower
/// down cannot be used higher up.
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
///
/// ## Warning: Shadowing Context Correctly
///
/// The reactive graph exists alongside the component tree. Generally
/// speaking, context provided by a parent component can be accessed by its children
/// and other descendants, and not vice versa. But components do not exist at
/// runtime: a parent and children that are all rendered unconditionally exist in the same
/// reactive scope.
///
/// This can have unexpected effects on context: namely, children can sometimes override
/// contexts provided by their parents, including for their siblings, if they “shadow” context
/// by providing another context of the same kind.
/// ```rust
/// use leptos::*;
///
/// #[component]
/// fn Parent() -> impl IntoView {
///     provide_context("parent_context");
///     view! {
///         <Child /> // this is receiving "parent_context" as expected
///         <Child /> // but this is receiving "child_context" instead of "parent_context"!
///     }
/// }
///
/// #[component]
/// fn Child() -> impl IntoView {
///     // first, we receive context from parent (just before the override)
///     let context = expect_context::<&'static str>();
///     // then we provide context under the same type
///     provide_context("child_context");
///     view! {
///         <div>{format!("child (context: {context})")}</div>
///     }
/// }
/// ```
/// In this case, neither of the children is rendered dynamically, so there is no wrapping
/// effect created around either. All three components here have the same reactive owner, so
/// providing a new context of the same type in the first `<Child/>` overrides the context
/// that was provided in `<Parent/>`, meaning that the second `<Child/>` receives the context
/// from its sibling instead.
///
/// ### Solution
///
/// If you are using the full Leptos framework, you can use the [`Provider`](../leptos/fn.Provider.html)
/// component to solve this issue.
///
/// ```rust
/// # use leptos::*;
/// #[component]
/// fn Child() -> impl IntoView {
///     let context = expect_context::<&'static str>();
///     // creates a new reactive node, which means the context will
///     // only be provided to its children, not modified in the parent
///     view! {
///         <Provider value="child_context">
///             <div>{format!("child (context: {context})")}</div>
///         </Provider>
///     }
/// }
/// ```
///
/// ### Alternate Solution
///
/// This can also be solved by introducing some additional reactivity. In this case, it’s simplest
/// to simply make the body of `<Child/>` a function, which means it will be wrapped in a
/// new reactive node when rendered:
/// ```rust
/// # use leptos::*;
/// #[component]
/// fn Child() -> impl IntoView {
///     let context = expect_context::<&'static str>();
///     // creates a new reactive node, which means the context will
///     // only be provided to its children, not modified in the parent
///     move || {
///         provide_context("child_context");
///         view! {
///             <div>{format!("child (context: {context})")}</div>
///         }
///     }
/// }
/// ```
///
/// This is equivalent to the difference between two different forms of variable shadowing
/// in ordinary Rust:
/// ```rust
/// // shadowing in a flat hierarchy overrides value for siblings
/// // <Parent/>: declares variable
/// let context = "parent_context";
/// // First <Child/>: consumes variable, then shadows
/// println!("{context:?}");
/// let context = "child_context";
/// // Second <Child/>: consumes variable, then shadows
/// println!("{context:?}");
/// let context = "child_context";
///
/// // but shadowing in nested scopes works as expected
/// // <Parent/>
/// let context = "parent_context";
///
/// // First <Child/>
/// {
///     println!("{context:?}");
///     let context = "child_context";
/// }
///
/// // Second <Child/>
/// {
///     println!("{context:?}");
///     let context = "child_context";
/// }
/// ```
#[cfg_attr(
    any(debug_assertions, feature = "ssr"),
    instrument(level = "trace", skip_all,)
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
/// the reactive graph can be used lower down, but a context that is provided lower
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
    instrument(level = "trace", skip_all,)
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
/// the reactive graph can be used lower down, but a context that is provided lower
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
