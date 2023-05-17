#![cfg_attr(not(feature = "stable"), feature(proc_macro_span))]
#![forbid(unsafe_code)]

#[macro_use]
extern crate proc_macro_error;

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenTree};
use quote::ToTokens;
use server_fn_macro::{server_macro_impl, ServerContext};
use syn::parse_macro_input;
use rstml::{parse, node::KeyedAttribute};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum Mode {
    Client,
    Ssr,
}

impl Default for Mode {
    fn default() -> Self {
        if cfg!(feature = "hydrate")
            || cfg!(feature = "csr")
            || cfg!(feature = "web")
        {
            Mode::Client
        } else {
            Mode::Ssr
        }
    }
}

mod params;
mod view;
use template::render_template;
use view::render_view;
mod component;
mod slot;
mod template;

/// The `view` macro uses RSX (like JSX, but Rust!) It follows most of the
/// same rules as HTML, with the following differences:
///
/// 1. Text content should be provided as a Rust string, i.e., double-quoted:
/// ```rust
/// # use leptos::*;
/// # run_scope(create_runtime(), |cx| {
/// # if !cfg!(any(feature = "csr", feature = "hydrate")) {
/// view! { cx, <p>"Here’s some text"</p> };
/// # }
/// # });
/// ```
///
/// 2. Self-closing tags need an explicit `/` as in XML/XHTML
/// ```rust,compile_fail
/// # use leptos::*;
/// # run_scope(create_runtime(), |cx| {
/// # if !cfg!(any(feature = "csr", feature = "hydrate")) {
/// // ❌ not like this
/// view! { cx, <input type="text" name="name"> }
/// # ;
/// # }
/// # });
/// ```
/// ```rust
/// # use leptos::*;
/// # run_scope(create_runtime(), |cx| {
/// # if !cfg!(any(feature = "csr", feature = "hydrate")) {
/// // ✅ add that slash
/// view! { cx, <input type="text" name="name" /> }
/// # ;
/// # }
/// # });
/// ```
///
/// 3. Components (functions annotated with `#[component]`) can be inserted as camel-cased tags
/// ```rust
/// # use leptos::*;
/// # #[component]
/// # fn Counter(cx: Scope, initial_value: i32) -> impl IntoView { view! { cx, <p></p>} }
/// # run_scope(create_runtime(), |cx| {
/// # if !cfg!(any(feature = "csr", feature = "hydrate")) {
/// view! { cx, <div><Counter initial_value=3 /></div> }
/// # ;
/// # }
/// # });
/// ```
///
/// 4. Dynamic content can be wrapped in curly braces (`{ }`) to insert text nodes, elements, or set attributes.
///    If you insert a signal here, Leptos will create an effect to update the DOM whenever the value changes.
///    *(“Signal” here means `Fn() -> T` where `T` is the appropriate type for that node: a `String` in case
///    of text nodes, a `bool` for `class:` attributes, etc.)*
///
///    Attributes can take a wide variety of primitive types that can be converted to strings. They can also
///    take an `Option`, in which case `Some` sets the attribute and `None` removes the attribute.
///
/// ```rust
/// # use leptos::*;
/// # run_scope(create_runtime(), |cx| {
/// # if !cfg!(any(feature = "csr", feature = "hydrate")) {
/// let (count, set_count) = create_signal(cx, 0);
///
/// view! {
///   cx,
///   // ❌ not like this: `count()` returns an `i32`, not a function
///   <p>{count()}</p>
///   // ✅ this is good: Leptos sees the function and knows it's a dynamic value
///   <p>{move || count.get()}</p>
///   // 🔥 `count` is itself a function, so you can pass it directly (unless you're on `stable`)
///   <p>{count}</p>
/// }
/// # ;
/// # }
/// # });
/// ```
///
/// 5. Event handlers can be added with `on:` attributes. In most cases, the events are given the correct type
///    based on the event name.
/// ```rust
/// # use leptos::*;
/// # run_scope(create_runtime(), |cx| {
/// # if !cfg!(any(feature = "csr", feature = "hydrate")) {
/// view! {
///   cx,
///   <button on:click=|ev| {
///     log::debug!("click event: {ev:#?}");
///   }>
///     "Click me"
///   </button>
/// }
/// # ;
/// # }
/// # });
/// ```
///
/// 6. DOM properties can be set with `prop:` attributes, which take any primitive type or `JsValue` (or a signal
///    that returns a primitive or JsValue). They can also take an `Option`, in which case `Some` sets the property
///    and `None` deletes the property.
/// ```rust
/// # use leptos::*;
/// # run_scope(create_runtime(), |cx| {
/// # if !cfg!(any(feature = "csr", feature = "hydrate")) {
/// let (name, set_name) = create_signal(cx, "Alice".to_string());
///
/// view! {
///   cx,
///   <input
///     type="text"
///     name="user_name"
///     value={name} // this only sets the default value!
///     prop:value={name} // here's how you update values. Sorry, I didn’t invent the DOM.
///     on:click=move |ev| set_name(event_target_value(&ev)) // `event_target_value` is a useful little Leptos helper
///   />
/// }
/// # ;
/// # }
/// # });
/// ```
///
/// 7. Classes can be toggled with `class:` attributes, which take a `bool` (or a signal that returns a `bool`).
/// ```rust
/// # use leptos::*;
/// # run_scope(create_runtime(), |cx| {
/// # if !cfg!(any(feature = "csr", feature = "hydrate")) {
/// let (count, set_count) = create_signal(cx, 2);
/// view! { cx, <div class:hidden-div={move || count() < 3}>"Now you see me, now you don’t."</div> }
/// # ;
/// # }
/// # });
/// ```
///
/// Class names can include dashes, but cannot (at the moment) include a dash-separated segment of only numbers.
/// ```rust,compile_fail
/// # use leptos::*;
/// # run_scope(create_runtime(), |cx| {
/// # if !cfg!(any(feature = "csr", feature = "hydrate")) {
/// let (count, set_count) = create_signal(cx, 2);
/// // `hidden-div-25` is invalid at the moment
/// view! { cx, <div class:hidden-div-25={move || count() < 3}>"Now you see me, now you don’t."</div> }
/// # ;
/// # }
/// # });
/// ```
///
/// However, you can pass arbitrary class names using the syntax `class=("name", value)`.
/// ```rust
/// # use leptos::*;
/// # run_scope(create_runtime(), |cx| {
/// # if !cfg!(any(feature = "csr", feature = "hydrate")) {
/// let (count, set_count) = create_signal(cx, 2);
/// // this allows you to use CSS frameworks that include complex class names
/// view! { cx,
///   <div
///     class=("is-[this_-_really]-necessary-42", move || count() < 3)
///   >
///     "Now you see me, now you don’t."
///   </div>
/// }
/// # ;
/// # }
/// # });
/// ```
///
/// 8. Individual styles can also be set with `style:` or `style=("property-name", value)` syntax.
/// ```rust
/// # use leptos::*;
/// # run_scope(create_runtime(), |cx| {
/// # if !cfg!(any(feature = "csr", feature = "hydrate")) {
/// let (x, set_x) = create_signal(cx, 0);
/// let (y, set_y) = create_signal(cx, 0);
/// view! { cx,
///   <div
///     style="position: absolute"
///     style:left=move || format!("{}px", x())
///     style:top=move || format!("{}px", y())
///     style=("background-color", move || format!("rgb({}, {}, 100)", x(), y()))
///   >
///     "Moves when coordinates change"
///   </div>
/// }
/// # ;
/// # }
/// # });
/// ```
///
/// 9. You can use the `node_ref` or `_ref` attribute to store a reference to its DOM element in a
///    [NodeRef](https://docs.rs/leptos/latest/leptos/struct.NodeRef.html) to use later.
/// ```rust
/// # use leptos::*;
/// # run_scope(create_runtime(), |cx| {
/// # if !cfg!(any(feature = "csr", feature = "hydrate")) {
/// use leptos::html::Input;
///
/// let (value, set_value) = create_signal(cx, 0);
/// let my_input = create_node_ref::<Input>(cx);
/// view! { cx, <input type="text" _ref=my_input/> }
/// // `my_input` now contains an `Element` that we can use anywhere
/// # ;
/// # }
/// # });
/// ```
///
/// 10. You can add the same class to every element in the view by passing in a special
///    `class = {/* ... */},` argument after `cx, `. This is useful for injecting a class
///    provided by a scoped styling library.
/// ```rust
/// # use leptos::*;
/// # run_scope(create_runtime(), |cx| {
/// # if !cfg!(any(feature = "csr", feature = "hydrate")) {
/// let class = "mycustomclass";
/// view! { cx, class = class,
///   <div> // will have class="mycustomclass"
///     <p>"Some text"</p> // will also have class "mycustomclass"
///   </div>
/// }
/// # ;
/// # }
/// # });
/// ```
///
/// 11. You can set any HTML element’s `innerHTML` with the `inner_html` attribute on an
///     element. Be careful: this HTML will not be escaped, so you should ensure that it
///     only contains trusted input.
/// ```rust
/// # use leptos::*;
/// # run_scope(create_runtime(), |cx| {
/// # if !cfg!(any(feature = "csr", feature = "hydrate")) {
/// let html = "<p>This HTML will be injected.</p>";
/// view! { cx,
///   <div inner_html=html/>
/// }
/// # ;
/// # }
/// # });
/// ```
///
/// Here’s a simple example that shows off several of these features, put together
/// ```rust
/// # use leptos::*;
///
/// # if !cfg!(any(feature = "csr", feature = "hydrate")) {
/// pub fn SimpleCounter(cx: Scope) -> impl IntoView {
///     // create a reactive signal with the initial value
///     let (value, set_value) = create_signal(cx, 0);
///
///     // create event handlers for our buttons
///     // note that `value` and `set_value` are `Copy`, so it's super easy to move them into closures
///     let clear = move |_ev| set_value(0);
///     let decrement = move |_ev| set_value.update(|value| *value -= 1);
///     let increment = move |_ev| set_value.update(|value| *value += 1);
///
///     // this JSX is compiled to an HTML template string for performance
///     view! {
///         cx,
///         <div>
///             <button on:click=clear>"Clear"</button>
///             <button on:click=decrement>"-1"</button>
///             <span>"Value: " {move || value().to_string()} "!"</span>
///             <button on:click=increment>"+1"</button>
///         </div>
///     }
/// }
/// # ;
/// # }
/// ```
#[proc_macro_error::proc_macro_error]
#[proc_macro]
#[cfg_attr(
    any(debug_assertions, feature = "ssr"),
    tracing::instrument(level = "trace", skip_all,)
)]
pub fn view(tokens: TokenStream) -> TokenStream {
    let tokens: proc_macro2::TokenStream = tokens.into();
    let mut tokens = tokens.into_iter();
    let (cx, comma) = (tokens.next(), tokens.next());

    match (cx, comma) {
        (Some(TokenTree::Ident(cx)), Some(TokenTree::Punct(punct)))
            if punct.as_char() == ',' =>
        {
            let first = tokens.next();
            let second = tokens.next();
            let third = tokens.next();
            let fourth = tokens.next();
            let global_class = match (&first, &second) {
                (Some(TokenTree::Ident(first)), Some(TokenTree::Punct(eq)))
                    if *first == "class" && eq.as_char() == '=' =>
                {
                    match &fourth {
                        Some(TokenTree::Punct(comma))
                            if comma.as_char() == ',' =>
                        {
                            third.clone()
                        }
                        _ => {
                            abort!(
                                punct, "To create a scope class with the view! macro you must put a comma `,` after the value";
                                help = r#"e.g., view!{cx, class="my-class", <div>...</div>}"#
                            )
                        }
                    }
                }
                _ => None,
            };
            let tokens = if global_class.is_some() {
                tokens.collect::<proc_macro2::TokenStream>()
            } else {
                [first, second, third, fourth]
                    .into_iter()
                    .flatten()
                    .chain(tokens)
                    .collect()
            };
            let config = rstml::ParserConfig::default().recover_block(true);
            let parser = rstml::Parser::new(config);
            let (nodes, errors)  = parser.parse_recoverable(tokens).split_vec();
            let errors = errors.into_iter().map(|e| e.emit_as_expr_tokens());
            let nodes_output = render_view(
                    &cx,
                    &nodes,
                    Mode::default(),
                    global_class.as_ref(),
                    normalized_call_site(proc_macro::Span::call_site()),
                );
            quote!{
                {
                    #(#errors;)*
                    #nodes_output
                }
            }.into()
        }
        _ => {
            abort_call_site!(
                "view! macro needs a context and RSX: e.g., view! {{ cx, \
                 <div>...</div> }}"
            )
        }
    }
}

