#![feature(stmt_expr_attributes)]
//! # About Leptos
//!
//! Leptos is a full-stack framework for building web applications in Rust. You can use it to build
//! - single-page apps (SPAs) rendered entirely in the browser, using client-side routing and loading
//!   or mutating data via async requests to the server
//! - multi-page apps (MPAs) rendered on the server, managing navigation, data, and mutations via
//!   web-standard `<a>` and `<form>` tags
//! - progressively-enhanced multi-page apps ([PEMPAs](https://www.epicweb.dev/the-webs-next-transition)?)
//!   that are rendered on the server and then hydrated on the client, enhancing your `<a>` and `<form>`
//!   navigations and mutations seamlessly when WASM is available.
//!
//! And you can do all three of these **using the same Leptos code.**
//!
//! # Learning by Example
//!  
//! These docs are a work in progress. If you want to see what Leptos is capable of, check out
//! the [examples](https://github.com/gbj/leptos/tree/main/examples):
//! - [`counter`](https://github.com/gbj/leptos/tree/main/examples/counter) is the classic
//!   counter example, showing the basics of client-side rendering and reactive DOM updates
//! - [`counters`](https://github.com/gbj/leptos/tree/main/examples/counter) introduces parent-child
//!   communication via contexts, and the `<For/>` component for efficient keyed list updates.
//! - [`todomvc`](https://github.com/gbj/leptos/tree/main/examples/todomvc) implements the classic to-do
//!   app in Leptos. This is a good example of a complete, simple app. In particular, you might want to
//!   see how we use [create_effect] to [serialize JSON to `localStorage`](https://github.com/gbj/leptos/blob/16f084a71268ac325fbc4a5e50c260df185eadb6/examples/todomvc/src/lib.rs#L164)
//!   and [reactively call DOM methods](https://github.com/gbj/leptos/blob/6d7c36655c9e7dcc3a3ad33d2b846a3f00e4ae74/examples/todomvc/src/lib.rs#L291)
//!   on [references to elements](https://github.com/gbj/leptos/blob/6d7c36655c9e7dcc3a3ad33d2b846a3f00e4ae74/examples/todomvc/src/lib.rs#L254).
//! - [`fetch`](https://github.com/gbj/leptos/tree/main/examples/fetch) introduces
//!   [Resource](leptos_reactive::Resource)s, which allow you to integrate arbitrary `async` code like an
//!   HTTP request within your reactive code.
//! - [`router`](https://github.com/gbj/leptos/tree/main/examples/router) shows how to use Leptos’s nested router
//!   to enable client-side navigation and route-specific, reactive data loading.
//! - [`todomvc`](https://github.com/gbj/leptos/tree/main/examples/todomvc) shows the basics of building an
//!   isomorphic web app. Both the server and the client import the same app code from the `todomvc` example.
//!   The server renders the app directly to an HTML string, and the client hydrates that HTML to make it interactive.
//! - [`hackernews`](https://github.com/gbj/leptos/tree/main/examples/hackernews) pulls everything together.
//!   It integrates calls to a real external REST API, routing, server-side rendering and hydration to create
//!   a fully-functional PEMPA that works as intended even before WASM has loaded and begun to run.
//!
//! (The SPA examples can be run using `trunk serve`. For information about Trunk,
//! [see here]((https://trunkrs.dev/)).)
//!
//! # Quick Links
//!
//! Here are links to the most important sections of the docs:
//! - **Reactivity**: the [leptos_reactive] overview, and more details in
//!   - [create_signal], [ReadSignal], and [WriteSignal]
//!   - [create_memo] and [Memo]
//!   - [create_resource] and [Resource]
//!   - [create_effect]
//! - **Templating/Views**: the [view] macro
//! - **Routing**: the [leptos_router](https://docs.rs/leptos_router/latest/leptos_router/) crate
//!
//! # A Simple Counter
//!
//! ```rust
//! use leptos::*;
//!
//! #[component]
//! pub fn SimpleCounter(cx: Scope, initial_value: i32) -> Element {
//!     // create a reactive signal with the initial value
//!     let (value, set_value) = create_signal(cx, initial_value);
//!
//!     // create event handlers for our buttons
//!     // note that `value` and `set_value` are `Copy`, so it's super easy to move them into closures
//!     let clear = move |_| set_value(0);
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
//!
//! // Easy to use with Trunk (trunkrs.dev) or with a simple wasm-bindgen setup
//! pub fn main() {
//!     mount_to_body(|cx| view! { cx,  <SimpleCounter initial_value=3 /> })
//! }
//! ```

pub use leptos_core::*;
pub use leptos_dom;
pub use leptos_dom::wasm_bindgen::{JsCast, UnwrapThrowExt};
pub use leptos_dom::*;
pub use leptos_macro::*;
pub use leptos_reactive::*;

pub use leptos_reactive::debug_warn;
