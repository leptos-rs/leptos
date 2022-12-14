#![cfg_attr(not(feature = "stable"), feature(proc_macro_span))]

use proc_macro::{TokenStream, TokenTree};
use quote::ToTokens;
use server::server_macro_impl;
use syn_rsx::{parse, NodeElement};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum Mode {
    Client,
    Hydrate,
    Ssr,
}

impl Default for Mode {
    fn default() -> Self {
        // what's the deal with this order of priority?
        // basically, it's fine for the server to compile wasm-bindgen, but it will panic if it runs it
        // for the sake of testing, we need to fall back to `ssr` if no flags are enabled
        // if you have `hydrate` enabled, you definitely want that rather than `csr`
        // if you have both `csr` and `ssr` we assume you want the browser
        if cfg!(feature = "hydrate") {
            Mode::Hydrate
        } else if cfg!(feature = "csr") {
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
mod server;

/// The `view` macro uses RSX (like JSX, but Rust!) It follows most of the
/// same rules as HTML, with the following differences:
/// 1. Text content should be provided as a Rust string, i.e., double-quoted:
/// ```rust
/// # use leptos_reactive::*; use leptos_dom::*; use leptos_macro::view; use leptos_dom::wasm_bindgen::JsCast;
/// # run_scope(create_runtime(), |cx| {
/// # if !cfg!(any(feature = "csr", feature = "hydrate")) {
/// view! { cx, <p>"Here’s some text"</p> };
/// # }
/// # });
/// ```
///
/// 2. Self-closing tags need an explicit `/` as in XML/XHTML
/// ```rust,compile_fail
/// # use leptos_reactive::*; use leptos_dom::*; use leptos_macro::view; use leptos_dom::wasm_bindgen::JsCast;
/// # run_scope(create_runtime(), |cx| {
/// # if !cfg!(any(feature = "csr", feature = "hydrate")) {
/// // ❌ not like this
/// view! { cx, <input type="text" name="name"> }
/// # ;
/// # }
/// # });
/// ```
/// ```rust
/// # use leptos_reactive::*; use leptos_dom::*; use leptos_macro::view; use leptos_dom::wasm_bindgen::JsCast;
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
/// # use leptos_reactive::*; use leptos_dom::*; use leptos_macro::*; use typed_builder::TypedBuilder; use leptos_dom::wasm_bindgen::JsCast; use leptos_dom as leptos; use leptos_dom::Marker;
/// # #[derive(TypedBuilder)] struct CounterProps { initial_value: i32 }
/// # fn Counter(cx: Scope, props: CounterProps) -> Element { view! { cx, <p></p>} }
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
/// # use leptos_reactive::*; use leptos_dom::*; use leptos_macro::view; use leptos_dom::wasm_bindgen::JsCast; use leptos_dom as leptos; use leptos_dom::Marker;
/// # run_scope(create_runtime(), |cx| {
/// # if !cfg!(any(feature = "csr", feature = "hydrate")) {
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
/// # ;
/// # }
/// # });
/// ```
///
/// 5. Event handlers can be added with `on:` attributes. In most cases, the events are given the correct type
///    based on the event name.
/// ```rust
/// # use leptos_reactive::*; use leptos_dom::*; use leptos_macro::view; use leptos_dom::wasm_bindgen::JsCast;
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
/// # use leptos_reactive::*; use leptos_dom::*; use leptos_macro::view; use leptos_dom::wasm_bindgen::JsCast;
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
/// # use leptos_reactive::*; use leptos_dom::*; use leptos_macro::view; use leptos_dom::wasm_bindgen::JsCast;
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
/// # use leptos_reactive::*; use leptos_dom::*; use leptos_macro::view; use leptos_dom::wasm_bindgen::JsCast;
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
/// # use leptos_reactive::*; use leptos_dom::*; use leptos_macro::view; use leptos_dom::wasm_bindgen::JsCast;
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
/// # use leptos_reactive::*; use leptos_dom::*; use leptos_macro::*; use leptos_dom as leptos; use leptos_dom::Marker; use leptos_dom::wasm_bindgen::JsCast;
///
/// # if !cfg!(any(feature = "csr", feature = "hydrate")) {
/// pub fn SimpleCounter(cx: Scope) -> Element {
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

/// Annotates a function so that it can be used with your template as a Leptos `<Component/>`.
/// 
/// The `#[component]` macro allows you to annotate plain Rust functions that return [Element](leptos_dom::Element)s,
/// and use them within your Leptos [view](mod@view) as if they were custom HTML elements. The 
/// component function takes a [Scope](leptos_reactive::Scope) and any number of other arguments.
/// When you use the component somewhere else, the names of its arguments are the names
/// of the properties you use in the [view](mod@view) macro.
/// 
/// Here’s how you would define and use a simple Leptos component which can accept custom properties for a name and age:
/// ```rust
/// # use leptos::*;
/// use std::time::Duration;
/// 
/// #[component]
/// fn HelloComponent(cx: Scope, name: String, age: u8) -> Element {
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
/// fn App(cx: Scope) -> Element {
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
/// fn my_component(cx: Scope) -> Element { todo!() }
///
/// // ✅ CamelCase
/// #[component]
/// fn MyComponent(cx: Scope) -> Element { todo!() }
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
///   pub fn MyComponent(cx: Scope) -> Element { todo!() }
/// }
/// ```
///
/// 4. You can pass generic arguments, but they should be defined in a `where` clause and not inline.
///
/// ```compile_error
/// // ❌ This won't work.
/// # use leptos::*;
/// #[component]
/// fn MyComponent<T: Fn() -> Element>(cx: Scope, render_prop: T) -> Element {
///   todo!()
/// }
/// ```
///
/// ```
/// // ✅ Do this instead
/// # use leptos::*;
/// #[component]
/// fn MyComponent<T>(cx: Scope, render_prop: T) -> Element where T: Fn() -> Element {
///   todo!()
/// }
/// ```
///
/// 5. You can access the children passed into the component with the `children` property, which takes
///    an argument of the form `Box<dyn Fn() -> Vec<T>>` where `T` is the child type (usually `Element`).
///
/// ```
/// # use leptos::*;
/// #[component]
/// fn ComponentWithChildren(cx: Scope, children: Box<dyn Fn() -> Vec<Element>>) -> Element {
///   // wrap each child in a <strong> element
///   let children = children()
///     .into_iter()
///     .map(|child| view! { cx, <strong>{child}</strong> })
///     .collect::<Vec<_>>();
///
///   // wrap the whole set in a fancy wrapper
///   view! { cx,
///     <p class="fancy-wrapper">{children}</p>
///   }
/// }
///
/// #[component]
/// fn WrapSomeChildren(cx: Scope) -> Element {
///   view! { cx,
///     <ComponentWithChildren>
///       <span>"Ooh, look at us!"</span>
///       <span>"We're being projected!"</span>
///     </ComponentWithChildren>
///   }
/// }
/// ```
///
/// ```
/// # use leptos::*;
/// #[component]
/// fn MyComponent<T>(cx: Scope, render_prop: T) -> Element
/// where
///     T: Fn() -> Element,
/// {
///     todo!()
/// }
/// ```
#[proc_macro_attribute]
pub fn component(_args: proc_macro::TokenStream, s: TokenStream) -> TokenStream {
    match syn::parse::<component::InlinePropsBody>(s) {
        Err(e) => e.to_compile_error().into(),
        Ok(s) => s.to_token_stream().into(),
    }
}

/// Declares that a function is a [server function](leptos_server). This means that 
/// its body will only run on the server, i.e., when the `ssr` feature is enabled.
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
///   Defaults to `"Url"`. If you want to use this server function to power an 
///   [ActionForm](leptos_router::ActionForm) the encoding must be `"Url"`.
///
/// The server function itself can take any number of arguments, each of which should be serializable 
/// and deserializable with `serde`. Optionally, its first argument can be a Leptos [Scope](leptos::Scope),
/// which will be injected *on the server side.* This can be used to inject the raw HTTP request or other
/// server-side context into the server function.
///
/// ```
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
/// - **Return types must be [Serializable](leptos_reactive::Serializable).**
///   This should be fairly obvious: we have to serialize arguments to send them to the server, and we
///   need to deserialize the result to return it to the client.
/// - **Arguments must be implement [serde::Serialize].** They are serialized as an `application/x-www-form-urlencoded`
///   form data using [`serde_urlencoded`](https://docs.rs/serde_urlencoded/latest/serde_urlencoded/) or as `application/cbor`
///   using [`cbor`](https://docs.rs/cbor/latest/cbor/).
/// - **The [Scope](leptos_reactive::Scope) comes from the server.** Optionally, the first argument of a server function
///   can be a Leptos [Scope](leptos_reactive::Scope). This scope can be used to inject dependencies like the HTTP request
///   or response or other server-only dependencies, but it does *not* have access to reactive state that exists in the client.
#[proc_macro_attribute]
pub fn server(args: proc_macro::TokenStream, s: TokenStream) -> TokenStream {
    match server_macro_impl(args, s.into()) {
        Err(e) => e.to_compile_error().into(),
        Ok(s) => s.to_token_stream().into(),
    }
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