fn normalized_call_site(site: proc_macro::Span) -> Option<String> {
    cfg_if::cfg_if! {
        if #[cfg(all(debug_assertions, not(feature = "stable")))] {
            Some(leptos_hot_reload::span_to_stable_id(
                site.source_file().path(),
                site.into()
            ))
        } else {
            _ = site;
            None
        }
    }
}

/// An optimized, cached template for client-side rendering. Follows the same
/// syntax as the [view!] macro. In hydration or server-side rendering mode,
/// behaves exactly as the `view` macro. In client-side rendering mode, uses a `<template>`
/// node to efficiently render the element. Should only be used with a single root element.
#[proc_macro_error::proc_macro_error]
#[proc_macro]
pub fn template(tokens: TokenStream) -> TokenStream {
    if cfg!(feature = "csr") {
        let tokens: proc_macro2::TokenStream = tokens.into();
        let mut tokens = tokens.into_iter();
        let (cx, comma) = (tokens.next(), tokens.next());
        match (cx, comma) {
            (Some(TokenTree::Ident(cx)), Some(TokenTree::Punct(punct)))
                if punct.as_char() == ',' =>
            {
                match parse(tokens.collect::<proc_macro2::TokenStream>().into())
                {
                    Ok(nodes) => render_template(
                        &proc_macro2::Ident::new(&cx.to_string(), cx.span()),
                        &nodes,
                    ),
                    Err(error) => error.to_compile_error(),
                }
                .into()
            }
            _ => {
                abort_call_site!(
                    "view! macro needs a context and RSX: e.g., view! {{ cx, \
                     <div>...</div> }}"
                )
            }
        }
    } else {
        view(tokens)
    }
}

