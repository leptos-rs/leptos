mod attribute;
mod child;
mod class;
#[cfg(feature = "browser")]
mod event_delegation;
pub mod logging;
#[cfg(feature = "browser")]
mod operations;
mod property;
#[cfg(feature = "browser")]
mod reconcile;
#[cfg(feature = "browser")]
mod render;

pub use attribute::*;
pub use child::*;
pub use class::*;
pub use logging::*;
#[cfg(feature = "browser")]
pub use operations::*;
pub use property::*;
#[cfg(feature = "browser")]
pub use render::*;

pub use js_sys;
pub use wasm_bindgen;
pub use web_sys;

#[cfg(feature = "browser")]
pub type Element = web_sys::Element;
#[cfg(not(feature = "browser"))]
pub type Element = String;

#[cfg(feature = "browser")]
pub type Node = web_sys::Node;
#[cfg(not(feature = "browser"))]
pub type Node = String;

use leptos_reactive::{create_scope, Scope};
pub use wasm_bindgen::UnwrapThrowExt;

#[cfg(feature = "browser")]
pub trait Mountable {
    fn mount(&self, parent: &web_sys::Element);
}

#[cfg(feature = "browser")]
impl Mountable for Element {
    fn mount(&self, parent: &web_sys::Element) {
        parent.append_child(self).unwrap_throw();
    }
}

#[cfg(feature = "browser")]
impl Mountable for Vec<Element> {
    fn mount(&self, parent: &web_sys::Element) {
        for element in self {
            parent.append_child(element).unwrap_throw();
        }
    }
}

#[cfg(feature = "browser")]
pub fn mount_to_body<T, F>(f: F)
where
    F: Fn(Scope) -> T + 'static,
    T: Mountable,
{
    mount(document().body().unwrap_throw(), f)
}

#[cfg(feature = "browser")]
pub fn mount<T, F>(parent: web_sys::HtmlElement, f: F)
where
    F: Fn(Scope) -> T + 'static,
    T: Mountable,
{
    // running "mount" intentionally leaks the memory,
    // as the "mount" has no parent that can clean it up
    let _ = create_scope(move |cx| {
        (f(cx)).mount(&parent);
    });
}

#[cfg(feature = "browser")]
pub fn hydrate<T, F>(parent: web_sys::HtmlElement, f: F)
where
    F: Fn(Scope) -> T + 'static,
    T: Mountable,
{
    // running "hydrate" intentionally leaks the memory,
    // as the "hydrate" has no parent that can clean it up
    let _ = create_scope(move |cx| {
        cx.start_hydration(&parent);
        (f(cx));
        cx.end_hydration();
    });
}

pub fn create_component<F, T>(cx: Scope, f: F) -> T
where
    F: Fn() -> T,
{
    // TODO hydration logic here
    cx.untrack(f)
}

#[macro_export]
macro_rules! is_server {
    () => {
        cfg!(feature = "server")
    };
}

#[macro_export]
macro_rules! is_dev {
    () => {
        cfg!(debug_assertions)
    };
}
