use std::time::Duration;

use wasm_bindgen::{prelude::Closure, JsCast, JsValue, UnwrapThrowExt};

use crate::{debug_warn, event_delegation, is_server};

thread_local! {
    pub(crate) static WINDOW: web_sys::Window = web_sys::window().unwrap_throw();

    pub(crate) static DOCUMENT: web_sys::Document = web_sys::window().unwrap_throw().document().unwrap_throw();
}

/// Returns the [`Window`](https://developer.mozilla.org/en-US/docs/Web/API/Window).
///
/// This is cached as a thread-local variable, so calling `window()` multiple times
/// requires only one call out to JavaScript.
pub fn window() -> web_sys::Window {
    WINDOW.with(|window| window.clone())
}

/// Returns the [`Document`](https://developer.mozilla.org/en-US/docs/Web/API/Document).
///
/// This is cached as a thread-local variable, so calling `window()` multiple times
/// requires only one call out to JavaScript.
pub fn document() -> web_sys::Document {
    DOCUMENT.with(|document| document.clone())
}

/// Returns the `<body>` elements of the current HTML document, if it exists.
pub fn body() -> Option<web_sys::HtmlElement> {
    document().body()
}

/// Creates a DOM [`Element`](https://developer.mozilla.org/en-US/docs/Web/API/Element). See
/// [`Document.createElement`](https://developer.mozilla.org/en-US/docs/Web/API/Document/createElement).
pub fn create_element(tag_name: &str) -> web_sys::Element {
    document().create_element(tag_name).unwrap_throw()
}

/// Creates a DOM [`Text`](https://developer.mozilla.org/en-US/docs/Web/API/Text) node. See
/// [`Document.createTextNode`](https://developer.mozilla.org/en-US/docs/Web/API/Document/createTextNode).
pub fn create_text_node(data: &str) -> web_sys::Text {
    document().create_text_node(data)
}

/// Creates a [`DocumentFragment`](https://developer.mozilla.org/en-US/docs/Web/API/DocumentFragment). See
/// [`Document.createElement`](https://developer.mozilla.org/en-US/docs/Web/API/Document/createDocumentFragment).
pub fn create_fragment() -> web_sys::DocumentFragment {
    document().create_document_fragment()
}

/// Creates a [`Comment`](https://developer.mozilla.org/en-US/docs/Web/API/Comment) node.
/// See [`Document.createCommentNode`](https://developer.mozilla.org/en-US/docs/Web/API/Document/createComment).
pub fn create_comment_node() -> web_sys::Node {
    document().create_comment("").unchecked_into()
}

/// Creates an [`HTMLTemplateElement`](https://developer.mozilla.org/en-US/docs/Web/API/HTMLTemplateElement)
/// and sets its `innerHTML` to the given HTML string.
pub fn create_template(html: &str) -> web_sys::HtmlTemplateElement {
    let template = create_element("template");
    template.set_inner_html(html);
    template.unchecked_into()
}

/// Clones an an [`HTMLTemplateElement`](https://developer.mozilla.org/en-US/docs/Web/API/HTMLTemplateElement)
/// and returns its first element child.
pub fn clone_template(template: &web_sys::HtmlTemplateElement) -> web_sys::Element {
    template
        .content()
        .first_element_child()
        .unwrap_throw()
        .clone_node_with_deep(true)
        .unwrap_throw()
        .unchecked_into()
}

/// Appends a child node to the parent element.
/// See [`Node.appendChild`](https://developer.mozilla.org/en-US/docs/Web/API/Node/appendChild).
pub fn append_child(parent: &web_sys::Element, child: &web_sys::Node) -> web_sys::Node {
    parent.append_child(child).unwrap_throw()
}

/// Removes the child node from its parent element.
/// See [`Node.removeChild`](https://developer.mozilla.org/en-US/docs/Web/API/Node/removeChild).
pub fn remove_child(parent: &web_sys::Element, child: &web_sys::Node) {
    _ = parent.remove_child(child);
}

