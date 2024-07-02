use once_cell::unsync::Lazy;
use wasm_bindgen::JsCast;
use web_sys::{Document, HtmlElement, Node, Window};

thread_local! {
    pub(crate) static WINDOW: web_sys::Window = web_sys::window().unwrap();

    pub(crate) static DOCUMENT: web_sys::Document = web_sys::window().unwrap().document().unwrap();
}

/// Returns the [`Window`](https://developer.mozilla.org/en-US/docs/Web/API/Window).
///
/// This is cached as a thread-local variable, so calling `window()` multiple times
/// requires only one call out to JavaScript.
pub fn window() -> Window {
    WINDOW.with(Clone::clone)
}

/// Returns the [`Document`](https://developer.mozilla.org/en-US/docs/Web/API/Document).
///
/// This is cached as a thread-local variable, so calling `document()` multiple times
/// requires only one call out to JavaScript.
pub fn document() -> Document {
    DOCUMENT.with(Clone::clone)
}

pub fn body() -> HtmlElement {
    document().body().unwrap()
}

pub fn comment() -> Node {
    thread_local! {
        static COMMENT: Lazy<Node> = Lazy::new(|| {
            document().create_comment("").unchecked_into()
        });
    }
    COMMENT.with(|n| n.clone_node().unwrap())
}

pub fn log(s: &str) {
    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(s));
}

/// Helper function to extract [`Event.target`](https://developer.mozilla.org/en-US/docs/Web/API/Event/target)
/// from any event.
pub fn event_target<T>(event: &web_sys::Event) -> T
where
    T: JsCast,
{
    event.target().unwrap().unchecked_into::<T>()
}

/// Helper function to extract `event.target.value` from an event.
///
/// This is useful in the `on:input` or `on:change` listeners for an `<input>` element.
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
pub fn event_target_checked(ev: &web_sys::Event) -> bool {
    ev.target()
        .unwrap()
        .unchecked_into::<web_sys::HtmlInputElement>()
        .checked()
}
