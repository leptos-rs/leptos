use crate::{document, Element};
use cfg_if::cfg_if;
use leptos_reactive::Scope;
use wasm_bindgen::UnwrapThrowExt;

pub trait Mountable {
    fn mount(&self, parent: &web_sys::Element);
}

impl Mountable for Element {
    fn mount(&self, parent: &web_sys::Element) {
        cfg_if! {
            if #[cfg(any(feature = "csr", feature = "hydrate"))] {
                parent.append_child(self).unwrap_throw();
            } else {
                let _ = parent;
            }
        }
    }
}

impl Mountable for Vec<Element> {
    fn mount(&self, parent: &web_sys::Element) {
        cfg_if! {
            if #[cfg(any(feature = "csr", feature = "hydrate"))] {
                for element in self {
                    parent.append_child(element).unwrap_throw();
                }
            }
        }
    }
}

pub fn mount_to_body<T, F>(f: F)
where
    F: Fn(Scope) -> T + 'static,
    T: Mountable,
{
    mount(document().body().unwrap_throw(), f)
}

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