/// Replaces the old node with the new one, within the parent element.
/// See [`Node.replaceChild`](https://developer.mozilla.org/en-US/docs/Web/API/Node/replaceChild).
pub fn replace_child(parent: &web_sys::Element, new: &web_sys::Node, old: &web_sys::Node) {
    _ = parent.replace_child(new, old);
}

/// Inserts the new node before the existing node (or, if `None`, at the end of the parent's children.)
/// See [`Node.insertBefore`](https://developer.mozilla.org/en-US/docs/Web/API/Node/insertBefore).
pub fn insert_before(
    parent: &web_sys::Element,
    new: &web_sys::Node,
    existing: Option<&web_sys::Node>,
) -> web_sys::Node {
    if parent.node_type() != 1 {
        debug_warn!("insert_before: trying to insert on a parent node that is not an element");
        new.clone()
    } else if let Some(existing) = existing {
        let parent = existing.parent_node().unwrap_throw();
        match parent.insert_before(new, Some(existing)) {
            Ok(c) => c,
            Err(e) => {
                debug_warn!("{:?}", e.as_string());
                new.clone()
            }
        }
    } else {
        parent.append_child(new).unwrap_throw()
    }
}

/// Replace the old node with the new node in the DOM.
/// See [`Element.replaceWith`](https://developer.mozilla.org/en-US/docs/Web/API/Element/replaceWith).
pub fn replace_with(old_node: &web_sys::Element, new_node: &web_sys::Node) {
    _ = old_node.replace_with_with_node_1(new_node);
}

/// Sets the text of a DOM text node.
pub fn set_data(node: &web_sys::Text, value: &str) {
    node.set_data(value);
}

/// Sets the value of an attribute on a DOM element.
/// See [`Element.setAttribute`](https://developer.mozilla.org/en-US/docs/Web/API/Element/setAttribute).
pub fn set_attribute(el: &web_sys::Element, attr_name: &str, value: &str) {
    _ = el.set_attribute(attr_name, value);
}

/// Removes an attribute from a DOM element.
/// See [`Element.removeAttribute`](https://developer.mozilla.org/en-US/docs/Web/API/Element/removeAttribute).
pub fn remove_attribute(el: &web_sys::Element, attr_name: &str) {
    _ = el.remove_attribute(attr_name);
}

/// Sets a property on a DOM element.
pub fn set_property(el: &web_sys::Element, prop_name: &str, value: &Option<JsValue>) {
    let key = JsValue::from_str(prop_name);
    match value {
        Some(value) => _ = js_sys::Reflect::set(el, &key, value),
        None => _ = js_sys::Reflect::delete_property(el, &key),
    };
}

/// Returns the current [`window.location`](https://developer.mozilla.org/en-US/docs/Web/API/Window/location).
pub fn location() -> web_sys::Location {
    window().location()
}

/// Current [`window.location.hash`](https://developer.mozilla.org/en-US/docs/Web/API/Window/location)
/// without the beginning #.
pub fn location_hash() -> Option<String> {
    if is_server!() {
        None
    } else {
        location().hash().ok().map(|hash| hash.replace('#', ""))
    }
}

/// Current [`window.location.pathname`](https://developer.mozilla.org/en-US/docs/Web/API/Window/location).
pub fn location_pathname() -> Option<String> {
    location().pathname().ok()
}

/// Helper function to extract [`Event.target`](https://developer.mozilla.org/en-US/docs/Web/API/Event/target)
/// from any event.
pub fn event_target<T>(event: &web_sys::Event) -> T
where
    T: JsCast,
{
    event.target().unwrap_throw().unchecked_into::<T>()
}

