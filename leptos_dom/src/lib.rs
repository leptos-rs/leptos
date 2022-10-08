pub mod attribute;
pub mod child;
pub mod class;
pub mod event_delegation;
pub mod logging;
pub mod operations;
pub mod property;
#[cfg(any(feature = "csr", feature = "hydrate"))]
pub mod reconcile;
#[cfg(any(feature = "csr", feature = "hydrate"))]
pub mod render;

pub use attribute::*;
pub use child::*;
pub use class::*;
pub use logging::*;
pub use operations::*;
pub use property::*;
#[cfg(any(feature = "csr", feature = "hydrate"))]
pub use render::*;

pub use js_sys;
pub use wasm_bindgen;
pub use web_sys;

#[cfg(any(feature = "csr", feature = "hydrate"))]
pub type Element = web_sys::Element;
#[cfg(feature = "ssr")]
pub type Element = String;

#[cfg(any(feature = "csr", feature = "hydrate"))]
pub type Node = web_sys::Node;
#[cfg(feature = "ssr")]
pub type Node = String;

use leptos_reactive::Scope;
pub use wasm_bindgen::UnwrapThrowExt;

#[cfg(any(feature = "csr", feature = "hydrate"))]
pub trait Mountable {
    fn mount(&self, parent: &web_sys::Element);
}

#[cfg(any(feature = "csr", feature = "hydrate"))]
impl Mountable for Element {
    fn mount(&self, parent: &web_sys::Element) {
        parent.append_child(self).unwrap_throw();
    }
}

#[cfg(any(feature = "csr", feature = "hydrate"))]
impl Mountable for Vec<Element> {
    fn mount(&self, parent: &web_sys::Element) {
        for element in self {
            parent.append_child(element).unwrap_throw();
        }
    }
}

#[cfg(any(feature = "csr", feature = "hydrate"))]
pub fn mount_to_body<T, F>(f: F)
where
    F: Fn(Scope) -> T + 'static,
    T: Mountable,
{
    mount(document().body().unwrap_throw(), f)
}

#[cfg(any(feature = "csr", feature = "hydrate"))]
pub fn mount<T, F>(parent: web_sys::HtmlElement, f: F)
where
    F: Fn(Scope) -> T + 'static,
    T: Mountable,
{
    use leptos_reactive::create_scope;

    // running "mount" intentionally leaks the memory,
    // as the "mount" has no parent that can clean it up
    let _ = create_scope(move |cx| {
        (f(cx)).mount(&parent);
    });
}

#[cfg(feature = "hydrate")]
pub fn hydrate<T, F>(parent: web_sys::HtmlElement, f: F)
where
    F: Fn(Scope) -> T + 'static,
    T: Mountable,
{
    // running "hydrate" intentionally leaks the memory,
    // as the "hydrate" has no parent that can clean it up
    let _ = leptos_reactive::create_scope(move |cx| {
        cx.start_hydration(&parent);
        (f(cx));
        cx.end_hydration();
    });
}

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
