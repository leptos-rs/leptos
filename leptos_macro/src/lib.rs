//! Macros for use with the Leptos framework.

#![cfg_attr(feature = "nightly", feature(proc_macro_span))]
#![forbid(unsafe_code)]
// to prevent warnings from popping up when a nightly feature is stabilized
#![allow(stable_features)]
// FIXME? every use of quote! {} is warning here -- false positive?
#![allow(unknown_lints)]
#![allow(private_macro_use)]
#![deny(missing_docs)]

#[macro_use]
extern crate proc_macro_error2;

use component::DummyModel;
use proc_macro::TokenStream;
use proc_macro2::{Span, TokenTree};
use quote::{quote, ToTokens};
use std::str::FromStr;
use syn::{parse_macro_input, spanned::Spanned, token::Pub, Visibility};

mod params;
mod view;
use crate::component::unmodified_fn_name_from_fn_name;
mod component;
mod lazy;
mod memo;
mod slice;
mod slot;

/// The `view` macro uses RSX (like JSX, but Rust!) It follows most of the
/// same rules as HTML, with the following differences:
///
/// 1. Text content should be provided as a Rust string, i.e., double-quoted:
/// ```rust
/// # use leptos::prelude::*;
/// # fn test() -> impl IntoView {
/// view! { <p>"Here’s some text"</p> }
/// # }
/// ```
///
/// 2. Self-closing tags need an explicit `/` as in XML/XHTML
/// ```rust,compile_fail
/// # use leptos::prelude::*;
///
/// # fn test() -> impl IntoView {
/// // ❌ not like this
/// view! { <input type="text" name="name"> }
/// # ;
/// # }
/// ```
/// ```rust
/// # use leptos::prelude::*;
/// # fn test() -> impl IntoView {
/// // ✅ add that slash
/// view! { <input type="text" name="name" /> }
/// # }
/// ```
///
/// 3. Components (functions annotated with `#[component]`) can be inserted as camel-cased tags. (Generics
///    on components are specified as `<Component<T>/>`, not the turbofish `<Component::<T>/>`.)
/// ```rust
/// # use leptos::prelude::*;
///
/// # #[component]
/// # fn Counter(initial_value: i32) -> impl IntoView { view! { <p></p>} }
/// # fn test() -> impl IntoView {
/// view! { <div><Counter initial_value=3 /></div> }
/// # ;
/// # }
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
///    Note that in some cases, rust-analyzer support may be better if attribute values are surrounded with braces (`{}`).
///    Unlike in JSX, attribute values are not required to be in braces, but braces can be used and may improve this LSP support.
///
/// ```rust,ignore
/// # use leptos::prelude::*;
///
/// # fn test() -> impl IntoView {
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
/// ```
///
/// 5. Event handlers can be added with `on:` attributes. In most cases, the events are given the correct type
///    based on the event name.
/// ```rust
/// # use leptos::prelude::*;
/// # fn test() -> impl IntoView {
/// view! {
///   <button on:click=|ev| {
///     log::debug!("click event: {ev:#?}");
///   }>
///     "Click me"
///   </button>
/// }
/// # }
/// ```
///
/// 6. DOM properties can be set with `prop:` attributes, which take any primitive type or `JsValue` (or a signal
///    that returns a primitive or JsValue). They can also take an `Option`, in which case `Some` sets the property
///    and `None` deletes the property.
/// ```rust
/// # use leptos::prelude::*;
/// # fn test() -> impl IntoView {
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
/// # }
/// ```
///
/// 7. Classes can be toggled with `class:` attributes, which take a `bool` (or a signal that returns a `bool`).
/// ```rust
/// # use leptos::prelude::*;
/// # fn test() -> impl IntoView {
/// let (count, set_count) = create_signal(2);
/// view! { <div class:hidden-div={move || count.get() < 3}>"Now you see me, now you don’t."</div> }
/// # }
/// ```
///
/// Class names can include dashes, and since v0.5.0 can include a dash-separated segment of only numbers.
/// ```rust
/// # use leptos::prelude::*;
/// # fn test() -> impl IntoView {
/// let (count, set_count) = create_signal(2);
/// view! { <div class:hidden-div-25={move || count.get() < 3}>"Now you see me, now you don’t."</div> }
/// # }
/// ```
///
/// Class names cannot include special symbols.
/// ```rust,compile_fail
/// # use leptos::prelude::*;
/// # fn test() -> impl IntoView {
/// let (count, set_count) = create_signal(2);
/// // class:hidden-[div]-25 is invalid attribute name
/// view! { <div class:hidden-[div]-25={move || count.get() < 3}>"Now you see me, now you don’t."</div> }
/// # }
/// ```
///
/// However, you can pass arbitrary class names using the syntax `class=("name", value)`.
/// ```rust
/// # use leptos::prelude::*;
/// # fn test() -> impl IntoView {
/// let (count, set_count) = create_signal(2);
/// // this allows you to use CSS frameworks that include complex class names
/// view! {
///   <div
///     class=("is-[this_-_really]-necessary-42", move || count.get() < 3)
///   >
///     "Now you see me, now you don’t."
///   </div>
/// }
/// # }
/// ```
///
/// 8. Individual styles can also be set with `style:` or `style=("property-name", value)` syntax.
/// ```rust
/// # use leptos::prelude::*;
///
/// # fn test() -> impl IntoView {
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
/// # }
/// ```
///
/// 9. You can use the `node_ref` or `_ref` attribute to store a reference to its DOM element in a
///    [NodeRef](https://docs.rs/leptos/latest/leptos/struct.NodeRef.html) to use later.
/// ```rust
/// # use leptos::prelude::*;
///
/// # fn test() -> impl IntoView {
/// use leptos::html::Input;
///
/// let (value, set_value) = signal(0);
/// let my_input = NodeRef::<Input>::new();
/// view! { <input type="text" node_ref=my_input/> }
/// // `my_input` now contains an `Element` that we can use anywhere
/// # ;
/// # };
/// ```
///
/// 10. You can add the same class to every element in the view by passing in a special
///    `class = {/* ... */},` argument after ``. This is useful for injecting a class
///    provided by a scoped styling library.
/// ```rust
/// # use leptos::prelude::*;
///
/// # fn test() -> impl IntoView {
/// let class = "mycustomclass";
/// view! { class = class,
///   <div> // will have class="mycustomclass"
///     <p>"Some text"</p> // will also have class "mycustomclass"
///   </div>
/// }
/// # }
/// ```
///
/// 11. You can set any HTML element’s `innerHTML` with the `inner_html` attribute on an
///     element. Be careful: this HTML will not be escaped, so you should ensure that it
///     only contains trusted input.
/// ```rust
/// # use leptos::prelude::*;
/// # fn test() -> impl IntoView {
/// let html = "<p>This HTML will be injected.</p>";
/// view! {
///   <div inner_html=html/>
/// }
/// # }
/// ```
///
/// Here’s a simple example that shows off several of these features, put together
/// ```rust
/// # use leptos::prelude::*;
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
/// ```
#[proc_macro_error2::proc_macro_error]
#[proc_macro]
#[cfg_attr(feature = "tracing", tracing::instrument(level = "trace", skip_all))]
pub fn view(tokens: TokenStream) -> TokenStream {
    view_macro_impl(tokens, false)
}

