pub mod attribute;
pub mod child;
pub mod class;
pub mod event_delegation;
pub mod logging;
#[cfg(not(feature = "ssr"))]
pub mod mount;
pub mod operations;
pub mod property;
#[cfg(not(feature = "ssr"))]
pub mod reconcile;
#[cfg(not(feature = "ssr"))]
pub mod render;
#[cfg(feature = "ssr")]
pub mod render_to_string;

pub use attribute::*;
pub use child::*;
pub use class::*;
pub use logging::*;
#[cfg(not(feature = "ssr"))]
pub use mount::*;
pub use operations::*;
pub use property::*;
#[cfg(not(feature = "ssr"))]
pub use render::*;
#[cfg(feature = "ssr")]
pub use render_to_string::*;

pub use js_sys;
pub use wasm_bindgen;
pub use web_sys;

#[cfg(not(feature = "ssr"))]
pub type Element = web_sys::Element;
#[cfg(feature = "ssr")]
pub type Element = String;

#[cfg(not(feature = "ssr"))]
pub type Node = web_sys::Node;
#[cfg(feature = "ssr")]
pub type Node = String;

use leptos_reactive::Scope;
pub use wasm_bindgen::UnwrapThrowExt;

#[cfg(feature = "csr")]
pub fn create_component<F, T>(cx: Scope, f: F) -> T
where
    F: FnOnce() -> T,
{
    cx.untrack(f)
}

#[cfg(not(feature = "csr"))]
pub fn create_component<F, T>(cx: Scope, f: F) -> T
where
    F: FnOnce() -> T,
{
    cx.with_next_context(f)
}

#[macro_export]
macro_rules! is_server {
    () => {
        cfg!(feature = "ssr")
    };
}

#[macro_export]
macro_rules! is_dev {
    () => {
        cfg!(debug_assertions)
    };
}