/// Helper function to extract `event.target.value` from an event.
///
/// This is useful in the `on:input` or `on:change` listeners for an `<input>` element.
pub fn event_target_value(event: &web_sys::Event) -> String {
    event
        .target()
        .unwrap_throw()
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

/// Runs the given function between the next repaint
/// using [`Window.requestAnimationFrame`](https://developer.mozilla.org/en-US/docs/Web/API/window/requestAnimationFrame).
pub fn request_animation_frame(cb: impl Fn() + 'static) {
    let cb = Closure::wrap(Box::new(cb) as Box<dyn Fn()>).into_js_value();
    _ = window().request_animation_frame(cb.as_ref().unchecked_ref());
}

/// Queues the given function during an idle period  
/// using [`Window.requestIdleCallback`](https://developer.mozilla.org/en-US/docs/Web/API/window/requestIdleCallback).
pub fn request_idle_callback(cb: impl Fn() + 'static) {
    let cb = Closure::wrap(Box::new(cb) as Box<dyn Fn()>).into_js_value();
    _ = window().request_idle_callback(cb.as_ref().unchecked_ref());
}

/// Executes the given function after the given duration of time has passed.
/// [`setTimeout()`](https://developer.mozilla.org/en-US/docs/Web/API/setTimeout).
pub fn set_timeout(cb: impl FnOnce() + 'static, duration: Duration) {
    let cb = Closure::once_into_js(Box::new(cb) as Box<dyn FnOnce()>);
    _ = window().set_timeout_with_callback_and_timeout_and_arguments_0(
        cb.as_ref().unchecked_ref(),
        duration.as_millis().try_into().unwrap_throw(),
    );
}

/// Handle that is generated by [set_interval] and can be used to clear the interval.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct IntervalHandle(i32);

impl IntervalHandle {
    /// Cancels the repeating event to which this refers.
    /// See [`clearInterval()`](https://developer.mozilla.org/en-US/docs/Web/API/clearInterval)
    pub fn clear(&self) {
        window().clear_interval_with_handle(self.0);
    }
}

/// Repeatedly calls the given function, with a delay of the given duration between calls.
/// See [`setInterval()`](https://developer.mozilla.org/en-US/docs/Web/API/setInterval).
pub fn set_interval(
    cb: impl Fn() + 'static,
    duration: Duration,
) -> Result<IntervalHandle, JsValue> {
    let cb = Closure::wrap(Box::new(cb) as Box<dyn Fn()>).into_js_value();
    let handle = window().set_interval_with_callback_and_timeout_and_arguments_0(
        cb.as_ref().unchecked_ref(),
        duration.as_millis().try_into().unwrap_throw(),
    )?;
    Ok(IntervalHandle(handle))
}

/// Adds an event listener to the target DOM element using implicit event delegation.
pub fn add_event_listener(
    target: &web_sys::Element,
    event_name: &'static str,
    cb: impl FnMut(web_sys::Event) + 'static,
) {
    let cb = Closure::wrap(Box::new(cb) as Box<dyn FnMut(web_sys::Event)>).into_js_value();
    let key = event_delegation::event_delegation_key(event_name);
    _ = js_sys::Reflect::set(target, &JsValue::from_str(&key), &cb);
    event_delegation::add_event_listener(event_name);
}

#[doc(hidden)]
pub fn add_event_listener_undelegated(
    target: &web_sys::Element,
    event_name: &'static str,
    cb: impl FnMut(web_sys::Event) + 'static,
) {
    let cb = Closure::wrap(Box::new(cb) as Box<dyn FnMut(web_sys::Event)>).into_js_value();
    _ = target.add_event_listener_with_callback(event_name, cb.unchecked_ref());
}

#[inline(always)]
pub fn ssr_event_listener(_cb: impl FnMut(web_sys::Event) + 'static) {
    // this function exists only for type inference in templates for SSR
}

/// Adds an event listener to the `Window`.
pub fn window_event_listener(event_name: &str, cb: impl Fn(web_sys::Event) + 'static) {
    if !is_server!() {
        let handler = Box::new(cb) as Box<dyn FnMut(web_sys::Event)>;

        let cb = Closure::wrap(handler).into_js_value();
        _ = window().add_event_listener_with_callback(event_name, cb.unchecked_ref());
    }
}

/// Removes all event listeners from an element.
pub fn remove_event_listeners(el: &web_sys::Element) {
    let clone = el.clone_node().unwrap_throw();
    replace_with(el, clone.unchecked_ref());
}
