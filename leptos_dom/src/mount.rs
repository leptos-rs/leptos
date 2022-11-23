use crate::{document, Element};
use cfg_if::cfg_if;
use leptos_reactive::Scope;
use wasm_bindgen::UnwrapThrowExt;

/// Describes a type that can be mounted to a parent element in the DOM.
pub trait Mountable {
    /// Injects the element into the parent as its next child.
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
            } else {
                _ = parent; // to clear warning
            }
        }
    }
}

/// Runs the given function to mount something to the `<body>` element in the DOM.
///
/// ```
/// // the simplest Leptos application
/// # use leptos_dom::*; use leptos_dom::wasm_bindgen::JsCast;
/// # use leptos_macro::view;
/// # if false { // can't actually run as a doctest on any feature
/// mount_to_body(|cx| view! { cx,  <p>"Hello, world!"</p> });
/// # }
/// ```
pub fn mount_to_body<T, F>(f: F)
where
    F: Fn(Scope) -> T + 'static,
    T: Mountable,
{
    mount(document().body().unwrap_throw(), f)
}

/// Runs the given function to mount something to the given element in the DOM.
///
/// ```
/// // a very simple Leptos application
/// # use leptos_dom::*; use leptos_dom::wasm_bindgen::JsCast;
/// # use leptos_macro::view;
/// # if false { // can't actually run as a doctest on any feature
/// mount(
///   document().get_element_by_id("root").unwrap().unchecked_into(),
///   |cx| view! { cx,  <p>"Hello, world!"</p> }
/// );
/// # }
/// ```
pub fn mount<T, F>(parent: web_sys::HtmlElement, f: F)
where
    F: Fn(Scope) -> T + 'static,
    T: Mountable,
{
    use leptos_reactive::{create_runtime, create_scope};

    // this is not a leak
    // CSR and hydrate mode define a single, thread-local Runtime
    let _ = create_scope(create_runtime(), move |cx| {
        (f(cx)).mount(&parent);
    });
}

/// “Hydrates” server-rendered HTML, attaching event listeners and setting up reactivity
/// while reusing the existing DOM nodes, by running the given function beginning with
/// the parent node.
///
/// ```
/// // rehydrate a very simple Leptos application
/// # use leptos_dom::*; use leptos_dom::wasm_bindgen::JsCast;
/// # use leptos_macro::view;
/// # if false { // can't actually run as a doctest on any feature
/// if let Some(body) = body() {
///   hydrate(body, |cx| view! { cx,  <p>"Hello, world!"</p> });
/// }
/// # }
/// ```
#[cfg(feature = "hydrate")]
pub fn hydrate<T, F>(parent: web_sys::HtmlElement, f: F)
where
    F: Fn(Scope) -> T + 'static,
    T: Mountable,
{
    use leptos_reactive::create_runtime;

    // this is not a leak
    // CSR and hydrate mode define a single, thread-local Runtime
    let _ = leptos_reactive::create_scope(create_runtime(), move |cx| {
        cx.start_hydration(&parent);
        (f(cx));
        cx.end_hydration();
    });
}