/// Annotates a function so that it can be used with your template as a Leptos `<Component/>`.
///
/// The `#[component]` macro allows you to annotate plain Rust functions as components
/// and use them within your Leptos [view](crate::view!) as if they were custom HTML elements. The
/// component function takes a [Scope](https://docs.rs/leptos/latest/leptos/struct.Scope.html)
/// and any number of other arguments. When you use the component somewhere else,
/// the names of its arguments are the names of the properties you use in the [view](crate::view!) macro.
///
/// Every component function should have the return type `-> impl IntoView`.
///
/// You can add Rust doc comments to component function arguments and the macro will use them to
/// generate documentation for the component.
///
/// Here’s how you would define and use a simple Leptos component which can accept custom properties for a name and age:
/// ```rust
/// # use leptos::*;
/// use std::time::Duration;
///
/// #[component]
/// fn HelloComponent(
///     cx: Scope,
///     /// The user's name.
///     name: String,
///     /// The user's age.
///     age: u8,
/// ) -> impl IntoView {
///     // create the signals (reactive values) that will update the UI
///     let (age, set_age) = create_signal(cx, age);
///     // increase `age` by 1 every second
///     set_interval(
///         move || set_age.update(|age| *age += 1),
///         Duration::from_secs(1),
///     );
///
///     // return the user interface, which will be automatically updated
///     // when signal values change
///     view! { cx,
///       <p>"Your name is " {name} " and you are " {age} " years old."</p>
///     }
/// }
///
/// #[component]
/// fn App(cx: Scope) -> impl IntoView {
///     view! { cx,
///       <main>
///         <HelloComponent name="Greg".to_string() age=32/>
///       </main>
///     }
/// }
/// ```
///
/// The `#[component]` macro creates a struct with a name like `HelloComponentProps`. If you define
/// your component in one module and import it into another, make sure you import this `___Props`
/// struct as well.
///
/// Here are some important details about how Leptos components work within the framework:
/// 1. **The component function only runs once.** Your component function is not a “render” function
///    that re-runs whenever changes happen in the state. It’s a “setup” function that runs once to
///    create the user interface, and sets up a reactive system to update it. This means it’s okay
///    to do relatively expensive work within the component function, as it will only happen once,
///    not on every state change.
///
/// 2. Component names are usually in `PascalCase`. If you use a `snake_case` name,
///    then the generated component's name will still be in `PascalCase`. This is how the framework
///    recognizes that a particular tag is a component, not an HTML element. It's important to be aware
///    of this when using or importing the component.
///
/// ```
/// # use leptos::*;
///
/// // PascalCase: Generated component will be called MyComponent
/// #[component]
/// fn MyComponent(cx: Scope) -> impl IntoView {
///     todo!()
/// }
///
/// // snake_case: Generated component will be called MySnakeCaseComponent
/// #[component]
/// fn my_snake_case_component(cx: Scope) -> impl IntoView {
///     todo!()
/// }
/// ```
///
/// 3. The macro generates a type `ComponentProps` for every `Component` (so, `HomePage` generates `HomePageProps`,
///   `Button` generates `ButtonProps`, etc.) When you’re importing the component, you also need to **explicitly import
///   the prop type.**
///
/// ```
/// # use leptos::*;
///
/// use component::{MyComponent, MyComponentProps};
///
/// mod component {
///     use leptos::*;
///
///     #[component]
///     pub fn MyComponent(cx: Scope) -> impl IntoView {
///         todo!()
///     }
/// }
/// ```
/// ```
/// # use leptos::*;
///
/// use snake_case_component::{
///     MySnakeCaseComponent, MySnakeCaseComponentProps,
/// };
///
/// mod snake_case_component {
///     use leptos::*;
///
///     #[component]
///     pub fn my_snake_case_component(cx: Scope) -> impl IntoView {
///         todo!()
///     }
/// }
/// ```
///
/// 4. You can pass generic arguments, but they should be defined in a `where` clause and not inline.
///
/// ```compile_error
/// // ❌ This won't work.
/// # use leptos::*;
/// use leptos::html::Div;
///
/// #[component]
/// fn MyComponent<T: Fn() -> HtmlElement<Div>>(cx: Scope, render_prop: T) -> impl IntoView {
///   todo!()
/// }
/// ```
///
/// ```
/// // ✅ Do this instead
/// # use leptos::*;
/// use leptos::html::Div;
///
/// #[component]
/// fn MyComponent<T>(cx: Scope, render_prop: T) -> impl IntoView
/// where
///     T: Fn() -> HtmlElement<Div>,
/// {
///     todo!()
/// }
/// ```
///
/// 5. You can access the children passed into the component with the `children` property, which takes
///    an argument of the type `Children`. This is an alias for `Box<dyn FnOnce(Scope) -> Fragment>`.
///    If you need `children` to be a `Fn` or `FnMut`, you can use the `ChildrenFn` or `ChildrenFnMut`
///    type aliases.
///
/// ```
/// # use leptos::*;
/// #[component]
/// fn ComponentWithChildren(cx: Scope, children: Children) -> impl IntoView {
///     view! {
///       cx,
///       <ul>
///         {children(cx)
///           .nodes
///           .into_iter()
///           .map(|child| view! { cx, <li>{child}</li> })
///           .collect::<Vec<_>>()}
///       </ul>
///     }
/// }
///
/// #[component]
/// fn WrapSomeChildren(cx: Scope) -> impl IntoView {
///     view! { cx,
///       <ComponentWithChildren>
///         "Ooh, look at us!"
///         <span>"We're being projected!"</span>
///       </ComponentWithChildren>
///     }
/// }
/// ```
///
/// ## Customizing Properties
/// You can use the `#[prop]` attribute on individual component properties (function arguments) to
/// customize the types that component property can receive. You can use the following attributes:
/// * `#[prop(into)]`: This will call `.into()` on any value passed into the component prop. (For example,
///   you could apply `#[prop(into)]` to a prop that takes
///   [Signal](https://docs.rs/leptos/latest/leptos/struct.Signal.html), which would
///   allow users to pass a [ReadSignal](https://docs.rs/leptos/latest/leptos/struct.ReadSignal.html) or
///   [RwSignal](https://docs.rs/leptos/latest/leptos/struct.RwSignal.html)
///   and automatically convert it.)
/// * `#[prop(optional)]`: If the user does not specify this property when they use the component,
///   it will be set to its default value. If the property type is `Option<T>`, values should be passed
///   as `name=T` and will be received as `Some(T)`.
/// * `#[prop(optional_no_strip)]`: The same as `optional`, but requires values to be passed as `None` or
///   `Some(T)` explicitly. This means that the optional property can be omitted (and be `None`), or explicitly
///   specified as either `None` or `Some(T)`.
/// ```rust
/// # use leptos::*;
///
/// #[component]
/// pub fn MyComponent(
///     cx: Scope,
///     #[prop(into)] name: String,
///     #[prop(optional)] optional_value: Option<i32>,
///     #[prop(optional_no_strip)] optional_no_strip: Option<i32>,
/// ) -> impl IntoView {
///     // whatever UI you need
/// }
///
/// #[component]
/// pub fn App(cx: Scope) -> impl IntoView {
///     view! { cx,
///       <MyComponent
///         name="Greg" // automatically converted to String with `.into()`
///         optional_value=42 // received as `Some(42)`
///         optional_no_strip=Some(42) // received as `Some(42)`
///       />
///       <MyComponent
///         name="Bob" // automatically converted to String with `.into()`
///         // optional values can both be omitted, and received as `None`
///       />
///     }
/// }
/// ```
#[proc_macro_error::proc_macro_error]
#[proc_macro_attribute]
pub fn component(args: proc_macro::TokenStream, s: TokenStream) -> TokenStream {
    let is_transparent = if !args.is_empty() {
        let transparent = parse_macro_input!(args as syn::Ident);

        if transparent != "transparent" {
            abort!(
                transparent,
                "only `transparent` is supported";
                help = "try `#[component(transparent)]` or `#[component]`"
            );
        }

        true
    } else {
        false
    };

    parse_macro_input!(s as component::Model)
        .is_transparent(is_transparent)
        .into_token_stream()
        .into()
}

