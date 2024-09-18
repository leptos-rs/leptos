#!rdeny(missing_docs)]
#![forbid(unsafe_code)]
//! # About Leptos
//!
//! Leptos is a full-stack framework for building web applications in Rust. You can use it to build
//! - single-page apps (SPAs) rendered entirely in the browser, using client-side routing and loading
//!   or mutating data via async requests to the server.
//! - multi-page apps (MPAs) rendered on the server, managing navigation, data, and mutations via
//!   web-standard `<a>` and `<form>` tags.
//! - progressively-enhanced single-page apps that are rendered on the server and then hydrated on the client,
//!   enhancing your `<a>` and `<form>` navigations and mutations seamlessly when WASM is available.
//!
//! And you can do all three of these **using the same Leptos code**.
//!
//! Take a look at the [Leptos Book](https://leptos-rs.github.io/leptos/) for a walkthrough of the framework.
//! Join us on our [Discord Channel](https://discord.gg/v38Eef6sWG) to see what the community is building.
//! Explore our [Examples](https://github.com/leptos-rs/leptos/tree/main/examples) to see Leptos in action.
//!
//! # Learning by Example
//!
//! If you want to see what Leptos is capable of, check out
//! the [examples](https://github.com/leptos-rs/leptos/tree/main/examples):
//!
//! - **[`counter`]** is the classic counter example, showing the basics of client-side rendering and reactive DOM updates.
//! - **[`counter_without_macros`]** adapts the counter example to use the builder pattern for the UI and avoids other macros,
//!   instead showing the code that Leptos generates.
//! - **[`counters`]** introduces parent-child communication via contexts, and the [`<For/>`](leptos::prelude::For) component
//!   for efficient keyed list updates.
//! - **[`error_boundary`]** shows how to use [`Result`] types to handle errors.
//! - **[`parent_child`]** shows four different ways a parent component can communicate with a child, including passing a closure,
//!   context, and more.
//! - **[`fetch`]** introduces [`Resource`](leptos::prelude::Resource)s, which allow you to integrate arbitrary `async` code like an
//!   HTTP request within your reactive code.
//! - **[`router`]** shows how to use Leptos’s nested router to enable client-side navigation and route-specific, reactive data loading.
//! - **[`slots`]** shows how to use slots on components.
//! - **[`spread`]** shows how the spread syntax can be used to spread data and/or event handlers onto elements.
//! - **[`counter_isomorphic`]** shows different methods of interaction with a stateful server, including server functions,
//!   server actions, forms, and server-sent events (SSE).
//! - **[`todomvc`]** shows the basics of building an isomorphic web app. Both the server and the client import the same app code.
//!   The server renders the app directly to an HTML string, and the client hydrates that HTML to make it interactive.
//!   You might also want to see how we use [`Effect::new`](leptos::prelude::Effect::new) to
//!   [serialize JSON to `localStorage`](https://github.com/leptos-rs/leptos/blob/20af4928b2fffe017408d3f4e7330db22cf68277/examples/todomvc/src/lib.rs#L191-L209)
//!   and [reactively call DOM methods](https://github.com/leptos-rs/leptos/blob/16f084a71268ac325fbc4a5e50c260df185eadb6/examples/todomvc/src/lib.rs#L292-L296)
//!   on [references to elements](https://github.com/leptos-rs/leptos/blob/20af4928b2fffe017408d3f4e7330db22cf68277/examples/todomvc/src/lib.rs#L228).
//! - **[`hackernews`]** and **[`hackernews_axum`]** integrate calls to a real external REST API, routing, server-side rendering and
//!   hydration to create a fully-functional application that works as intended even before WASM has loaded and begun to run.
//! - **[`todo_app_sqlite`]** and **[`todo_app_sqlite_axum`]** show how to build a full-stack app using server functions and
//!   database connections.
//! - **[`tailwind`]** shows how to integrate TailwindCSS with `trunk` for CSR.
//!
//! [`counter`]: https://github.com/leptos-rs/leptos/tree/main/examples/counter
//! [`counter_without_macros`]: https://github.com/leptos-rs/leptos/tree/main/examples/counter_without_macros
//! [`counters`]: https://github.com/leptos-rs/leptos/tree/main/examples/counters
//! [`error_boundary`]: https://github.com/leptos-rs/leptos/tree/main/examples/error_boundary
//! [`parent_child`]: https://github.com/leptos-rs/leptos/tree/main/examples/parent_child
//! [`fetch`]: https://github.com/leptos-rs/leptos/tree/main/examples/fetch
//! [`router`]: https://github.com/leptos-rs/leptos/tree/main/examples/router
//! [`slots`]: https://github.com/leptos-rs/leptos/tree/main/examples/slots
//! [`spread`]: https://github.com/leptos-rs/leptos/tree/main/examples/spread
//! [`counter_isomorphic`]: https://github.com/leptos-rs/leptos/tree/main/examples/counter_isomorphic
//! [`todomvc`]: https://github.com/leptos-rs/leptos/tree/main/examples/todomvc
//! [`hackernews`]: https://github.com/leptos-rs/leptos/tree/main/examples/hackernews
//! [`hackernews_axum`]: https://github.com/leptos-rs/leptos/tree/main/examples/hackernews_axum
//! [`todo_app_sqlite`]: https://github.com/leptos-rs/leptos/tree/main/examples/todo_app_sqlite
//! [`todo_app_sqlite_axum`]: https://github.com/leptos-rs/leptos/tree/main/examples/todo_app_sqlite_axum
//! [`tailwind`]: https://github.com/leptos-rs/leptos/tree/main/examples/tailwind_csr
//!
//! Details on how to run each example can be found in its README.
//!
//! # Quick Links
//!
//! Here are links to the most important sections of the docs:
//! - **Reactivity**: the [`reactive_graph`] overview, and more details in
//!   + signals: [`signal`](leptos::prelude::signal), [`ReadSignal`](leptos::prelude::ReadSignal),
//!     [`WriteSignal`](leptos::prelude::WriteSignal) and [`RwSignal`](leptos::prelude::RwSignal).
//!   + computations: [`Memo`](leptos::prelude::Memo).
//!   + `async` interop: [`Resource`](leptos::prelude::Resource) for loading data using `async` functions
//!     and [`Action`](leptos::prelude::Action) to mutate data or imperatively call `async` functions.
//!   + reactions: [`Effect`](leptos::prelude::Effect) and [`RenderEffect`](leptos::prelude::RenderEffect).
//! - **Templating/Views**: the [`view`] macro and [`IntoView`](leptos::IntoView) trait.
//! - **Routing**: the [`leptos_router`](https://docs.rs/leptos_router/latest/leptos_router/) crate
//! - **Server Functions**: the [`server`](macro@leptos::prelude::server) macro and [`ServerAction`](leptos::prelude::ServerAction).
//!
//! # Feature Flags
//!
//! - **`nightly`**: On `nightly` Rust, enables the function-call syntax for signal getters and setters.
//! - **`csr`** Client-side rendering: Generate DOM nodes in the browser.
//! - **`ssr`** Server-side rendering: Generate an HTML string (typically on the server).
//! - **`hydrate`** Hydration: use this to add interactivity to an SSRed Leptos app.
//! - **`rkyv`** In SSR/hydrate mode, uses [`rkyv`](https://docs.rs/rkyv/latest/rkyv/) to serialize resources and send them
//!   from the server to the client.
//! - **`tracing`** Adds support for [`tracing`](https://docs.rs/tracing/latest/tracing/).
//!
//! **Important Note:** You must enable one of `csr`, `hydrate`, or `ssr` to tell Leptos
//! which mode your app is operating in. You should only enable one of these per build target,
//! i.e., you should not have both `hydrate` and `ssr` enabled for your server binary, only `ssr`.
//!
//! # A Simple Counter
//!
//! ```rust
//! use leptos::prelude::*;
//!
//! #[component]
//! pub fn SimpleCounter(initial_value: i32) -> impl IntoView {
//!     // create a reactive signal with the initial value
//!     let (value, set_value) = signal(initial_value);
//!
//!     // create event handlers for our buttons
//!     // note that `value` and `set_value` are `Copy`, so it's super easy to move them into closures
//!     let clear = move |_| set_value.set(0);
//!     let decrement = move |_| *set_value.write() -= 1;
//!     let increment = move |_| *set_value.write() += 1;
//!
//!     view! {
//!         <div>
//!             <button on:click=clear>"Clear"</button>
//!             <button on:click=decrement>"-1"</button>
//!             <span>"Value: " {value} "!"</span>
//!             <button on:click=increment>"+1"</button>
//!         </div>
//!     }
//! }
//! ```
//!
//! Leptos is easy to use with [Trunk](https://trunkrs.dev/) (or with a simple wasm-bindgen setup):
//!
//! ```rust
//! use leptos::{mount::mount_to_body, prelude::*};
//!
//! #[component]
//! fn SimpleCounter(initial_value: i32) -> impl IntoView {
//!     // ...
//!     # _ = initial_value;
//! }
//!
//! pub fn main() {
//! # if false { // can't run in doctest
//!     mount_to_body(|| view! { <SimpleCounter initial_value=3 /> })
//! # }
//! }
//! ```