/// The `template` macro behaves like [`view`](view!), except that it wraps the entire tree in a
/// [`ViewTemplate`](https://docs.rs/leptos/0.7.0-gamma3/leptos/prelude/struct.ViewTemplate.html). This optimizes creation speed by rendering
/// most of the view into a `<template>` tag with HTML rendered at compile time, then hydrating it.
/// In exchange, there is a small binary size overhead.
#[proc_macro_error2::proc_macro_error]
#[proc_macro]
#[cfg_attr(feature = "tracing", tracing::instrument(level = "trace", skip_all))]
pub fn template(tokens: TokenStream) -> TokenStream {
    view_macro_impl(tokens, true)
}

fn view_macro_impl(tokens: TokenStream, template: bool) -> TokenStream {
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
    let (mut nodes, errors) = parser.parse_recoverable(tokens).split_vec();
    let errors = errors.into_iter().map(|e| e.emit_as_expr_tokens());
    let nodes_output = view::render_view(
        &mut nodes,
        global_class.as_ref(),
        normalized_call_site(proc_macro::Span::call_site()),
        template,
    );

    // The allow lint needs to be put here instead of at the expansion of
    // view::attribute_value(). Adding this next to the expanded expression
    // seems to break rust-analyzer, but it works when the allow is put here.
    let output = quote! {
        {
            #[allow(unused_braces)]
            {
                #(#errors;)*
                #nodes_output
            }
        }
    };

    if template {
        quote! {
            ::leptos::prelude::ViewTemplate::new(#output)
        }
    } else {
        output
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

/// This behaves like the [`view`](view!) macro, but loads the view from an external file instead of
/// parsing it inline.
///
/// This is designed to allow editing views in a separate file, if this improves a user's workflow.
///
/// The file is loaded and parsed during proc-macro execution, and its path is resolved relative to
/// the crate root rather than relative to the file from which it is called.
#[proc_macro_error2::proc_macro_error]
#[proc_macro]
pub fn include_view(tokens: TokenStream) -> TokenStream {
    let file_name = syn::parse::<syn::LitStr>(tokens).unwrap_or_else(|_| {
        abort!(
            Span::call_site(),
            "the only supported argument is a string literal"
        );
    });
    let file =
        std::fs::read_to_string(file_name.value()).unwrap_or_else(|_| {
            abort!(Span::call_site(), "could not open file");
        });
    let tokens = proc_macro2::TokenStream::from_str(&file)
        .unwrap_or_else(|e| abort!(Span::call_site(), e));
    view(tokens.into())
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
/// # use leptos::prelude::*;
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
/// # use leptos::prelude::*;
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
/// 5. You can access the children passed into the component with the `children` property, which takes
///    an argument of the type `Children`. This is an alias for `Box<dyn FnOnce() -> AnyView<_>>`.
///    If you need `children` to be a `Fn` or `FnMut`, you can use the `ChildrenFn` or `ChildrenFnMut`
///    type aliases. If you want to iterate over the children, you can take `ChildrenFragment`.
///
/// ```
/// # use leptos::prelude::*;
/// #[component]
/// fn ComponentWithChildren(children: ChildrenFragment) -> impl IntoView {
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
/// # use leptos::prelude::*;
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
#[proc_macro_error2::proc_macro_error]
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

    component_macro(s, is_transparent, None)
}

/// Defines a component as an interactive island when you are using the
/// `islands` feature of Leptos. Apart from the macro name,
/// the API is the same as the [`component`](macro@component) macro.
///
/// When you activate the `islands` feature, every `#[component]`
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
/// use leptos::prelude::*;
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
#[proc_macro_error2::proc_macro_error]
#[proc_macro_attribute]
pub fn island(args: proc_macro::TokenStream, s: TokenStream) -> TokenStream {
    let is_transparent = if !args.is_empty() {
        let transparent = parse_macro_input!(args as syn::Ident);

        if transparent != "transparent" {
            abort!(
                transparent,
                "only `transparent` is supported";
                help = "try `#[island(transparent)]` or `#[island]`"
            );
        }

        true
    } else {
        false
    };

    let island_src = s.to_string();
    component_macro(s, is_transparent, Some(island_src))
}

fn component_macro(
    s: TokenStream,
    is_transparent: bool,
    island: Option<String>,
) -> TokenStream {
    let mut dummy = syn::parse::<DummyModel>(s.clone());
    let parse_result = syn::parse::<component::Model>(s);

    if let (Ok(ref mut unexpanded), Ok(model)) = (&mut dummy, parse_result) {
        let expanded = model.is_transparent(is_transparent).with_island(island).into_token_stream();
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
            #[allow(non_snake_case, dead_code, clippy::too_many_arguments, clippy::needless_lifetimes)]
            #unexpanded
        }
    } else {
        match dummy {
            Ok(mut dummy) => {
                dummy.sig.ident = unmodified_fn_name_from_fn_name(&dummy.sig.ident);
                quote! {
                    #[doc(hidden)]
                    #[allow(non_snake_case, dead_code, clippy::too_many_arguments, clippy::needless_lifetimes)]
                    #dummy
                }
            }
            Err(e) => {
                proc_macro_error2::abort!(e.span(), e);
            }
        }
    }.into()
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
/// # use leptos::prelude::*;
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
///     /// Component slot, should be passed through the <HelloSlot slot> syntax.
///     hello_slot: HelloSlot,
/// ) -> impl IntoView {
///     hello_slot.children.map(|children| children())
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
/// # use leptos::prelude::*;
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
/// # use leptos::prelude::*;
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
#[proc_macro_error2::proc_macro_error]
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
/// This means that its body will only run on the server, i.e., when the `ssr` feature on this crate is enabled.
///
/// If you call a server function from the client (i.e., when the `csr` or `hydrate` features
/// are enabled), it will instead make a network request to the server.
///
/// ## Named Arguments
///
/// You can provide any combination of the following named arguments:
/// - `name`: sets the identifier for the server function’s type, which is a struct created
///    to hold the arguments (defaults to the function identifier in PascalCase)
/// - `prefix`: a prefix at which the server function handler will be mounted (defaults to `/api`)
///    your prefix must begin with `/`. Otherwise your function won't be found.
/// - `endpoint`: specifies the exact path at which the server function handler will be mounted,
///   relative to the prefix (defaults to the function name followed by unique hash)
/// - `input`: the encoding for the arguments (defaults to `PostUrl`)
/// - `output`: the encoding for the response (defaults to `Json`)
/// - `client`: a custom `Client` implementation that will be used for this server fn
/// - `encoding`: (legacy, may be deprecated in future) specifies the encoding, which may be one
///   of the following (not case sensitive)
///     - `"Url"`: `POST` request with URL-encoded arguments and JSON response
///     - `"GetUrl"`: `GET` request with URL-encoded arguments and JSON response
///     - `"Cbor"`: `POST` request with CBOR-encoded arguments and response
///     - `"GetCbor"`: `GET` request with URL-encoded arguments and CBOR response
/// - `req` and `res` specify the HTTP request and response types to be used on the server (these
///   should usually only be necessary if you are integrating with a server other than Actix/Axum)
/// - `impl_from`: specifies whether to implement trait `From` for server function's type or not.
///   By default, if a server function only has one argument, the macro automatically implements the `From` trait
///   to convert from the argument type to the server function type, and vice versa, allowing you to convert
///   between them easily. Setting `impl_from` to `false` disables this, which can be necessary for argument types
///   for which this would create a conflicting implementation. (defaults to `true`)
///
/// ```rust,ignore
/// #[server(
///   name = SomeStructName,
///   prefix = "/my_api",
///   endpoint = "my_fn",
///   input = Cbor,
///   output = Json
///   impl_from = true
/// )]
/// pub async fn my_wacky_server_fn(input: Vec<String>) -> Result<usize, ServerFnError> {
///   todo!()
/// }
/// ```
///
/// ## Server Function Encodings
///
/// Server functions are designed to allow a flexible combination of `input` and `output` encodings, the set
/// of which can be found in the [`server_fn::codec`](../server_fn/codec/index.html) module.
///
/// The serialization/deserialization process for server functions consists of a series of steps,
/// each of which is represented by a different trait:
/// 1. [`IntoReq`](../server_fn/codec/trait.IntoReq.html): The client serializes the [`ServerFn`](../server_fn/trait.ServerFn.html) argument type into an HTTP request.
/// 2. The [`Client`](../server_fn/client/trait.Client.html) sends the request to the server.
/// 3. [`FromReq`](../server_fn/codec/trait.FromReq.html): The server deserializes the HTTP request back into the [`ServerFn`](../server_fn/client/trait.Client.html) type.
/// 4. The server calls calls [`ServerFn::run_body`](../server_fn/trait.ServerFn.html#tymethod.run_body) on the data.
/// 5. [`IntoRes`](../server_fn/codec/trait.IntoRes.html): The server serializes the [`ServerFn::Output`](../server_fn/trait.ServerFn.html#associatedtype.Output) type into an HTTP response.
/// 6. The server integration applies any middleware from [`ServerFn::middleware`](../server_fn/middleware/index.html) and responds to the request.
/// 7. [`FromRes`](../server_fn/codec/trait.FromRes.html): The client deserializes the response back into the [`ServerFn::Output`](../server_fn/trait.ServerFn.html#associatedtype.Output) type.
///
/// Whatever encoding is provided to `input` should implement `IntoReq` and `FromReq`. Whatever encoding is provided
/// to `output` should implement `IntoRes` and `FromRes`.
///
/// ## Default Values for Parameters
///
/// Individual function parameters can be annotated with `#[server(default)]`, which will pass
/// through `#[serde(default)]`. This is useful for the empty values of arguments with some
/// encodings. The URL encoding, for example, omits a field entirely if it is an empty `Vec<_>`,
/// but this causes a deserialization error: the correct solution is to add `#[server(default)]`.
/// ```rust,ignore
/// pub async fn with_default_value(#[server(default)] values: Vec<u32>) /* etc. */
/// ```
///
/// ## Important Notes
/// - **Server functions must be `async`.** Even if the work being done inside the function body
///   can run synchronously on the server, from the client’s perspective it involves an asynchronous
///   function call.
/// - **Server functions must return `Result<T, ServerFnError>`.** Even if the work being done
///   inside the function body can’t fail, the processes of serialization/deserialization and the
///   network call are fallible.
///     - [`ServerFnError`](../server_fn/error/enum.ServerFnError.html) can be generic over some custom error type. If so, that type should implement
///       [`FromStr`](std::str::FromStr) and [`Display`](std::fmt::Display), but does not need to implement [`Error`](std::error::Error). This is so the value
///       can be easily serialized and deserialized along with the result.
/// - **Server functions are part of the public API of your application.** A server function is an
///   ad hoc HTTP API endpoint, not a magic formula. Any server function can be accessed by any HTTP
///   client. You should take care to sanitize any data being returned from the function to ensure it
///   does not leak data that should exist only on the server.
/// - **Server functions can’t be generic.** Because each server function creates a separate API endpoint,
///   it is difficult to monomorphize. As a result, server functions cannot be generic (for now?) If you need to use
///   a generic function, you can define a generic inner function called by multiple concrete server functions.
/// - **Arguments and return types must be serializable.** We support a variety of different encodings,
///   but one way or another arguments need to be serialized to be sent to the server and deserialized
///   on the server, and the return type must be serialized on the server and deserialized on the client.
///   This means that the set of valid server function argument and return types is a subset of all
///   possible Rust argument and return types. (i.e., server functions are strictly more limited than
///   ordinary functions.)
/// - **Context comes from the server.** Server functions are provided access to the HTTP request and other relevant
///   server data via the server integrations, but they do *not* have access to reactive state that exists in the client.
/// - Your server must be ready to handle the server functions at the API prefix you list. The easiest way to do this
///   is to use the `handle_server_fns` function from [`leptos_actix`](https://docs.rs/leptos_actix/latest/leptos_actix/fn.handle_server_fns.html)
///   or [`leptos_axum`](https://docs.rs/leptos_axum/latest/leptos_axum/fn.handle_server_fns.html).
/// - **Server functions must have unique paths**. Unique paths are automatically generated for each
///   server function. If you choose to specify a path in the fourth argument, you must ensure that these
///   are unique. You cannot define two server functions with the same URL prefix and endpoint path,
///   even if they have different URL encodings, e.g. a POST method at `/api/foo` and a GET method at `/api/foo`.
#[proc_macro_attribute]
#[proc_macro_error]
pub fn server(args: proc_macro::TokenStream, s: TokenStream) -> TokenStream {
    match server_fn_macro::server_macro_impl(
        args.into(),
        s.into(),
        Some(syn::parse_quote!(::leptos::server_fn)),
        "/api",
        None,
        None,
    ) {
        Err(e) => e.to_compile_error().into(),
        Ok(s) => s.to_token_stream().into(),
    }
}

/// Derives a trait that parses a map of string keys and values into a typed
/// data structure, e.g., for route params.
#[proc_macro_derive(Params)]
pub fn params_derive(
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    match syn::parse(input) {
        Ok(ast) => params::params_impl(&ast),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Generates a `slice` into a struct with a default getter and setter.
///
/// Can be used to access deeply nested fields within a global state object.
///
/// ```rust
/// # use leptos::prelude::*;
/// # use leptos_macro::slice;
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
/// let outer_signal = RwSignal::new(Outer::default());
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

/// Generates a `memo` into a struct with a default getter.
///
/// Can be used to access deeply nested fields within a global state object.
///
/// ```rust
/// # use leptos::prelude::*;
/// # use leptos_macro::memo;
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
/// let outer_signal = RwSignal::new(Outer::default());
///
/// let count = memo!(outer_signal.count);
///
/// let inner_count = memo!(outer_signal.inner.inner_count);
/// let inner_name = memo!(outer_signal.inner.inner_name);
/// ```
#[proc_macro]
pub fn memo(input: TokenStream) -> TokenStream {
    memo::memo_impl(input)
}

/// The `#[lazy]` macro marks an `async` function as a function that can be lazy-loaded from a
/// separate (WebAssembly) binary.
///
/// The first time the function is called, calling the function will first load that other binary,
/// then call the function. On subsequent call it will be called immediately, but still return
/// asynchronously to maintain the same API.
///
/// All parameters and output types should be concrete types, with no generics.
#[proc_macro_attribute]
#[proc_macro_error]
pub fn lazy(args: proc_macro::TokenStream, s: TokenStream) -> TokenStream {
    lazy::lazy_impl(args, s)
}
