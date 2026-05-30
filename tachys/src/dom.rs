#[cfg(not(target_os = "wasi"))]
use wasm_bindgen::JsCast;
#[cfg(not(target_os = "wasi"))]
pub use web_sys::{Document, HtmlElement, Window};

/// Dummy type for Window on WASI.
#[cfg(target_os = "wasi")]
pub type Window = ();

/// Dummy type for Document on WASI.
#[cfg(target_os = "wasi")]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Document;

#[cfg(target_os = "wasi")]
impl Document {
    /// Returns the body of the document.
    pub fn body(&self) -> Option<HtmlElement> {
        None
    }
    /// Sets the title of the document.
    pub fn set_title(&self, title: &str) {
        let _ = title;
    }
}

/// Dummy type for HtmlElement on WASI.
#[cfg(target_os = "wasi")]
pub type HtmlElement = crate::renderer::types::Element;

#[cfg(not(target_os = "wasi"))]
thread_local! {
    pub(crate) static WINDOW: web_sys::Window = web_sys::window().unwrap();

    pub(crate) static DOCUMENT: web_sys::Document = web_sys::window().unwrap().document().unwrap();
}

/// Returns the [`Window`](https://developer.mozilla.org/en-US/docs/Web/API/Window).
///
/// This is cached as a thread-local variable, so calling `window()` multiple times
/// requires only one call out to JavaScript.
pub fn window() -> Window {
    #[cfg(not(target_os = "wasi"))]
    {
        WINDOW.with(Clone::clone)
    }
    #[cfg(target_os = "wasi")]
    {
        ()
    }
}

/// Returns the [`Document`](https://developer.mozilla.org/en-US/docs/Web/API/Document).
///
/// This is cached as a thread-local variable, so calling `document()` multiple times
/// requires only one call out to JavaScript.
///
/// ## Panics
/// Panics if called outside a browser environment.
pub fn document() -> Document {
    #[cfg(not(target_os = "wasi"))]
    {
        DOCUMENT.with(Clone::clone)
    }
    #[cfg(target_os = "wasi")]
    {
        Document
    }
}

/// The `<body>` element.
///
/// ## Panics
/// Panics if there is no `<body>` in the current document, or if it is called outside a browser
/// environment.
pub fn body() -> HtmlElement {
    #[cfg(not(target_os = "wasi"))]
    {
        document().body().unwrap()
    }
    #[cfg(target_os = "wasi")]
    {
        crate::renderer::types::Element
    }
}

/// Helper function to extract [`Event.target`](https://developer.mozilla.org/en-US/docs/Web/API/Event/target)
/// from any event.
#[cfg(not(target_os = "wasi"))]
pub fn event_target<T>(event: &web_sys::Event) -> T
where
    T: JsCast,
{
    event.target().unwrap().unchecked_into::<T>()
}

/// Helper function to extract `event.target.value` from an event.
///
/// This is useful in the `on:input` or `on:change` listeners for an `<input>` element.
#[cfg(not(target_os = "wasi"))]
pub fn event_target_value<T>(event: &T) -> String
where
    T: JsCast,
{
    event
        .unchecked_ref::<web_sys::Event>()
        .target()
        .unwrap()
        .unchecked_into::<web_sys::HtmlInputElement>()
        .value()
}

/// Helper function to extract `event.target.checked` from an event.
///
/// This is useful in the `on:change` listeners for an `<input type="checkbox">` element.
#[cfg(not(target_os = "wasi"))]
pub fn event_target_checked(ev: &web_sys::Event) -> bool {
    ev.target()
        .unwrap()
        .unchecked_into::<web_sys::HtmlInputElement>()
        .checked()
}
