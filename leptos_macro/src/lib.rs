#![cfg_attr(feature = "nightly", feature(proc_macro_span))]
#![forbid(unsafe_code)]

#[macro_use]
extern crate proc_macro_error;

use component::DummyModel;
use proc_macro::TokenStream;
use proc_macro2::{Span, TokenTree};
use quote::ToTokens;
use rstml::{node::KeyedAttribute, parse};
use syn::{parse_macro_input, spanned::Spanned, token::Pub, Visibility};

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
use crate::component::unmodified_fn_name_from_fn_name;
use view::{client_template::render_template, render_view};
mod component;
mod server;
mod slice;
mod slot;

/// The `view` macro uses RSX (like JSX, but Rust!) It follows most of the
/// same rules as HTML, with the following differences:
///
/// 1. Text content should be provided as a Rust string, i.e., double-quoted:
/// ```rust
/// # use leptos::*;
/// # let runtime = create_runtime();
/// # if !cfg!(any(feature = "csr", feature = "hydrate")) {
/// view! { <p>"Here’s some text"</p> };
/// # }
/// # runtime.dispose();
/// ```
///
/// 2. Self-closing tags need an explicit `/` as in XML/XHTML
/// ```rust,compile_fail
/// # use leptos::*;
/// # let runtime = create_runtime();
/// # if !cfg!(any(feature = "csr", feature = "hydrate")) {
/// // ❌ not like this
/// view! { <input type="text" name="name"> }
/// # ;
/// # }
/// # runtime.dispose();
/// ```
/// ```rust
/// # use leptos::*;
/// # let runtime = create_runtime();
/// # if !cfg!(any(feature = "csr", feature = "hydrate")) {
/// // ✅ add that slash
/// view! { <input type="text" name="name" /> }
/// # ;
/// # }
/// # runtime.dispose();
/// ```
///
/// 3. Components (functions annotated with `#[component]`) can be inserted as camel-cased tags
/// ```rust
/// # use leptos::*;
/// # let runtime = create_runtime();
/// # #[component]
/// # fn Counter(initial_value: i32) -> impl IntoView { view! { <p></p>} }
/// # if !cfg!(any(feature = "csr", feature = "hydrate")) {
/// view! { <div><Counter initial_value=3 /></div> }
/// # ;
/// # }
/// # runtime.dispose();
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
/// ```rust,ignore
/// # use leptos::*;
/// # let runtime = create_runtime();
/// # if !cfg!(any(feature = "csr", feature = "hydrate")) {
/// let (count, set_count) = create_signal(0);
///
/// view! {
///   // ❌ not like this: `count.get()` returns an `i32`, not a function
///   <p>{count.get()}</p>
///   // ✅ this is good: Leptos sees the function and knows it's a dynamic value
///   <p>{move || count.get()}</p>
///   // 🔥 with the `nightly` feature, `count` is a function, so `count` itself can be passed directly into the view
///   <p>{count}</p>
/// }
/// # ;
/// # };
/// # runtime.dispose();
/// ```
///
/// 5. Event handlers can be added with `on:` attributes. In most cases, the events are given the correct type
///    based on the event name.
/// ```rust
/// # use leptos::*;
/// # let runtime = create_runtime();
/// # if !cfg!(any(feature = "csr", feature = "hydrate")) {
/// view! {
///   <button on:click=|ev| {
///     log::debug!("click event: {ev:#?}");
///   }>
///     "Click me"
///   </button>
/// }
/// # ;
/// # };
/// # runtime.dispose();
/// ```
///
/// 6. DOM properties can be set with `prop:` attributes, which take any primitive type or `JsValue` (or a signal
///    that returns a primitive or JsValue). They can also take an `Option`, in which case `Some` sets the property
///    and `None` deletes the property.
/// ```rust
/// # use leptos::*;
/// # let runtime = create_runtime();
/// # if !cfg!(any(feature = "csr", feature = "hydrate")) {
/// let (name, set_name) = create_signal("Alice".to_string());
///
/// view! {
///   <input
///     type="text"
///     name="user_name"
///     value={move || name.get()} // this only sets the default value!
///     prop:value={move || name.get()} // here's how you update values. Sorry, I didn’t invent the DOM.
///     on:click=move |ev| set_name.set(event_target_value(&ev)) // `event_target_value` is a useful little Leptos helper
///   />
/// }
/// # ;
/// # };
/// # runtime.dispose();
/// ```
///
/// 7. Classes can be toggled with `class:` attributes, which take a `bool` (or a signal that returns a `bool`).
/// ```rust
/// # use leptos::*;
/// # let runtime = create_runtime();
/// # if !cfg!(any(feature = "csr", feature = "hydrate")) {
/// let (count, set_count) = create_signal(2);
/// view! { <div class:hidden-div={move || count.get() < 3}>"Now you see me, now you don’t."</div> }
/// # ;
/// # }
/// # runtime.dispose();
/// ```
///
/// Class names can include dashes, and since v0.5.0 can include a dash-separated segment of only numbers.
/// ```rust
/// # use leptos::*;
/// # let runtime = create_runtime();
/// # if !cfg!(any(feature = "csr", feature = "hydrate")) {
/// let (count, set_count) = create_signal(2);
/// view! { <div class:hidden-div-25={move || count.get() < 3}>"Now you see me, now you don’t."</div> }
/// # ;
/// # };
/// # runtime.dispose();
/// ```
///
/// Class names cannot include special symbols.
/// ```rust,compile_fail
/// # use leptos::*;
/// # let runtime = create_runtime();
/// # if !cfg!(any(feature = "csr", feature = "hydrate")) {
/// let (count, set_count) = create_signal(2);
/// // class:hidden-[div]-25 is invalid attribute name
/// view! { <div class:hidden-[div]-25={move || count.get() < 3}>"Now you see me, now you don’t."</div> }
/// # ;
/// # };
/// # runtime.dispose();
/// ```
///
/// However, you can pass arbitrary class names using the syntax `class=("name", value)`.
/// ```rust
/// # use leptos::*;
/// # let runtime = create_runtime();
/// # if !cfg!(any(feature = "csr", feature = "hydrate")) {
/// let (count, set_count) = create_signal(2);
/// // this allows you to use CSS frameworks that include complex class names
/// view! {
///   <div
///     class=("is-[this_-_really]-necessary-42", move || count.get() < 3)
///   >
///     "Now you see me, now you don’t."
///   </div>
/// }
/// # ;
/// # };
/// # runtime.dispose();
/// ```
///
/// 8. Individual styles can also be set with `style:` or `style=("property-name", value)` syntax.
/// ```rust
/// # use leptos::*;
/// # let runtime = create_runtime();
/// # if !cfg!(any(feature = "csr", feature = "hydrate")) {
/// let (x, set_x) = create_signal(0);
/// let (y, set_y) = create_signal(0);
/// view! {
///   <div
///     style="position: absolute"
///     style:left=move || format!("{}px", x.get())
///     style:top=move || format!("{}px", y.get())
///     style=("background-color", move || format!("rgb({}, {}, 100)", x.get(), y.get()))
///   >
///     "Moves when coordinates change"
///   </div>
/// }
/// # ;
/// # };
/// # runtime.dispose();
/// ```
///
/// 9. You can use the `node_ref` or `_ref` attribute to store a reference to its DOM element in a
///    [NodeRef](https://docs.rs/leptos/latest/leptos/struct.NodeRef.html) to use later.
/// ```rust
/// # use leptos::*;
/// # let runtime = create_runtime();
/// # if !cfg!(any(feature = "csr", feature = "hydrate")) {
/// use leptos::html::Input;
///
/// let (value, set_value) = create_signal(0);
/// let my_input = create_node_ref::<Input>();
/// view! { <input type="text" _ref=my_input/> }
/// // `my_input` now contains an `Element` that we can use anywhere
/// # ;
/// # };
/// # runtime.dispose();
/// ```
///
/// 10. You can add the same class to every element in the view by passing in a special
///    `class = {/* ... */},` argument after ``. This is useful for injecting a class
///    provided by a scoped styling library.
/// ```rust
/// # use leptos::*;
/// # let runtime = create_runtime();
/// # if !cfg!(any(feature = "csr", feature = "hydrate")) {
/// let class = "mycustomclass";
/// view! { class = class,
///   <div> // will have class="mycustomclass"
///     <p>"Some text"</p> // will also have class "mycustomclass"
///   </div>
/// }
/// # ;
/// # };
/// # runtime.dispose();
/// ```
///
/// 11. You can set any HTML element’s `innerHTML` with the `inner_html` attribute on an
///     element. Be careful: this HTML will not be escaped, so you should ensure that it
///     only contains trusted input.
/// ```rust
/// # use leptos::*;
/// # let runtime = create_runtime();
/// # if !cfg!(any(feature = "csr", feature = "hydrate")) {
/// let html = "<p>This HTML will be injected.</p>";
/// view! {
///   <div inner_html=html/>
/// }
/// # ;
/// # };
/// # runtime.dispose();
/// ```
///
/// Here’s a simple example that shows off several of these features, put together
/// ```rust
/// # use leptos::*;
///
/// # if !cfg!(any(feature = "csr", feature = "hydrate")) {
/// pub fn SimpleCounter() -> impl IntoView {
///     // create a reactive signal with the initial value
///     let (value, set_value) = create_signal(0);
///
///     // create event handlers for our buttons
///     // note that `value` and `set_value` are `Copy`, so it's super easy to move them into closures
///     let clear = move |_ev| set_value.set(0);
///     let decrement = move |_ev| set_value.update(|value| *value -= 1);
///     let increment = move |_ev| set_value.update(|value| *value += 1);
///
///     view! {
///         <div>
///             <button on:click=clear>"Clear"</button>
///             <button on:click=decrement>"-1"</button>
///             <span>"Value: " {move || value.get().to_string()} "!"</span>
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

    let first = tokens.next();
    let second = tokens.next();
    let third = tokens.next();
    let fourth = tokens.next();
    let global_class = match (&first, &second) {
        (Some(TokenTree::Ident(first)), Some(TokenTree::Punct(eq)))
            if *first == "class" && eq.as_char() == '=' =>
        {
            match &fourth {
                Some(TokenTree::Punct(comma)) if comma.as_char() == ',' => {
                    third.clone()
                }
                _ => {
                    abort!(
                        second, "To create a scope class with the view! macro you must put a comma `,` after the value";
                        help = r#"e.g., view!{ class="my-class", <div>...</div>}"#
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
    let (nodes, errors) = parser.parse_recoverable(tokens).split_vec();
    let errors = errors.into_iter().map(|e| e.emit_as_expr_tokens());
    let nodes_output = render_view(
        &nodes,
        Mode::default(),
        global_class.as_ref(),
        normalized_call_site(proc_macro::Span::call_site()),
    );
    quote! {
        {
            #(#errors;)*
            #nodes_output
        }
    }
    .into()
}

fn normalized_call_site(site: proc_macro::Span) -> Option<String> {
    cfg_if::cfg_if! {
        if #[cfg(all(debug_assertions, feature = "nightly"))] {
            Some(leptos_hot_reload::span_to_stable_id(
                site.source_file().path(),
                site.start().line()
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
        match parse(tokens) {
            Ok(nodes) => render_template(&nodes),
            Err(error) => error.to_compile_error(),
        }
        .into()
    } else {
        view(tokens)
    }
}

/// Annotates a function so that it can be used with your template as a Leptos `<Component/>`.
///
/// The `#[component]` macro allows you to annotate plain Rust functions as components
/// and use them within your Leptos [view](crate::view!) as if they were custom HTML elements. The
/// component function takes any number of other arguments. When you use the component somewhere else,
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
///     /// The user's name.
///     name: String,
///     /// The user's age.
///     age: u8,
/// ) -> impl IntoView {
///     // create the signals (reactive values) that will update the UI
///     let (age, set_age) = create_signal(age);
///     // increase `age` by 1 every second
///     set_interval(
///         move || set_age.update(|age| *age += 1),
///         Duration::from_secs(1),
///     );
///
///     // return the user interface, which will be automatically updated
///     // when signal values change
///     view! {
///       <p>"Your name is " {name} " and you are " {move || age.get()} " years old."</p>
///     }
/// }
///
/// #[component]
/// fn App() -> impl IntoView {
///     view! {
///       <main>
///         <HelloComponent name="Greg".to_string() age=32/>
///       </main>
///     }
/// }
/// ```
///
/// Here are some important details about how Leptos components work within the framework:
/// * **The component function only runs once.** Your component function is not a “render” function
///    that re-runs whenever changes happen in the state. It’s a “setup” function that runs once to
///    create the user interface, and sets up a reactive system to update it. This means it’s okay
///    to do relatively expensive work within the component function, as it will only happen once,
///    not on every state change.
///
/// * Component names are usually in `PascalCase`. If you use a `snake_case` name, then the generated
///    component's name will still be in `PascalCase`. This is how the framework recognizes that
///    a particular tag is a component, not an HTML element.
///
/// ```
/// # use leptos::*;
///
/// // PascalCase: Generated component will be called MyComponent
/// #[component]
/// fn MyComponent() -> impl IntoView {}
///
/// // snake_case: Generated component will be called MySnakeCaseComponent
/// #[component]
/// fn my_snake_case_component() -> impl IntoView {}
/// ```
///
/// * You can pass generic arguments, and they can either be defined in a `where` clause
/// or inline in the generic block, but not in an `impl` in function argument position.
///
/// ```compile_error
/// // ❌ This won't work.
/// # use leptos::*;
/// use leptos::html::Div;
///
/// #[component]
/// fn MyComponent<T: Fn() -> HtmlElement<Div>>(render_prop: impl Fn() -> HtmlElement<Div>) -> impl IntoView {
/// }
/// ```
///
/// ```
/// // ✅ Do this instead
/// # use leptos::*;
/// use leptos::html::Div;
///
/// #[component]
/// fn MyComponent<T>(render_prop: T) -> impl IntoView
/// where
///     T: Fn() -> HtmlElement<Div>,
/// {
/// }
///
/// // or
/// #[component]
/// fn MyComponent2<T: Fn() -> HtmlElement<Div>>(
///     render_prop: T,
/// ) -> impl IntoView {
/// }
/// ```
///
/// 5. You can access the children passed into the component with the `children` property, which takes
///    an argument of the type `Children`. This is an alias for `Box<dyn FnOnce() -> Fragment>`.
///    If you need `children` to be a `Fn` or `FnMut`, you can use the `ChildrenFn` or `ChildrenFnMut`
///    type aliases.
///
/// ```
/// # use leptos::*;
/// #[component]
/// fn ComponentWithChildren(children: Children) -> impl IntoView {
///     view! {
///       <ul>
///         {children()
///           .nodes
///           .into_iter()
///           .map(|child| view! { <li>{child}</li> })
///           .collect::<Vec<_>>()}
///       </ul>
///     }
/// }
///
/// #[component]
/// fn WrapSomeChildren() -> impl IntoView {
///     view! {
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
///     #[prop(into)] name: String,
///     #[prop(optional)] optional_value: Option<i32>,
///     #[prop(optional_no_strip)] optional_no_strip: Option<i32>,
/// ) -> impl IntoView {
///     // whatever UI you need
/// }
///
/// #[component]
/// pub fn App() -> impl IntoView {
///     view! {
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

    let mut dummy = syn::parse::<DummyModel>(s.clone());
    let parse_result = syn::parse::<component::Model>(s);

    if let (Ok(ref mut unexpanded), Ok(model)) = (&mut dummy, parse_result) {
        let expanded = model.is_transparent(is_transparent).into_token_stream();
        unexpanded.sig.ident =
            unmodified_fn_name_from_fn_name(&unexpanded.sig.ident);
        quote! {
            #expanded
            #[doc(hidden)]
            #[allow(non_snake_case, dead_code, clippy::too_many_arguments)]
            #unexpanded
        }
    } else if let Ok(mut dummy) = dummy {
        dummy.sig.ident = unmodified_fn_name_from_fn_name(&dummy.sig.ident);
        quote! {
            #[doc(hidden)]
            #[allow(non_snake_case, dead_code, clippy::too_many_arguments)]
            #dummy
        }
    } else {
        quote! {}
    }
    .into()
}

/// Defines a component as an interactive island when you are using the
/// `experimental-islands` feature of Leptos. Apart from the macro name,
/// the API is the same as the [`component`](macro@component) macro.
///
/// When you activate the `experimental-islands` feature, every `#[component]`
/// is server-only by default. This "default to server" behavior is important:
/// you opt into shipping code to the client, rather than opting out. You can
/// opt into client-side interactivity for any given component by changing from
///  `#[component]` to `#[island]`—the two macros are otherwise identical.
///
/// Everything that is included inside an island will be compiled to WASM and
/// shipped to the browser. So the key to really benefiting from this architecture
/// is to make islands as small as possible, and include only the minimal
/// required amount of functionality in islands themselves.
///
/// Only code included in an island itself is compiled to WASM. This means:
/// 1. `children` can be provided from a server `#[component]` to an `#[island]`
/// without the island needing to be able to hydrate them.
/// 2. Props can be passed from the server to an island.
///
/// ## Present Limitations
/// A few noteworthy limitations, at the moment:
/// 1. `children` are completely opaque in islands. You can't iterate over `children`;
/// in fact they're all bundled into a single `<leptos-children>` HTML element.
/// 2. Similarly, `children` need to be used in the HTML rendered on the server.
/// If they need to be displayed conditionally, they should be included in the HTML
/// and rendered or not using `display: none` rather than `<Show>` or ordinary control flow.
/// This is because the children aren't serialized at all, other than as HTML: if that
/// HTML isn't present in the DOM, even if hidden, it is never sent and not available
/// to the client at all.
///
/// ## Example
/// ```rust,ignore
/// use leptos::*;
///
/// #[component]
/// pub fn App() -> impl IntoView {
///     // this would panic if it ran in the browser
///     // but because this isn't an island, it only runs on the server
///     let file =
///         std::fs::read_to_string("./src/is_this_a_server_component.txt")
///             .unwrap();
///     let len = file.len();
///
///     view! {
///         <p>"The starting value for the button is the file's length."</p>
///         // `value` is serialized and given to the island as a prop
///         <Island value=len>
///             // `file` is only available on the server
///             // island props are projected in, so we can nest
///             // server-only content inside islands inside server content etc.
///             <p>{file}</p>
///         </Island>
///     }
/// }
///
/// #[island]
/// pub fn Island(
///     #[prop(into)] value: RwSignal<usize>,
///     children: Children,
/// ) -> impl IntoView {
///     // because `RwSignal<T>` implements `From<T>`, we can pass in a plain
///     // value and use it as the starting value of a signal here
///     view! {
///         <button on:click=move |_| value.update(|n| *n += 1)>
///             {value}
///         </button>
///         {children()}
///     }
/// }
/// ```
#[proc_macro_error::proc_macro_error]
#[proc_macro_attribute]
pub fn island(_args: proc_macro::TokenStream, s: TokenStream) -> TokenStream {
    let mut dummy = syn::parse::<DummyModel>(s.clone());
    let parse_result = syn::parse::<component::Model>(s);

    if let (Ok(ref mut unexpanded), Ok(model)) = (&mut dummy, parse_result) {
        let expanded = model.is_island().into_token_stream();
        if !matches!(unexpanded.vis, Visibility::Public(_)) {
            unexpanded.vis = Visibility::Public(Pub {
                span: unexpanded.vis.span(),
            })
        }
        unexpanded.sig.ident =
            unmodified_fn_name_from_fn_name(&unexpanded.sig.ident);
        quote! {
            #expanded
            #[doc(hidden)]
            #[allow(non_snake_case, dead_code, clippy::too_many_arguments)]
            #unexpanded
        }
    } else if let Ok(mut dummy) = dummy {
        dummy.sig.ident = unmodified_fn_name_from_fn_name(&dummy.sig.ident);
        quote! {
            #[doc(hidden)]
            #[allow(non_snake_case, dead_code, clippy::too_many_arguments)]
            #dummy
        }
    } else {
        quote! {}
    }
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
///     
///     /// Component slot, should be passed through the <HelloSlot slot> syntax.
///     hello_slot: HelloSlot,
/// ) -> impl IntoView {
///     // mirror the children from the slot, if any were passed
///     if let Some(children) = hello_slot.children {
///         (children)().into_view()
///     } else {
///         ().into_view()
///     }
/// }
///
/// #[component]
/// fn App() -> impl IntoView {
///     view! {
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
/// 1. Most of the same rules from [`macro@component`] macro should also be followed on slots.
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
/// fn ComponentWithSlot(slot: SlotWithChildren) -> impl IntoView {
///     (slot.children)()
/// }
///
/// #[component]
/// fn App() -> impl IntoView {
///     view! {
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
/// fn ComponentWithSlot(slot: SlotWithChildren) -> impl IntoView {
///     (slot.children)()
/// }
///
/// #[component]
/// fn App() -> impl IntoView {
///     view! {
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
/// You can specify one, two, three, or four arguments to the server function. All of these arguments are optional.
/// 1. **`name`**: A type name that will be used to identify and register the server function
///   (e.g., `MyServerFn`). Defaults to a PascalCased version of the function name.
/// 2. **`prefix`**: A URL prefix at which the function will be mounted when it’s registered
///   (e.g., `"/api"`). Defaults to `"/api"`.
/// 3. **`encoding`**: The encoding for the server function (`"Url"`, `"Cbor"`, `"GetJson"`, or `"GetCbor`". See **Server Function Encodings** below.)
/// 4. **`endpoint`**: A specific endpoint path to be used in the URL. (By default, a unique path will be generated.)
///
/// ```rust,ignore
/// // will generate a server function at `/api-prefix/hello`
/// #[server(MyServerFnType, "/api-prefix", "Url", "hello")]
/// pub async fn my_server_fn_type() /* ... */
///
/// // will generate a server function with struct `HelloWorld` and path
/// // `/api/hello2349232342342` (hash based on location in source)
/// #[server]
/// pub async fn hello_world() /* ... */
///
/// // The server function accepts keyword parameters
/// #[server(endpoint = "my_endpoint")]
/// pub async fn hello_leptos() /* ... */
/// ```
///
/// The server function itself can take any number of arguments, each of which should be serializable
/// and deserializable with `serde`.
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
/// - **Server functions must be `async`.** Even if the work being done inside the function body
///   can run synchronously on the server, from the client’s perspective it involves an asynchronous
///   function call.
/// - **Server functions must return `Result<T, ServerFnError>`.** Even if the work being done
///   inside the function body can’t fail, the processes of serialization/deserialization and the
///   network call are fallible.
/// - **Return types must implement [`Serialize`](https://docs.rs/serde/latest/serde/trait.Serialize.html).**
///   This should be fairly obvious: we have to serialize arguments to send them to the server, and we
///   need to deserialize the result to return it to the client.
/// - **Arguments must implement [`Serialize`](https://docs.rs/serde/latest/serde/trait.Serialize.html)
///   and [`DeserializeOwned`](https://docs.rs/serde/latest/serde/de/trait.DeserializeOwned.html).**
///   They are serialized as an `application/x-www-form-urlencoded`
///   form data using [`serde_qs`](https://docs.rs/serde_qs/latest/serde_qs/) or as `application/cbor`
///   using [`cbor`](https://docs.rs/cbor/latest/cbor/). **Note**: You should explicitly include `serde` with the
///   `derive` feature enabled in your `Cargo.toml`. You can do this by running `cargo add serde --features=derive`.
/// - **Context comes from the server.** Server functions are provided access to the HTTP request and other relevant
///   server data via the server integrations, but they do *not* have access to reactive state that exists in the client.
/// - Your server must be ready to handle the server functions at the API prefix you list. The easiest way to do this
///   is to use the `handle_server_fns` function from [`leptos_actix`](https://docs.rs/leptos_actix/latest/leptos_actix/fn.handle_server_fns.html)
///   or [`leptos_axum`](https://docs.rs/leptos_axum/latest/leptos_axum/fn.handle_server_fns.html).
/// - **Server functions must have unique paths**. Unique paths are automatically generated for each
///   server function. If you choose to specify a path in the fourth argument, you must ensure that these
///   are unique. You cannot define two server functions with the same URL prefix and endpoint path,
///   even if they have different URL encodings, e.g. a POST method at `/api/foo` and a GET method at `/api/foo`.
///
/// ## Server Function Encodings
///
/// By default, the server function call is a `POST` request that serializes the arguments as URL-encoded form data in the body
/// of the request. But there are a few other methods supported. Optionally, we can provide another argument to the `#[server]`
/// macro to specify an alternate encoding:
///
/// ```rust,ignore
/// #[server(AddTodo, "/api", "Url")]
/// #[server(AddTodo, "/api", "GetJson")]
/// #[server(AddTodo, "/api", "Cbor")]
/// #[server(AddTodo, "/api", "GetCbor")]
/// ```
///
/// The four options use different combinations of HTTP verbs and encoding methods:
///
/// | Name              | Method | Request     | Response |
/// | ----------------- | ------ | ----------- | -------- |
/// | **Url** (default) | POST   | URL encoded | JSON     |
/// | **GetJson**       | GET    | URL encoded | JSON     |
/// | **Cbor**          | POST   | CBOR        | CBOR     |
/// | **GetCbor**       | GET    | URL encoded | CBOR     |
///
/// In other words, you have two choices:
///
/// - `GET` or `POST`? This has implications for things like browser or CDN caching; while `POST` requests should not be cached,
/// `GET` requests can be.
/// - Plain text (arguments sent with URL/form encoding, results sent as JSON) or a binary format (CBOR, encoded as a base64
/// string)?
///
/// ## Why not `PUT` or `DELETE`? Why URL/form encoding, and not JSON?**
///
/// These are reasonable questions. Much of the web is built on REST API patterns that encourage the use of semantic HTTP
/// methods like `DELETE` to delete an item from a database, and many devs are accustomed to sending data to APIs in the
/// JSON format.
///
/// The reason we use `POST` or `GET` with URL-encoded data by default is the `<form>` support. For better or for worse,
/// HTML forms don’t support `PUT` or `DELETE`, and they don’t support sending JSON. This means that if you use anything
/// but a `GET` or `POST` request with URL-encoded data, it can only work once WASM has loaded.
///
/// The CBOR encoding is suported for historical reasons; an earlier version of server functions used a URL encoding that
/// didn’t support nested objects like structs or vectors as server function arguments, which CBOR did. But note that the
/// CBOR forms encounter the same issue as `PUT`, `DELETE`, or JSON: they do not degrade gracefully if the WASM version of
/// your app is not available.
#[proc_macro_attribute]
#[proc_macro_error]
pub fn server(args: proc_macro::TokenStream, s: TokenStream) -> TokenStream {
    server::server_impl(args, s)
}

/// Derives a trait that parses a map of string keys and values into a typed
/// data structure, e.g., for route params.
#[proc_macro_derive(Params, attributes(params))]
pub fn params_derive(
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    match syn::parse(input) {
        Ok(ast) => params::params_impl(&ast),
        Err(err) => err.to_compile_error().into(),
    }
}

pub(crate) fn attribute_value(attr: &KeyedAttribute) -> &syn::Expr {
    match attr.value() {
        Some(value) => value,
        None => abort!(attr.key, "attribute should have value"),
    }
}

/// Generates a `slice` into a struct with a default getter and setter.
///
/// Can be used to access deeply nested fields within a global state object.
///
/// ```rust
/// # use leptos::{create_runtime, create_rw_signal};
/// # use leptos_macro::slice;
/// # let runtime = create_runtime();
///
/// #[derive(Default)]
/// pub struct Outer {
///     count: i32,
///     inner: Inner,
/// }
///
/// #[derive(Default)]
/// pub struct Inner {
///     inner_count: i32,
///     inner_name: String,
/// }
///
/// let outer_signal = create_rw_signal(Outer::default());
///
/// let (count, set_count) = slice!(outer_signal.count);
///
/// let (inner_count, set_inner_count) = slice!(outer_signal.inner.inner_count);
/// let (inner_name, set_inner_name) = slice!(outer_signal.inner.inner_name);
/// ```
#[proc_macro]
pub fn slice(input: TokenStream) -> TokenStream {
    slice::slice_impl(input)
}
