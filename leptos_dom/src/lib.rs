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
    if #[cfg(not(any(feature = "hydrate", feature = "csr")))] {
        pub type Element = String;
        pub type Node = String;

        pub mod render_to_string;
        pub use render_to_string::*;
    } else {
        pub type Element = web_sys::Element;
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
