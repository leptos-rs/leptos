use crate::owner::Owner;
use or_poisoned::OrPoisoned;
use std::any::{Any, TypeId};

impl Owner {
    fn provide_context<T: Send + Sync + 'static>(&self, value: T) {
        self.inner
            .write()
            .or_poisoned()
            .contexts
            .insert(value.type_id(), Box::new(value));
    }

    fn use_context<T: Clone + 'static>(&self) -> Option<T> {
        let ty = TypeId::of::<T>();
        let inner = self.inner.read().or_poisoned();
        let mut parent = inner.parent.as_ref().and_then(|p| p.upgrade());
        let contexts = &self.inner.read().or_poisoned().contexts;
        if let Some(context) = contexts.get(&ty) {
            context.downcast_ref::<T>().cloned()
        } else {
            while let Some(ref this_parent) = parent.clone() {
                let this_parent = this_parent.read().or_poisoned();
                let contexts = &this_parent.contexts;
                let value = contexts.get(&ty);
                let downcast = value
                    .and_then(|context| context.downcast_ref::<T>().cloned());
                if let Some(value) = downcast {
                    return Some(value);
                } else {
                    parent =
                        this_parent.parent.as_ref().and_then(|p| p.upgrade());
                }
            }
            None
        }
    }

    fn take_context<T: 'static>(&self) -> Option<T> {
        let ty = TypeId::of::<T>();
        let inner = self.inner.read().or_poisoned();
        let mut parent = inner.parent.as_ref().and_then(|p| p.upgrade());
        let contexts = &mut self.inner.write().or_poisoned().contexts;
        if let Some(context) = contexts.remove(&ty) {
            context.downcast::<T>().ok().map(|n| *n)
        } else {
            while let Some(ref this_parent) = parent.clone() {
                let mut this_parent = this_parent.write().or_poisoned();
                let contexts = &mut this_parent.contexts;
                let value = contexts.remove(&ty);
                let downcast =
                    value.and_then(|context| context.downcast::<T>().ok());
                if let Some(value) = downcast {
                    return Some(*value);
                } else {
                    parent =
                        this_parent.parent.as_ref().and_then(|p| p.upgrade());
                }
            }
            None
        }
    }
}

/// Provides a context value of type `T` to the current reactive [`Owner`]
/// and all of its descendants. This can be accessed using [`use_context`].
///
/// This is useful for passing values down to components or functions lower in a
/// hierarchy without needs to “prop drill” by passing them through each layer as
/// arguments to a function or properties of a component.
///
/// Context works similarly to variable scope: a context that is provided higher in
/// the reactive graph can be used lower down, but a context that is provided lower
/// down cannot be used higher up.
///
/// ```rust
/// # use reactive_graph::prelude::*;
/// # use reactive_graph::owner::*;
/// # use reactive_graph::effect::Effect;
/// # futures::executor::block_on(async move {
/// # any_spawner::Executor::init_futures_executor();
/// Effect::new(move |_| {
///     println!("Provider");
///     provide_context(42i32); // provide an i32
///
///     Effect::new(move |_| {
///         println!("intermediate node");
///
///         Effect::new(move |_| {
///             let value = use_context::<i32>()
///                 .expect("could not find i32 in context");
///             assert_eq!(value, 42);
///         });
///     });
/// });
/// # });
/// ```
///
/// ## Context Shadowing
///
/// Only a single value of any type can be provided via context. If you need to provide multiple
/// values of the same type, wrap each one in a "newtype" struct wrapper so that each one is a
/// distinct type.
///
/// Providing a second value of the same type "lower" in the ownership tree will shadow the value,
/// just as a second `let` declaration with the same variable name will shadow that variable.
///
/// ```rust
/// # use reactive_graph::prelude::*;
/// # use reactive_graph::owner::*;
/// # use reactive_graph::effect::Effect;
/// # futures::executor::block_on(async move {
/// # any_spawner::Executor::init_futures_executor();
/// Effect::new(move |_| {
///     println!("Provider");
///     provide_context("foo"); // provide a &'static str
///
///     Effect::new(move |_| {
///         // before we provide another value of the same type, we can access the old one
///         assert_eq!(use_context::<&'static str>(), Some("foo"));
///         // but providing another value of the same type shadows it
///         provide_context("bar");
///
///         Effect::new(move |_| {
///             assert_eq!(use_context::<&'static str>(), Some("bar"));
///         });
///     });
/// });
/// # });
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
/// use leptos::prelude::*;
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
/// If you are using the full Leptos framework, you can use the [`Provider`](leptos::context::Provider)
/// component to solve this issue.
///
/// ```rust
/// # use leptos::prelude::*;
/// # use leptos::context::Provider;
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
/// # use leptos::prelude::*;
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
pub fn provide_context<T: Send + Sync + 'static>(value: T) {
    if let Some(owner) = Owner::current() {
        owner.provide_context(value);
    }
}