#![cfg_attr(feature = "nightly", feature(fn_traits))]
#![cfg_attr(feature = "nightly", feature(unboxed_closures))]

extern crate self as leptos;

/// Exports all the core types of the library.
pub mod prelude {
    // Traits
    // These should always be exported from the prelude
    pub use reactive_graph::prelude::*;
    pub use tachys::prelude::*;

    // Structs
    // In the future, maybe we should remove this blanket export
    // However, it is definitely useful relative to looking up every struct etc.
    mod export_types {
        #[cfg(feature = "nonce")]
        pub use crate::nonce::*;
        pub use crate::{
            callback::*, children::*, component::*, control_flow::*, error::*,
            form::*, hydration::*, into_view::*, mount::*, suspense::*,
        };
        pub use leptos_config::*;
        pub use leptos_dom::{helpers::*, *};
        pub use leptos_macro::*;
        pub use leptos_server::*;
        pub use oco_ref::*;
        pub use reactive_graph::{
            actions::*, computed::*, effect::*, owner::*, signal::*, untrack,
            wrappers::read::*,
        };
        pub use server_fn::{self, ServerFnError};
        pub use tachys::{
            self,
            reactive_graph::{node_ref::*, Suspend},
            view::template::ViewTemplate,
        };
    }
    pub use export_types::*;
}

