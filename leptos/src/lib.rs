#![deny(missing_docs)]
#![forbid(unsafe_code)]
//! # About Leptos
//!
//! Leptos is a full-stack framework for building web applications in Rust. You can use it to build
//! - single-page apps (SPAs) rendered entirely in the browser, using client-side routing and loading
//!   or mutating data via async requests to the server
//! - multi-page apps (MPAs) rendered on the server, managing navigation, data, and mutations via
//!   web-standard `<a>` and `<form>` tags
//! - progressively-enhanced single-page apps that are rendered on the server and then hydrated on the client,
//!   enhancing your `<a>` and `<form>` navigations and mutations seamlessly when WASM is available.
//!
//! And you can do all three of these **using the same Leptos code.**
//!
//! # `nightly` Note
//! Most of the examples assume you’re using `nightly` Rust. If you’re on stable, note the following:
//! 1. You need to enable the `"stable"` flag in `Cargo.toml`: `leptos = { version = "0.0", features = ["stable"] }`
//! 2. `nightly` enables the function call syntax for accessing and setting signals. If you’re using `stable`,
//!    you’ll just call `.get()`, `.set()`, or `.update()` manually. Check out the
//!    [`counters_stable` example](https://github.com/leptos-rs/leptos/blob/main/examples/counters_stable/src/main.rs)
//!    for examples of the correct API.
//!
//! # Learning by Example
//!
//! If you want to see what Leptos is capable of, check out
//! the [examples](https://github.com/leptos-rs/leptos/tree/main/examples):
//! - [`counter`](https://github.com/leptos-rs/leptos/tree/main/examples/counter) is the classic
//!   counter example, showing the basics of client-side rendering and reactive DOM updates
//! - [`counter_without_macros`](https://github.com/leptos-rs/leptos/tree/main/examples/counter_without_macros)
//!   adapts the counter example to use the builder pattern for the UI and avoids other macros, instead showing
//!   the code that Leptos generates.
//! - [`counters`](https://github.com/leptos-rs/leptos/tree/main/examples/counters) introduces parent-child
//!   communication via contexts, and the `<For/>` component for efficient keyed list updates.
//! - [`counters_stable`](https://github.com/leptos-rs/leptos/tree/main/examples/counters_stable) adapts the `counters` example
//!   to show how to use Leptos with `stable` Rust.
//! - [`error_boundary`](https://github.com/leptos-rs/leptos/tree/main/examples/error_boundary) shows how to use
//!   `Result` types to handle errors.
//! - [`parent_child`](https://github.com/leptos-rs/leptos/tree/main/examples/parent_child) shows four different
//!   ways a parent component can communicate with a child, including passing a closure, context, and more
//! - [`fetch`](https://github.com/leptos-rs/leptos/tree/main/examples/fetch) introduces
//!   [Resource](leptos_reactive::Resource)s, which allow you to integrate arbitrary `async` code like an
//!   HTTP request within your reactive code.
//! - [`router`](https://github.com/leptos-rs/leptos/tree/main/examples/router) shows how to use Leptos’s nested router
//!   to enable client-side navigation and route-specific, reactive data loading.
//! - [`slots`](https://github.com/leptos-rs/leptos/tree/main/examples/slots) shows how to use slots on components.
//! - [`counter_isomorphic`](https://github.com/leptos-rs/leptos/tree/main/examples/counter_isomorphic) shows
//!   different methods of interaction with a stateful server, including server functions, server actions, forms,
//!   and server-sent events (SSE).
//! - [`todomvc`](https://github.com/leptos-rs/leptos/tree/main/examples/todomvc) shows the basics of building an
//!   isomorphic web app. Both the server and the client import the same app code from the `todomvc` example.
//!   The server renders the app directly to an HTML string, and the client hydrates that HTML to make it interactive.
//!   You might also want to
//!   see how we use [create_effect] to [serialize JSON to `localStorage`](https://github.com/leptos-rs/leptos/blob/16f084a71268ac325fbc4a5e50c260df185eadb6/examples/todomvc/src/lib.rs#L164)
//!   and [reactively call DOM methods](https://github.com/leptos-rs/leptos/blob/6d7c36655c9e7dcc3a3ad33d2b846a3f00e4ae74/examples/todomvc/src/lib.rs#L291)
//!   on [references to elements](https://github.com/leptos-rs/leptos/blob/6d7c36655c9e7dcc3a3ad33d2b846a3f00e4ae74/examples/todomvc/src/lib.rs#L254).
//! - [`hackernews`](https://github.com/leptos-rs/leptos/tree/main/examples/hackernews)
//!   and [`hackernews_axum`](https://github.com/leptos-rs/leptos/tree/main/examples/hackernews_axum)
//!   integrate calls to a real external REST API, routing, server-side rendering and hydration to create
//!   a fully-functional application that works as intended even before WASM has loaded and begun to run.
//! - [`todo_app_sqlite`](https://github.com/leptos-rs/leptos/tree/main/examples/todo_app_sqlite),
//!   [`todo_app_sqlite_axum`](https://github.com/leptos-rs/leptos/tree/main/examples/todo_app_sqlite_axum), and
//!   [`todo_app_sqlite_viz`](https://github.com/leptos-rs/leptos/tree/main/examples/todo_app_sqlite_viz)
//!   show how to build a full-stack app using server functions and database connections.
//! - [`tailwind`](https://github.com/leptos-rs/leptos/tree/main/examples/tailwind) shows how to integrate
//!   TailwindCSS with `cargo-leptos`.
//!
//! Details on how to run each example can be found in its README.
//!
//! # Quick Links
//!
//! Here are links to the most important sections of the docs:
//! - **Reactivity**: the [leptos_reactive] overview, and more details in
//!   - signals: [create_signal], [ReadSignal], and [WriteSignal] (and [create_rw_signal] and [RwSignal])
//!   - computations: [create_memo] and [Memo]
//!   - `async` interop: [create_resource] and [Resource] for loading data using `async` functions,
//!     and [create_action] and [Action] to mutate data or imperatively call `async` functions.
//!   - reactions: [create_effect]
//! - **Templating/Views**: the [view] macro
//! - **Routing**: the [leptos_router](https://docs.rs/leptos_router/latest/leptos_router/) crate
//! - **Server Functions**: the [server](crate::leptos_server) macro, [create_action], and [create_server_action]
//!
//! # Feature Flags
//! - `csr` (*Default*) Client-side rendering: Generate DOM nodes in the browser
//! - `ssr` Server-side rendering: Generate an HTML string (typically on the server)
//! - `hydrate` Hydration: use this to add interactivity to an SSRed Leptos app
//! - `stable` By default, Leptos requires `nightly` Rust, which is what allows the ergonomics
//!   of calling signals as functions. If you need to use `stable`, you will need to call `.get()`
//!   and `.set()` manually.
//! - `serde` (*Default*) In SSR/hydrate mode, uses [serde](https://docs.rs/serde/latest/serde/) to serialize resources and send them
//!   from the server to the client.
//! - `serde-lite` In SSR/hydrate mode, uses [serde-lite](https://docs.rs/serde-lite/latest/serde_lite/) to serialize resources and send them
//!   from the server to the client.
//! - `miniserde` In SSR/hydrate mode, uses [miniserde](https://docs.rs/miniserde/latest/miniserde/) to serialize resources and send them
//!   from the server to the client.
//!
//! **Important Note:** You must enable one of `csr`, `hydrate`, or `ssr` to tell Leptos
//! which mode your app is operating in.
//!
//! # A Simple Counter
//!
//! ```rust
//! use leptos::*;
//!
//! #[component]
//! pub fn SimpleCounter(cx: Scope, initial_value: i32) -> impl IntoView {
//!     // create a reactive signal with the initial value
//!     let (value, set_value) = create_signal(cx, initial_value);
//!
//!     // create event handlers for our buttons
//!     // note that `value` and `set_value` are `Copy`, so it's super easy to move them into closures
//!     let clear = move |_| set_value.set(0);
//!     let decrement = move |_| set_value.update(|value| *value -= 1);
//!     let increment = move |_| set_value.update(|value| *value += 1);
//!
//!     // this JSX is compiled to an HTML template string for performance
//!     view! {
//!         cx,
//!         <div>
//!             <button on:click=clear>"Clear"</button>
//!             <button on:click=decrement>"-1"</button>
//!             <span>"Value: " {move || value().to_string()} "!"</span>
//!             <button on:click=increment>"+1"</button>
//!         </div>
//!     }
//! }
//! ```
//!
//! Leptos is easy to use with [Trunk](https://trunkrs.dev/) (or with a simple wasm-bindgen setup):
//! ```
//! # use leptos::*;
//! # if false { // can't run in doctests
//!
//! #[component]
//! fn SimpleCounter(cx: Scope, initial_value: i32) -> impl IntoView {
//!     todo!()
//! }
//!
//! pub fn main() {
//!     mount_to_body(|cx| view! { cx,  <SimpleCounter initial_value=3 /> })
//! }
//! # }
//! ```