/// Extracts a context value of type `T` from the reactive system by traversing
/// it upwards, beginning from the current reactive [`Owner`] and iterating
/// through its parents, if any. When the value is found, it is cloned.
///
/// The context value should have been provided elsewhere using
/// [`provide_context`](provide_context).
///
/// This is useful for passing values down to components or functions lower in a
/// hierarchy without needs to “prop drill” by passing them through each layer as
/// arguments to a function or properties of a component.
///
/// Context works similarly to variable scope: a context that is provided higher in
/// the reactive graph can be used lower down, but a context that is provided lower
/// in the tree cannot be used higher up.
///
/// While the term “consume” is sometimes used, note that [`use_context`] clones the value, rather
/// than removing it; it is still accessible to other users.
///
/// ```rust
/// # use reactive_graph::prelude::*;
/// # use reactive_graph::owner::*;
/// # use reactive_graph::effect::Effect;
/// # futures::executor::block_on(async move {
/// # any_spawner::Executor::init_futures_executor();
/// Effect::new(move |_| {
///     provide_context(String::from("foo"));
///
///     Effect::new(move |_| {
///         // each use_context clones the value
///         let value =
///             use_context::<String>().expect("could not find i32 in context");
///         assert_eq!(value, "foo");
///         let value2 =
///             use_context::<String>().expect("could not find i32 in context");
///         assert_eq!(value2, "foo");
///     });
/// });
/// # });
/// ```
pub fn use_context<T: Clone + 'static>() -> Option<T> {
    Owner::current().and_then(|owner| owner.use_context())
}

/// Extracts a context value of type `T` from the reactive system by traversing
/// it upwards, beginning from the current reactive [`Owner`] and iterating
/// through its parents, if any. When the value is found, it is cloned.
///
/// Panics if no value is found.
///
/// The context value should have been provided elsewhere using
/// [`provide_context`](provide_context).
///
/// This is useful for passing values down to components or functions lower in a
/// hierarchy without needs to “prop drill” by passing them through each layer as
/// arguments to a function or properties of a component.
///
/// Context works similarly to variable scope: a context that is provided higher in
/// the reactive graph can be used lower down, but a context that is provided lower
/// in the tree cannot be used higher up.
///
/// While the term “consume” is sometimes used, note that [`use_context`] clones the value, rather
/// than removing it; it is still accessible to other users.
///
/// ```rust
/// # use reactive_graph::prelude::*;
/// # use reactive_graph::owner::*;
/// # use reactive_graph::effect::Effect;
/// # futures::executor::block_on(async move {
/// # any_spawner::Executor::init_futures_executor();
/// Effect::new(move |_| {
///     provide_context(String::from("foo"));
///
///     Effect::new(move |_| {
///         // each use_context clones the value
///         let value =
///             use_context::<String>().expect("could not find i32 in context");
///         assert_eq!(value, "foo");
///         let value2 =
///             use_context::<String>().expect("could not find i32 in context");
///         assert_eq!(value2, "foo");
///     });
/// });
/// # });
/// ```
/// ## Panics
/// Panics if a context of this type is not found in the current reactive
/// owner or its ancestors.
#[track_caller]
pub fn expect_context<T: Clone + 'static>() -> T {
    let location = std::panic::Location::caller();

    use_context().unwrap_or_else(|| {
        panic!(
            "{:?} expected context of type {:?} to be present",
            location,
            std::any::type_name::<T>()
        )
    })
}

/// Extracts a context value of type `T` from the reactive system by traversing
/// it upwards, beginning from the current reactive [`Owner`] and iterating
/// through its parents, if any. When the value is found, it is removed from the context,
/// and is not available to any other [`use_context`] or [`take_context`] calls.
///
/// If the value is `Clone`, use [`use_context`] instead.
///
/// The context value should have been provided elsewhere using
/// [`provide_context`](provide_context).
///
/// This is useful for passing values down to components or functions lower in a
/// hierarchy without needs to “prop drill” by passing them through each layer as
/// arguments to a function or properties of a component.
///
/// Context works similarly to variable scope: a context that is provided higher in
/// the reactive graph can be used lower down, but a context that is provided lower
/// in the tree cannot be used higher up.
/// ```rust
/// # use reactive_graph::prelude::*;
/// # use reactive_graph::owner::*;
/// # use reactive_graph::effect::Effect;
/// # futures::executor::block_on(async move {
/// # any_spawner::Executor::init_futures_executor();
///
/// #[derive(Debug, PartialEq)]
/// struct NotClone(String);
///
/// Effect::new(move |_| {
///     provide_context(NotClone(String::from("foo")));
///
///     Effect::new(move |_| {
///         // take_context removes the value from context without needing to clone
///         let value = take_context::<NotClone>();
///         assert_eq!(value, Some(NotClone(String::from("foo"))));
///         let value2 = take_context::<NotClone>();
///         assert_eq!(value2, None);
///     });
/// });
/// # });
/// ```
pub fn take_context<T: 'static>() -> Option<T> {
    Owner::current().and_then(|owner| owner.take_context())
}
