#![feature(drain_filter, iter_intersperse)]
#![cfg_attr(not(feature = "stable"), feature(proc_macro_span))]

#[macro_use]
extern crate proc_macro_error;

use proc_macro::{TokenStream, TokenTree};
use quote::ToTokens;
use server::server_macro_impl;
use syn::{parse_macro_input, DeriveInput};
use syn_rsx::{parse, NodeElement};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum Mode {
    Client,
    Ssr,
}

impl Default for Mode {
    fn default() -> Self {
        if cfg!(feature = "hydrate") || cfg!(feature = "csr") || cfg!(feature = "web") {
            Mode::Client
        } else {
            Mode::Ssr
        }
    }
}

mod params;
mod view;
use view::render_view;
mod component;
mod props;
mod server;

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
///   <button on:click=|ev: web_sys::MouseEvent| {
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
/// 8. You can use the `_ref` attribute to store a reference to its DOM element in a 
///    [NodeRef](leptos_reactive::NodeRef) to use later.
/// ```rust
/// # use leptos::*;
/// # run_scope(create_runtime(), |cx| {
/// # if !cfg!(any(feature = "csr", feature = "hydrate")) {
/// let (value, set_value) = create_signal(cx, 0);
/// let my_input = NodeRef::new(cx);
/// view! { cx, <input type="text" _ref=my_input/> }
/// // `my_input` now contains an `Element` that we can use anywhere
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
///     let clear = move |_ev: web_sys::MouseEvent| set_value(0);
///     let decrement = move |_ev: web_sys::MouseEvent| set_value.update(|value| *value -= 1);
///     let increment = move |_ev: web_sys::MouseEvent| set_value.update(|value| *value += 1);
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
pub fn view(tokens: TokenStream) -> TokenStream {
    let mut tokens = tokens.into_iter();
    let (cx, comma) = (tokens.next(), tokens.next());
    match (cx, comma) {
        (Some(TokenTree::Ident(cx)), Some(TokenTree::Punct(punct))) if punct.as_char() == ',' => {
            match parse(tokens.collect()) {
                Ok(nodes) => render_view(
                    &proc_macro2::Ident::new(&cx.to_string(), cx.span().into()),
                    &nodes,
                    // swap to Mode::default() to use faster SSR templating
                    Mode::Client, //Mode::default(),
                ),
                Err(error) => error.to_compile_error(),
            }
            .into()
        }
        _ => {
            panic!("view! macro needs a context and RSX: e.g., view! {{ cx, <div>...</div> }}")
        }
    }
}

/// Annotates a function so that it can be used with your template as a Leptos `<Component/>`.
/// 
/// The `#[component]` macro allows you to annotate plain Rust functions as components
/// and use them within your Leptos [view](mod@view) as if they were custom HTML elements. The 
/// component function takes a [Scope](leptos_reactive::Scope) and any number of other arguments.
/// When you use the component somewhere else, the names of its arguments are the names
/// of the properties you use in the [view](mod@view) macro.
/// 
/// Every component function should have the return type `-> impl [IntoView](leptos_dom::IntoView)`.
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
///   cx: Scope,
///   /// The user's name.
///   name: String,
///   /// The user's age.
///   age: u8
/// ) -> impl IntoView {
///   // create the signals (reactive values) that will update the UI
///   let (age, set_age) = create_signal(cx, age);
///   // increase `age` by 1 every second
///   set_interval(move || {
///     set_age.update(|age| *age += 1)
///   }, Duration::from_secs(1));
///   
///   // return the user interface, which will be automatically updated
///   // when signal values change
///   view! { cx,
///     <p>"Your name is " {name} " and you are " {age} " years old."</p>
///   }
/// }
/// 
/// #[component]
/// fn App(cx: Scope) -> impl IntoView {
///   view! { cx,
///     <main>
///       <HelloComponent name="Greg".to_string() age=32/>
///     </main>
///   }
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
/// 2. The component name should be `CamelCase` instead of `snake_case`. This is how the renderer
///    recognizes that a particular tag is a component, not an HTML element.
///
/// ```
/// # use leptos::*;
/// // ❌ not snake_case
/// #[component]
/// fn my_component(cx: Scope) -> impl IntoView { todo!() }
///
/// // ✅ CamelCase
/// #[component]
/// fn MyComponent(cx: Scope) -> impl IntoView { todo!() }
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
///   use leptos::*;
///
///   #[component]
///   pub fn MyComponent(cx: Scope) -> impl IntoView { todo!() }
/// }
/// ```
///
/// 4. You can pass generic arguments, but they should be defined in a `where` clause and not inline.
///
/// ```compile_error
/// // ❌ This won't work.
/// # use leptos::*;
/// #[component]
/// fn MyComponent<T: Fn() -> HtmlElement<Div>>(cx: Scope, render_prop: T) -> impl IntoView {
///   todo!()
/// }
/// ```
///
/// ```
/// // ✅ Do this instead
/// # use leptos::*;
/// #[component]
/// fn MyComponent<T>(cx: Scope, render_prop: T) -> impl IntoView 
/// where T: Fn() -> HtmlElement<Div> {
///   todo!()
/// }
/// ```
///
/// 5. You can access the children passed into the component with the `children` property, which takes
///    an argument of the form `Box<dyn Fn(Scope) -> Fragment>`.
///
/// ```
/// # use leptos::*;
/// #[component]
/// fn ComponentWithChildren(cx: Scope, children: Box<dyn Fn(Scope) -> Fragment>) -> impl IntoView {
///   view! {
///     cx,
///     <ul>
///       {children(cx)
///         .nodes
///         .into_iter()
///         .map(|child| view! { cx, <li>{child}</li> })
///         .collect::<Vec<_>>()}
///     </ul>
///   }
/// }
///
/// #[component]
/// fn WrapSomeChildren(cx: Scope) -> impl IntoView {
///   view! { cx,
///     <ComponentWithChildren>
///       "Ooh, look at us!"
///       <span>"We're being projected!"</span>
///     </ComponentWithChildren>
///   }
/// }
/// ```
#[proc_macro_error::proc_macro_error]
#[proc_macro_attribute]
pub fn component(args: proc_macro::TokenStream, s: TokenStream) -> TokenStream {
    let is_transparent = if !args.is_empty() {
        let transparent = parse_macro_input!(args as syn::Ident);

        let transparent_token: syn::Ident = syn::parse_quote!(transparent);

        if transparent != transparent_token {
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

#[proc_macro_attribute]
pub fn server(args: proc_macro::TokenStream, s: TokenStream) -> TokenStream {
    match server_macro_impl(args, s.into()) {
        Err(e) => e.to_compile_error().into(),
        Ok(s) => s.to_token_stream().into(),
    }
}

#[proc_macro_derive(Props, attributes(builder))]
pub fn derive_prop(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    props::impl_derive_prop(&input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

// Derive Params trait for routing
#[proc_macro_derive(Params, attributes(params))]
pub fn params_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = syn::parse(input).unwrap();
    params::impl_params(&ast)
}

pub(crate) fn is_component_node(node: &NodeElement) -> bool {
    let name = node.name.to_string();
    let first_char = name.chars().next();
    first_char
        .map(|first_char| first_char.is_ascii_uppercase())
        .unwrap_or(false)
}