mod additional_attributes;
pub use additional_attributes::*;
pub use leptos_config::{self, get_configuration, LeptosOptions};
#[cfg(not(all(
    target_arch = "wasm32",
    any(feature = "csr", feature = "hydrate")
)))]
/// Utilities for server-side rendering HTML.
pub mod ssr {
    pub use leptos_dom::{ssr::*, ssr_in_order::*};
}
#[allow(deprecated)]
pub use leptos_dom::{
    self, create_node_ref, debug_warn, document, error, ev,
    helpers::{
        event_target, event_target_checked, event_target_value,
        request_animation_frame, request_animation_frame_with_handle,
        request_idle_callback, request_idle_callback_with_handle, set_interval,
        set_interval_with_handle, set_timeout, set_timeout_with_handle,
        window_event_listener, window_event_listener_untyped,
        window_event_listener_with_precast,
    },
    html, log, math, mount_to, mount_to_body, svg, warn, window, Attribute,
    Class, CollectView, Errors, Fragment, HtmlElement, IntoAttribute,
    IntoClass, IntoProperty, IntoStyle, IntoView, NodeRef, Property, View,
};
pub use leptos_macro::*;
pub use leptos_reactive::*;
pub use leptos_server::{
    self, create_action, create_multi_action, create_server_action,
    create_server_multi_action, Action, MultiAction, ServerFn, ServerFnError,
};
pub use server_fn::{self, ServerFn as _};
pub use typed_builder;
mod error_boundary;
pub use error_boundary::*;
mod for_loop;
mod show;
pub use for_loop::*;
pub use show::*;
mod suspense;
pub use suspense::*;
mod text_prop;
mod transition;
pub use text_prop::TextProp;
#[cfg(any(debug_assertions, feature = "ssr"))]
#[doc(hidden)]
pub use tracing;
pub use transition::*;
extern crate self as leptos;

