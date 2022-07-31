mod attribute;
mod child;
mod class;
mod event_delegation;
pub mod logging;
mod operations;
mod property;
mod reconcile;
mod render;

pub use attribute::*;
pub use child::*;
pub use class::*;
pub use logging::*;
pub use operations::*;
pub use property::*;
pub use render::*;

pub use js_sys;
pub use wasm_bindgen;
pub use web_sys;

pub type Element = web_sys::Element;

use leptos_reactive::{create_scope, RootContext, Scope};
pub use wasm_bindgen::UnwrapThrowExt;

pub trait Mountable {
    fn mount(&self, parent: &web_sys::Element);
}

impl Mountable for Element {
    fn mount(&self, parent: &web_sys::Element) {
        parent.append_child(&self).unwrap_throw();
    }
}

impl Mountable for Vec<Element> {
    fn mount(&self, parent: &web_sys::Element) {
        for element in self {
            parent.append_child(&element).unwrap_throw();
        }
    }
}

pub fn mount_to_body<T, F>(f: F)
where
    F: Fn(Scope) -> T,
    T: Mountable,
{
    mount(&document().body().unwrap_throw(), f)
}

pub fn mount<T, F>(parent: &web_sys::Element, f: F)
where
    F: Fn(Scope) -> T,
    T: Mountable,
{
    let stack = Box::leak(Box::new(RootContext::new()));

    // running "mount" intentionally leaks the memory,
    // as the "mount" has no parent that can clean it up
    let _ = create_scope(stack, |cx| {
        (f(cx)).mount(parent);
    });
}

pub fn create_component<'a, F, T>(cx: Scope<'a>, f: F) -> T
where
    F: Fn() -> T,
    T: IntoChild<'a>,
{
    // TODO hydration logic here
    cx.untrack(f)
}

#[macro_export]
macro_rules! is_server {
    () => {
        !cfg!(target_arch = "wasm32")
    };
}

#[macro_export]
macro_rules! is_dev {
    () => {
        cfg!(debug_assertions)
    };
}