/// Components used for working with HTML forms, like `<ActionForm>`.
pub mod form;

/// A standard way to wrap functions and closures to pass them to components.
pub mod callback;

/// Types that can be passed as the `children` prop of a component.
pub mod children;

#[doc(hidden)]
/// Traits used to implement component constructors.
pub mod component;
mod error_boundary;

/// Tools for handling errors.
pub mod error {
    pub use crate::error_boundary::*;
    pub use throw_error::*;
}

/// Control-flow components like `<Show>` and `<For>`.
pub mod control_flow {
    pub use crate::{for_loop::*, show::*};
}
mod for_loop;
mod show;

/// A component that allows rendering a component somewhere else.
pub mod portal;

/// Components to enable server-side rendering and client-side hydration.
pub mod hydration;

/// Utilities for exporting nonces to be used for a Content Security Policy.
#[cfg(feature = "nonce")]
pub mod nonce;

/// Components to load asynchronous data.
pub mod suspense {
    pub use crate::{suspense_component::*, transition::*};
}

#[macro_use]
mod suspense_component;

/// Types for reactive string properties for components.
pub mod text_prop;
mod transition;
pub use leptos_macro::*;
pub use server_fn;
#[doc(hidden)]
pub use typed_builder;
#[doc(hidden)]
pub use typed_builder_macro;
mod into_view;
pub use into_view::IntoView;
pub use leptos_dom;
mod provider;
pub use tachys;
/// Tools to mount an application to the DOM, or to hydrate it from server-rendered HTML.
pub mod mount;
pub use leptos_config as config;
pub use oco_ref as oco;
mod from_form_data;
pub use either_of as either;
pub use reactive_graph;