/// The most common type for the `children` property on components,
/// which can only be called once.
pub type Children = Box<dyn FnOnce(Scope) -> Fragment>;

/// A type for the `children` property on components that can be called
/// more than once.
pub type ChildrenFn = Box<dyn Fn(Scope) -> Fragment>;

/// A type for the `children` property on components that can be called
/// more than once, but may mutate the children.
pub type ChildrenFnMut = Box<dyn FnMut(Scope) -> Fragment>;

/// A type for taking anything that implements [`IntoAttribute`].
///
/// ```rust
/// use leptos::*;
///
/// #[component]
/// pub fn MyHeading(
///     cx: Scope,
///     text: String,
///     #[prop(optional, into)] class: Option<AttributeValue>,
/// ) -> impl IntoView {
///     view! {
///       cx,
///       <h1 class=class>{text}</h1>
///     }
/// }
/// ```
pub type AttributeValue = Box<dyn IntoAttribute>;

#[doc(hidden)]
pub trait Component<P> {}

#[doc(hidden)]
pub trait Props {
    type Builder;
    fn builder() -> Self::Builder;
}

impl<P, F, R> Component<P> for F where F: FnOnce(::leptos::Scope, P) -> R {}

#[doc(hidden)]
pub fn component_props_builder<P: Props>(
    _f: &impl Component<P>,
) -> <P as Props>::Builder {
    <P as Props>::builder()
}