/// Annotates a struct so that it can be used with your Component as a `slot`.
///
/// The `#[slot]` macro allows you to annotate plain Rust struct as component slots and use them
/// within your Leptos [`component`](macro@crate::component) properties. The struct can contain any number
/// of fields. When you use the component somewhere else, the names of the slot fields are the
/// names of the properties you use in the [view](crate::view!) macro.
///
/// Here’s how you would define and use a simple Leptos component which can accept a custom slot:
/// ```rust
/// # use leptos::*;
/// use std::time::Duration;
///
/// #[slot]
/// struct HelloSlot {
///     // Same prop syntax as components.
///     #[prop(optional)]
///     children: Option<Children>,
/// }
///
/// #[component]
/// fn HelloComponent(
///     cx: Scope,
///     /// Component slot, should be passed through the <HelloSlot slot> syntax.
///     hello_slot: HelloSlot,
/// ) -> impl IntoView {
///     // mirror the children from the slot, if any were passed
///     if let Some(children) = hello_slot.children {
///         (children)(cx).into_view(cx)
///     } else {
///         ().into_view(cx)
///     }
/// }
///
/// #[component]
/// fn App(cx: Scope) -> impl IntoView {
///     view! { cx,
///         <HelloComponent>
///             <HelloSlot slot>
///                 "Hello, World!"
///             </HelloSlot>
///         </HelloComponent>
///     }
/// }
/// ```
///
/// /// Here are some important details about how slots work within the framework:
/// 1. Most of the same rules from [component](crate::component!) macro should also be followed on slots.
///
/// 2. Specifying only `slot` without a name (such as in `<HelloSlot slot>`) will default the chosen slot to
/// the a snake case version of the slot struct name (`hello_slot` for `<HelloSlot>`).
///
/// 3. Event handlers cannot be specified directly on the slot.
///
/// ```compile_error
/// // ❌ This won't work
/// # use leptos::*;
///
/// #[slot]
/// struct SlotWithChildren {
///     children: Children,
/// }
///
/// #[component]
/// fn ComponentWithSlot(cx: Scope, slot: SlotWithChildren) -> impl IntoView {
///     (slot.children)(cx)
/// }
///
/// #[component]
/// fn App(cx: Scope) -> impl IntoView {
///     view! { cx,
///         <ComponentWithSlot>
///           <SlotWithChildren slot:slot on:click=move |_| {}>
///             <h1>"Hello, World!"</h1>
///           </SlotWithChildren>
///         </ComponentWithSlot>
///     }
/// }
/// ```
///
/// ```
/// // ✅ Do this instead
/// # use leptos::*;
///
/// #[slot]
/// struct SlotWithChildren {
///     children: Children,
/// }
///
/// #[component]
/// fn ComponentWithSlot(cx: Scope, slot: SlotWithChildren) -> impl IntoView {
///     (slot.children)(cx)
/// }
///
/// #[component]
/// fn App(cx: Scope) -> impl IntoView {
///     view! { cx,
///         <ComponentWithSlot>
///           <SlotWithChildren slot:slot>
///             <div on:click=move |_| {}>
///               <h1>"Hello, World!"</h1>
///             </div>
///           </SlotWithChildren>
///         </ComponentWithSlot>
///     }
/// }
/// ```
#[proc_macro_error::proc_macro_error]
#[proc_macro_attribute]
pub fn slot(args: proc_macro::TokenStream, s: TokenStream) -> TokenStream {
    if !args.is_empty() {
        abort!(
            Span::call_site(),
            "no arguments are supported";
            help = "try just `#[slot]`"
        );
    }

    parse_macro_input!(s as slot::Model)
        .into_token_stream()
        .into()
}