/// Provide and access data along the reactive graph, sharing data without directly passing arguments.
pub mod context {
    pub use crate::provider::*;
    pub use reactive_graph::owner::{provide_context, use_context};
}

pub use leptos_server as server;
/// HTML attribute types.
pub use tachys::html::attribute as attr;
/// HTML element types.
pub use tachys::html::element as html;
/// HTML event types.
#[doc(no_inline)]
pub use tachys::html::event as ev;
/// MathML element types.
pub use tachys::mathml as math;
/// SVG element types.
pub use tachys::svg;

/// Utilities for simple isomorphic logging to the console or terminal.
pub mod logging {
    pub use leptos_dom::{debug_warn, error, log, warn};
}

pub mod spawn {
    pub use any_spawner::Executor;
    use std::future::Future;

    /// Spawns a thread-safe [`Future`].
    #[track_caller]
    #[inline(always)]
    pub fn spawn(fut: impl Future<Output = ()> + Send + 'static) {
        Executor::spawn(fut)
    }

    /// Spawns a [`Future`] that cannot be sent across threads.
    #[track_caller]
    #[inline(always)]
    pub fn spawn_local(fut: impl Future<Output = ()> + 'static) {
        Executor::spawn_local(fut)
    }

    pub async fn tick() {
        Executor::tick().await
    }
}

// these reexports are used in islands
#[cfg(feature = "experimental-islands")]
#[doc(hidden)]
pub use serde;
#[cfg(feature = "experimental-islands")]
#[doc(hidden)]
pub use serde_json;
#[cfg(feature = "tracing")]
#[doc(hidden)]
pub use tracing;
#[doc(hidden)]
pub use wasm_bindgen;
#[doc(hidden)]
pub use web_sys;

