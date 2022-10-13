#![feature(stmt_expr_attributes)]

//! **Please note:** This framework is in active development. I'm keeping it in a cycle of 0.0.x releases at the moment to indicate that it’s not even ready for its 0.1.0. Active work is being done on documentation and features, and APIs should not necessarily be considered stable. At the same time, it is more than a toy project or proof of concept, and I am actively using it for my own application development.
//!
//! ## Fine-Grained Reactivity
//!
//! Leptos is built on a fine-grained reactive system, which means that individual reactive values
//! (“signals,” sometimes known as observables) trigger the code that reacts to them (“effects,”
//! sometimes known as observers) to re-run. These two halves of the reactive system are inter-dependent.
//! Without effects, signals can change within the reactive system but never be observed in a way
//! that interacts with the outside world. Without signals, effects run once but never again, as
//! there’s no observable value to subscribe to.
//!
//! Here are the most commonly-used functions and types you'll need to build a reactive system:
//!
//! ### Signals
//! 1. *Signals:* [create_signal](crate::create_signal), which returns a ([ReadSignal](crate::ReadSignal),
//!    [WriteSignal](crate::WriteSignal)) tuple.
//! 2. *Derived Signals:* any function that relies on another signal.
//! 3. *Memos:* [create_memo](crate::create_memo), which returns a [Memo](crate::Memo).
//! 4. *Resources:* [create_resource], which converts an `async` [Future](std::future::Future)
//!    into a synchronous [Resource](crate::Resource) signal.
//!
//! ### Effects
//! 1. Use [create_effect](crate::create_effect) when you need to synchronize the reactive system
//!    with something outside it (for example: logging to the console, writing to a file or local storage)
//! 2. The Leptos DOM renderer wraps any [Fn] in your template with [create_effect](crate::create_effect), so
//!    components you write do *not* need explicit effects to synchronize with the DOM.
//!
//!
//! ## Views and Components
//!
//! More docs are needed for the UI system, and an in-depth guide is in progress. Here’s a simple example
//! of how the reactive system feeds into UI.
//!
//! ```rust
//! use leptos::*;
//!
//! #[component]
//! pub fn SimpleCounter(cx: Scope, initial_value: i32) -> Element {
//!     // create a reactive signal with the initial value
//!     let (value, set_value) = create_signal(cx, inital_value);
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
//!     mount_to_body(|cx| view! { cx,  <SimpleCounter initial_value=3> })
//! }
//! ```

pub use leptos_core::*;
pub use leptos_dom;
pub use leptos_dom::wasm_bindgen::{JsCast, UnwrapThrowExt};
pub use leptos_dom::*;
pub use leptos_macro::*;
pub use leptos_reactive::*;

pub use leptos_reactive::debug_warn;