/// Declares that a function is a [server function](https://docs.rs/server_fn/latest/server_fn/index.html).
/// This means that its body will only run on the server, i.e., when the `ssr` feature is enabled.
///
/// If you call a server function from the client (i.e., when the `csr` or `hydrate` features
/// are enabled), it will instead make a network request to the server.
///
/// You can specify one, two, or three arguments to the server function:
/// 1. **Required**: A type name that will be used to identify and register the server function
///   (e.g., `MyServerFn`).
/// 2. *Optional*: A URL prefix at which the function will be mounted when it’s registered
///   (e.g., `"/api"`). Defaults to `"/"`.
/// 3. *Optional*: either `"Cbor"` (specifying that it should use the binary `cbor` format for
///   serialization) or `"Url"` (specifying that it should be use a URL-encoded form-data string).
///   Defaults to `"Url"`. If you want to use this server function to power a `<form>` that will
///   work without WebAssembly, the encoding must be `"Url"`.
///
/// The server function itself can take any number of arguments, each of which should be serializable
/// and deserializable with `serde`. Optionally, its first argument can be a Leptos
/// [Scope](https://docs.rs/leptos/latest/leptos/struct.Scope.html),
/// which will be injected *on the server side.* This can be used to inject the raw HTTP request or other
/// server-side context into the server function.
///
/// ```ignore
/// # use leptos::*; use serde::{Serialize, Deserialize};
/// # #[derive(Serialize, Deserialize)]
/// # pub struct Post { }
/// #[server(ReadPosts, "/api")]
/// pub async fn read_posts(how_many: u8, query: String) -> Result<Vec<Post>, ServerFnError> {
///   // do some work on the server to access the database
///   todo!()   
/// }
/// ```
///
/// Note the following:
/// - You must **register** the server function by calling `T::register()` somewhere in your main function.
/// - **Server functions must be `async`.** Even if the work being done inside the function body
///   can run synchronously on the server, from the client’s perspective it involves an asynchronous
///   function call.
/// - **Server functions must return `Result<T, ServerFnError>`.** Even if the work being done
///   inside the function body can’t fail, the processes of serialization/deserialization and the
///   network call are fallible.
/// - **Return types must be [Serializable](https://docs.rs/leptos/latest/leptos/trait.Serializable.html).**
///   This should be fairly obvious: we have to serialize arguments to send them to the server, and we
///   need to deserialize the result to return it to the client.
/// - **Arguments must be implement [`Serialize`](https://docs.rs/serde/latest/serde/trait.Serialize.html)
///   and [`DeserializeOwned`](https://docs.rs/serde/latest/serde/de/trait.DeserializeOwned.html).**
///   They are serialized as an `application/x-www-form-urlencoded`
///   form data using [`serde_qs`](https://docs.rs/serde_qs/latest/serde_qs/) or as `application/cbor`
///   using [`cbor`](https://docs.rs/cbor/latest/cbor/). **Note**: You should explicitly include `serde` with the
///   `derive` feature enabled in your `Cargo.toml`. You can do this by running `cargo add serde --features=derive`.
/// - **The `Scope` comes from the server.** Optionally, the first argument of a server function
///   can be a Leptos `Scope`. This scope can be used to inject dependencies like the HTTP request
///   or response or other server-only dependencies, but it does *not* have access to reactive state that exists in the client.
#[proc_macro_attribute]
#[proc_macro_error]
pub fn server(args: proc_macro::TokenStream, s: TokenStream) -> TokenStream {
    let context = ServerContext {
        ty: syn::parse_quote!(Scope),
        path: syn::parse_quote!(::leptos::Scope),
    };
    match server_macro_impl(
        args.into(),
        s.into(),
        Some(context),
        Some(syn::parse_quote!(::leptos::server_fn)),
    ) {
        Err(e) => e.to_compile_error().into(),
        Ok(s) => s.to_token_stream().into(),
    }
}

/// Derives a trait that parses a map of string keys and values into a typed
/// data structure, e.g., for route params.
#[proc_macro_derive(Params, attributes(params))]
pub fn params_derive(
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    match syn::parse(input) {
        Ok(ast) => params::impl_params(&ast),
        Err(err) => err.to_compile_error().into(),
    }
}

pub(crate) fn attribute_value(attr: &KeyedAttribute) -> &syn::Expr {
    match &attr.possible_value {
        Some(value) => &value.value,
        None => abort!(attr.key, "attribute should have value"),
    }
}
