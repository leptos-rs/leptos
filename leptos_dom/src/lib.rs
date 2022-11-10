use cfg_if::cfg_if;

pub mod attribute;
pub mod child;
pub mod class;
pub mod event_delegation;
pub mod logging;
pub mod mount;
pub mod operations;
pub mod property;

cfg_if! {
    // can only include this if we're *only* enabling SSR, as it's the lowest-priority feature
    // if either `csr` or `hydrate` is enabled, `Element` is a `web_sys::Element` and can't be rendered
    if #[cfg(doc)] {
        /// The type of an HTML or DOM element. When server rendering, this is a `String`. When rendering in a browser,
        /// this is a DOM `Element`.
        pub type Element = web_sys::Element;

        /// The type of an HTML or DOM node. When server rendering, this is a `String`. When rendering in a browser,
        /// this is a DOM `Node`.
        pub type Node = web_sys::Node;

        pub mod render_to_string;
        pub use render_to_string::*;
        pub mod reconcile;
        pub mod render;

        pub use reconcile::*;
        pub use render::*;
    } else if #[cfg(not(any(feature = "hydrate", feature = "csr")))] {
        /// The type of an HTML or DOM element. When server rendering, this is a `String`. When rendering in a browser,
        /// this is a DOM `Element`.
        pub type Element = String;

        /// The type of an HTML or DOM node. When server rendering, this is a `String`. When rendering in a browser,
        /// this is a DOM `Node`.
        pub type Node = String;

        pub mod render_to_string;
        pub use render_to_string::*;

        pub struct Marker { }
    } else {
        /// The type of an HTML or DOM element. When server rendering, this is a `String`. When rendering in a browser,
        /// this is a DOM `Element`.
        pub type Element = web_sys::Element;

        /// The type of an HTML or DOM node. When server rendering, this is a `String`. When rendering in a browser,
        /// this is a DOM `Node`.
        pub type Node = web_sys::Node;

        pub mod reconcile;
        pub mod render;

        pub use reconcile::*;
        pub use render::*;
    }
}

pub use attribute::*;
pub use child::*;
pub use class::*;
pub use logging::*;
pub use mount::*;
pub use operations::*;
pub use property::*;

pub use js_sys;
pub use wasm_bindgen;
pub use web_sys;

use leptos_reactive::Scope;
pub use wasm_bindgen::UnwrapThrowExt;

// Hidden because this is primarily used by the `view` macro, not by library users.
#[doc(hidden)]
pub fn create_component<F, T>(cx: Scope, f: F) -> T
where
    F: FnOnce() -> T,
{
    cfg_if! {
        if #[cfg(feature = "csr")] {
            cx.untrack(f)
        } else {
            cx.with_next_context(f)
        }
    }
}

/// Shorthand to test for whether an `ssr` feature is enabled.
///
/// In the past, this was implemented by checking whether `not(target_arch = "wasm32")`.
/// Now that some cloud platforms are moving to run Wasm on the edge, we really can't
/// guarantee that compiling to Wasm means browser APIs are available, or that not compiling
/// to Wasm means we're running on the server.
///
/// ```
/// # use leptos_dom::is_server;
/// let todos = if is_server!() {
///   // if on the server, load from DB
/// } else {
///   // if on the browser, do something else
/// }
/// ```
#[macro_export]
macro_rules! is_server {
    () => {
        cfg!(feature = "ssr")
    };
}

/// A shorthand macro to test whether this is a debug build.
/// ```
/// # use leptos_dom::is_dev;
/// if is_dev!() {
///   // log something or whatever
/// }
/// ```
#[macro_export]
macro_rules! is_dev {
    () => {
        cfg!(debug_assertions)
    };
}