/*mod additional_attributes;
pub use additional_attributes::*;
mod await_;
pub use await_::*;
pub use leptos_config::{self, get_configuration, LeptosOptions};
#[cfg(not(all(
    target_arch = "wasm32",
    any(feature = "csr", feature = "hydrate")
)))]
/// Utilities for server-side rendering HTML.
pub mod ssr {
    pub use leptos_dom::{ssr::*, ssr_in_order::*};
}
pub use leptos_dom::{
    self, create_node_ref, document, ev,
    helpers::{
        event_target, event_target_checked, event_target_value,
        request_animation_frame, request_animation_frame_with_handle,
        request_idle_callback, request_idle_callback_with_handle, set_interval,
        set_interval_with_handle, set_timeout, set_timeout_with_handle,
        window_event_listener, window_event_listener_untyped,
    },
    html,
    html::Binding,
    math, mount_to, mount_to_body, nonce, svg, window, Attribute, Class,
    CollectView, Errors, EventHandlerFn, Fragment, HtmlElement, IntoAttribute,
    IntoClass, IntoProperty, IntoStyle, IntoView, NodeRef, Property, View,
};

/// Types to make it easier to handle errors in your application.
pub mod error {
    pub use server_fn::error::{Error, Result};
}
#[cfg(all(target_arch = "wasm32", feature = "template_macro"))]
pub use leptos_macro::template;
#[cfg(not(all(target_arch = "wasm32", feature = "template_macro")))]
pub use leptos_macro::view as template;
pub use leptos_macro::{component, island, slice, slot, view, Params};
cfg_if::cfg_if!(
    if #[cfg(feature="spin")] {
        pub use leptos_spin_macro::server;
    } else {
        pub use leptos_macro::server;
    }
);
pub use leptos_reactive::*;
pub use leptos_server::{
    self, create_action, create_multi_action, create_server_action,
    create_server_multi_action, Action, MultiAction, ServerFnError,
    ServerFnErrorErr,
};
pub use server_fn::{self, ServerFn as _};
mod error_boundary;
pub use error_boundary::*;
mod animated_show;
mod for_loop;
mod provider;
mod show;
pub use animated_show::*;
pub use for_loop::*;
pub use provider::*;
#[cfg(feature = "experimental-islands")]
pub use serde;
#[cfg(feature = "experimental-islands")]
pub use serde_json;
pub use show::*;
//pub use suspense_component::*;
mod suspense_component;
//mod transition;
#[cfg(feature = "tracing")]
#[doc(hidden)]
pub use tracing;
pub use transition::*;
#[doc(hidden)]
pub use typed_builder;
#[doc(hidden)]
pub use typed_builder::Optional;
#[doc(hidden)]
pub use typed_builder_macro;
#[doc(hidden)]
#[cfg(any(
    feature = "csr",
    feature = "hydrate",
    feature = "template_macro"
))]
pub use wasm_bindgen; // used in islands
#[doc(hidden)]
#[cfg(any(
    feature = "csr",
    feature = "hydrate",
    feature = "template_macro"
))]
pub use web_sys; // used in islands

mod children;
mod portal;
mod view_fn;
pub use children::*;
pub use portal::*;
pub use view_fn::*;

extern crate self as leptos;

/// A type for taking anything that implements [`IntoAttribute`].
///
/// ```rust
/// use leptos::*;
///
/// #[component]
/// pub fn MyHeading(
///     text: String,
///     #[prop(optional, into)] class: Option<AttributeValue>,
/// ) -> impl IntoView {
///     view! {
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

#[doc(hidden)]
pub trait DynAttrs {
    fn dyn_attrs(self, _args: Vec<(&'static str, Attribute)>) -> Self
    where
        Self: Sized,
    {
        self
    }
}

impl DynAttrs for () {}

#[doc(hidden)]
pub trait DynBindings {
    fn dyn_bindings<B: Into<Binding>>(
        self,
        _args: impl IntoIterator<Item = B>,
    ) -> Self
    where
        Self: Sized,
    {
        self
    }
}

impl DynBindings for () {}

#[doc(hidden)]
pub trait PropsOrNoPropsBuilder {
    type Builder;
    fn builder_or_not() -> Self::Builder;
}

#[doc(hidden)]
#[derive(Copy, Clone, Debug, Default)]
pub struct EmptyPropsBuilder {}

impl EmptyPropsBuilder {
    pub fn build(self) {}
}

impl<P: Props> PropsOrNoPropsBuilder for P {
    type Builder = <P as Props>::Builder;
    fn builder_or_not() -> Self::Builder {
        Self::builder()
    }
}

impl PropsOrNoPropsBuilder for EmptyPropsBuilder {
    type Builder = EmptyPropsBuilder;
    fn builder_or_not() -> Self::Builder {
        EmptyPropsBuilder {}
    }
}

impl<F, R> Component<EmptyPropsBuilder> for F where F: FnOnce() -> R {}

impl<P, F, R> Component<P> for F
where
    F: FnOnce(P) -> R,
    P: Props,
{
}

#[doc(hidden)]
pub fn component_props_builder<P: PropsOrNoPropsBuilder>(
    _f: &impl Component<P>,
) -> <P as PropsOrNoPropsBuilder>::Builder {
    <P as PropsOrNoPropsBuilder>::builder_or_not()
}

#[doc(hidden)]
pub fn component_view<P>(f: impl ComponentConstructor<P>, props: P) -> View {
    f.construct(props)
}

#[doc(hidden)]
pub trait ComponentConstructor<P> {
    fn construct(self, props: P) -> View;
}

impl<Func, V> ComponentConstructor<()> for Func
where
    Func: FnOnce() -> V,
    V: IntoView,
{
    fn construct(self, (): ()) -> View {
        (self)().into_view()
    }
}

impl<Func, V, P> ComponentConstructor<P> for Func
where
    Func: FnOnce(P) -> V,
    V: IntoView,
    P: PropsOrNoPropsBuilder,
{
    fn construct(self, props: P) -> View {
        (self)(props).into_view()
    }
}*/
