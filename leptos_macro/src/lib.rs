use proc_macro::{TokenStream, TokenTree};
use quote::ToTokens;
use server::server_macro_impl;
use syn::{parse_macro_input, DeriveInput};
use syn_rsx::{parse, Node, NodeType};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum Mode {
    Client,
    Hydrate,
    Ssr,
}

impl Default for Mode {
    fn default() -> Self {
        if cfg!(feature = "ssr") {
            Mode::Ssr
        } else if cfg!(feature = "hydrate") {
            Mode::Hydrate
        } else {
            Mode::Client
        }
        /* else if cfg!(feature = "csr") {
            Mode::Client
        } else {
            panic!("one of the features leptos/ssr, leptos/hydrate, or leptos/csr needs to be set")
        } */
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
/// 1. Text content should be provided as a Rust string, i.e., double-quoted:
/// ```rust
/// # use leptos_reactive::*; use leptos_dom::*; use leptos_macro::view;
/// # run_scope(|cx| {
/// view! { cx, <p>"Here’s some text"</p> }
/// # });
/// ```
///
/// 2. Self-closing tags need an explicit `/` as in XML/XHTML
/// ```rust,compile_fail
/// # use leptos_reactive::*; use leptos_dom::*; use leptos_macro::view;
/// # run_scope(|cx| {
/// // ❌ not like this
/// view! { cx, <input type="text" name="name"> }
/// # });
/// ```
/// ```rust
/// # use leptos_reactive::*; use leptos_dom::*; use leptos_macro::view;
/// # run_scope(|cx| {
/// // ✅ add that slash
/// view! { cx, <input type="text" name="name" /> }
/// # });
/// ```
///
/// 3. Components (functions annotated with `#[component]`) can be inserted as camel-cased tags
/// ```rust
/// # use leptos_reactive::*; use leptos_dom::*; use leptos_macro::*; use typed_builder::TypedBuilder;
/// # #[derive(TypedBuilder)] struct CounterProps { initial_value: i32 }
/// # fn Counter(cx: Scope, props: CounterProps) -> Element { view! { cx, <p></p>} }
/// # run_scope(|cx| {
/// view! { cx, <div><Counter initial_value=3 /></div> }
/// # });
/// ```
///
/// 4. Dynamic content can be wrapped in curly braces (`{ }`) to insert text nodes, elements, or set attributes.
///    If you insert signal here, Leptos will create an effect to update the DOM whenever the value changes.
///    *(“Signal” here means `Fn() -> T` where `T` is the appropriate type for that node: a `String` in case
///    of text nodes, a `bool` for `class:` attributes, etc.)*
/// ```rust
/// # use leptos_reactive::*; use leptos_dom::*; use leptos_macro::view;
/// # run_scope(|cx| {
/// let (count, set_count) = create_signal(cx, 0);
///
/// view! {
///   cx,
///   <div>
///     "Count: " {count} // pass a signal
///     <br/>
///     "Double Count: " {move || count() % 2} // or derive a signal inline
///   </div>
/// }
/// # });
/// ```
///
/// 5. Event handlers can be added with `on:` attributes
/// ```rust
/// # use leptos_reactive::*; use leptos_dom::*; use leptos_macro::view;
/// # run_scope(|cx| {
/// view! {
///   cx,
///   <button on:click=|ev: web_sys::Event| {
///     log::debug!("click event: {ev:#?}");
///   }>
///     "Click me"
///   </button>
/// }
/// # });
/// ```
///
/// 6. DOM properties can be set with `prop:` attributes, which take any primitive type or `JsValue` (or a signal
///    that returns a primitive or JsValue).
/// ```rust
/// # use leptos_reactive::*; use leptos_dom::*; use leptos_macro::view;
/// # run_scope(|cx| {
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
/// # });
/// ```
///
/// 7. Classes can be toggled with `class:` attributes, which take a `bool` (or a signal that returns a `bool`).
/// ```rust
/// # use leptos_reactive::*; use leptos_dom::*; use leptos_macro::view;
/// # run_scope(|cx| {
/// let (count, set_count) = create_signal(cx, 2);
/// view! { cx, <div class:hidden={move || count() < 3}>"Now you see me, now you don’t."</div> }
/// # });
/// ```
///
/// Here’s a simple example that shows off several of these features, put together
/// ```rust
/// # use leptos_reactive::*; use leptos_dom::*; use leptos_macro::*;
///
/// pub fn SimpleCounter(cx: Scope) -> Element {
///     // create a reactive signal with the initial value
///     let (value, set_value) = create_signal(cx, 0);
///
///     // create event handlers for our buttons
///     // note that `value` and `set_value` are `Copy`, so it's super easy to move them into closures
///     let clear = move |_ev: web_sys::Event| set_value(0);
///     let decrement = move |_ev: web_sys::Event| set_value.update(|value| *value -= 1);
///     let increment = move |_ev: web_sys::Event| set_value.update(|value| *value += 1);
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
/// ```
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
                    Mode::default(),
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

#[proc_macro_attribute]
pub fn component(_args: proc_macro::TokenStream, s: TokenStream) -> TokenStream {
    match syn::parse::<component::InlinePropsBody>(s) {
        Err(e) => e.to_compile_error().into(),
        Ok(s) => s.to_token_stream().into(),
    }
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

pub(crate) fn is_component_node(node: &Node) -> bool {
    if let NodeType::Element = node.node_type {
        node.name_as_string()
            .and_then(|node_name| node_name.chars().next())
            .map(|first_char| first_char.is_ascii_uppercase())
            .unwrap_or(false)
    } else {
        false
    }
}
