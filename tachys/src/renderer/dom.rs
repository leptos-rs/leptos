//! See [`Renderer`](crate::renderer::Renderer) and [`Rndr`](crate::renderer::Rndr) for additional information.

use super::{CastFrom, RemoveEventHandler};
use crate::{
    dom::{document, window},
    ok_or_debug, or_debug,
    view::{Mountable, ToTemplate},
};
use rustc_hash::FxHashSet;
use std::{
    any::TypeId,
    borrow::Cow,
    cell::{LazyCell, RefCell},
};
use wasm_bindgen::{intern, prelude::Closure, JsCast, JsValue};
use web_sys::{AddEventListenerOptions, Comment, HtmlTemplateElement};

/// A [`Renderer`](crate::renderer::Renderer) that uses `web-sys` to manipulate DOM elements in the browser.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Dom;

thread_local! {
    /// A set of events that have already been delegated.
    pub(crate) static GLOBAL_EVENTS: RefCell<FxHashSet<Cow<'static, str>>> = Default::default();
    /// A cache of template elements.
    pub static TEMPLATE_CACHE: RefCell<Vec<(Cow<'static, str>, web_sys::Element)>> = Default::default();
}

/// A DOM node.
pub type Node = web_sys::Node;
/// A DOM text node.
pub type Text = web_sys::Text;
/// A DOM element.
pub type Element = web_sys::Element;
/// A placeholder node.
pub type Placeholder = web_sys::Comment;
/// A DOM event.
pub type Event = wasm_bindgen::JsValue;
/// A DOM token list.
pub type ClassList = web_sys::DomTokenList;
/// A DOM CSS style declaration.
pub type CssStyleDeclaration = web_sys::CssStyleDeclaration;
/// A DOM HTML template element.
pub type TemplateElement = web_sys::HtmlTemplateElement;

/// A microtask is a short function which will run after the current task has
/// completed its work and when there is no other code waiting to be run before
/// control of the execution context is returned to the browser's event loop.
///
/// Microtasks are especially useful for libraries and frameworks that need
/// to perform final cleanup or other just-before-rendering tasks.
///
/// [MDN queueMicrotask](https://developer.mozilla.org/en-US/docs/Web/API/queueMicrotask)
pub fn queue_microtask(task: impl FnOnce() + 'static) {
    use js_sys::{Function, Reflect};

    let task = Closure::once_into_js(task);
    let window = window();
    let queue_microtask =
        Reflect::get(&window, &JsValue::from_str("queueMicrotask"))
            .expect("queueMicrotask not available");
    let queue_microtask = queue_microtask.unchecked_into::<Function>();
    _ = queue_microtask.call1(&JsValue::UNDEFINED, &task);
}

fn queue(fun: Box<dyn FnOnce()>) {
    use std::cell::{Cell, RefCell};

    thread_local! {
        static PENDING: Cell<bool> = const { Cell::new(false) };
        static QUEUE: RefCell<Vec<Box<dyn FnOnce()>>> = RefCell::new(Vec::new());
    }

    QUEUE.with_borrow_mut(|q| q.push(fun));
    if !PENDING.replace(true) {
        queue_microtask(|| {
            let tasks = QUEUE.take();
            for task in tasks {
                task();
            }
            PENDING.set(false);
        })
    }
}

impl Dom {
    /// Interns a string in the JS engine.
    pub fn intern(text: &str) -> &str {
        intern(text)
    }

    /// Creates a new element with the given tag name and optional namespace.
    pub fn create_element(tag: &str, namespace: Option<&str>) -> Element {
        thread_local! {
            static DIV: &'static str = Dom::intern("div");
        }
        if let Some(namespace) = namespace {
            document()
                .create_element_ns(
                    Some(Self::intern(namespace)),
                    Self::intern(tag),
                )
                .unwrap_or_else(|_| {
                    #[cfg(all(target_arch = "wasm32", debug_assertions))]
                    web_sys::console::error_2(
                        &"Failed to create element with namespace:".into(),
                        &tag.into(),
                    );
                    document().create_element(DIV.with(|d| *d)).unwrap_or_else(
                        |_| unreachable!("Could not even create a <div>"),
                    )
                })
        } else {
            document()
                .create_element(Self::intern(tag))
                .unwrap_or_else(|_| {
                    #[cfg(all(target_arch = "wasm32", debug_assertions))]
                    web_sys::console::error_2(
                        &"Failed to create element:".into(),
                        &tag.into(),
                    );
                    document().create_element(DIV.with(|d| *d)).unwrap_or_else(
                        |_| unreachable!("Could not even create a <div>"),
                    )
                })
        }
    }

    /// Creates a new text node with the given text.
    #[cfg_attr(feature = "tracing", tracing::instrument(level = "trace"))]
    pub fn create_text_node(text: &str) -> Text {
        document().create_text_node(text)
    }

    /// Creates a new placeholder node.
    pub fn create_placeholder() -> Placeholder {
        thread_local! {
            static COMMENT: LazyCell<Comment> = LazyCell::new(|| {
                document().create_comment("")
            });
        }
        COMMENT.with(|n| {
            n.clone_node()
                .unwrap_or_else(|_| {
                    #[cfg(all(target_arch = "wasm32", debug_assertions))]
                    web_sys::console::error_1(
                        &"Failed to clone placeholder node".into(),
                    );
                    document().create_comment("").unchecked_into()
                })
                .unchecked_into()
        })
    }

    /// Sets the text content of a text node.
    #[cfg_attr(feature = "tracing", tracing::instrument(level = "trace"))]
    pub fn set_text(node: &Text, text: &str) {
        node.set_node_value(Some(text));
    }

    /// Sets an attribute on an element.
    #[cfg_attr(feature = "tracing", tracing::instrument(level = "trace"))]
    pub fn set_attribute(node: &Element, name: &str, value: &str) {
        or_debug!(node.set_attribute(name, value), node, "setAttribute");
    }

    /// Removes an attribute from an element.
    #[cfg_attr(feature = "tracing", tracing::instrument(level = "trace"))]
    pub fn remove_attribute(node: &Element, name: &str) {
        or_debug!(node.remove_attribute(name), node, "removeAttribute");
    }

    /// Inserts a node before an anchor node in a parent element.
    #[cfg_attr(feature = "tracing", tracing::instrument(level = "trace"))]
    pub fn insert_node(
        parent: &Element,
        new_child: &Node,
        anchor: Option<&Node>,
    ) {
        ok_or_debug!(
            parent.insert_before(new_child, anchor),
            parent,
            "insertNode"
        );
    }

    /// Tries to insert a node before an anchor node in a parent element.
    #[cfg_attr(feature = "tracing", tracing::instrument(level = "trace"))]
    pub fn try_insert_node(
        parent: &Element,
        new_child: &Node,
        anchor: Option<&Node>,
    ) -> bool {
        parent.insert_before(new_child, anchor).is_ok()
    }

    /// Removes a node from a parent element.
    #[cfg_attr(feature = "tracing", tracing::instrument(level = "trace"))]
    pub fn remove_node(parent: &Element, child: &Node) -> Option<Node> {
        ok_or_debug!(parent.remove_child(child), parent, "removeNode")
    }

    /// Removes a node from its parent.
    #[cfg_attr(feature = "tracing", tracing::instrument(level = "trace"))]
    pub fn remove(node: &Node) {
        node.unchecked_ref::<Element>().remove();
    }

    /// Returns the parent of a node.
    pub fn get_parent(node: &Node) -> Option<Node> {
        node.parent_node()
    }

    /// Returns the first child of a node.
    pub fn first_child(node: &Node) -> Option<Node> {
        #[cfg(debug_assertions)]
        {
            let node = node.first_child();
            // if it's a comment node that starts with hot-reload, it's a marker that should be
            // ignored
            if let Some(node) = node.as_ref() {
                if node.node_type() == 8
                    && node
                        .text_content()
                        .unwrap_or_default()
                        .starts_with("hot-reload")
                {
                    return Self::next_sibling(node);
                }
            }

            node
        }
        #[cfg(not(debug_assertions))]
        {
            node.first_child()
        }
    }

    /// Returns the next sibling of a node.
    pub fn next_sibling(node: &Node) -> Option<Node> {
        #[cfg(debug_assertions)]
        {
            let node = node.next_sibling();
            // if it's a comment node that starts with hot-reload, it's a marker that should be
            // ignored
            if let Some(node) = node.as_ref() {
                if node.node_type() == 8
                    && node
                        .text_content()
                        .unwrap_or_default()
                        .starts_with("hot-reload")
                {
                    return Self::next_sibling(node);
                }
            }

            node
        }
        #[cfg(not(debug_assertions))]
        {
            node.next_sibling()
        }
    }

    /// Logs a node to the console.
    pub fn log_node(node: &Node) {
        web_sys::console::log_1(node);
    }

    /// Clears all children of a parent element.
    #[cfg_attr(feature = "tracing", tracing::instrument(level = "trace"))]
    pub fn clear_children(parent: &Element) {
        parent.set_text_content(Some(""));
    }

    /// Mounts the new child before the marker as its sibling.
    pub fn mount_before<M>(new_child: &mut M, before: &Node)
    where
        M: Mountable,
    {
        if !Self::try_mount_before(new_child, before) {
            #[cfg(all(target_arch = "wasm32", debug_assertions))]
            web_sys::console::error_1(
                &"could not find parent element to mount before".into(),
            );
        }
    }

    /// Tries to mount the new child before the marker as its sibling.
    ///
    /// Returns `false` if the child did not have a valid parent.
    #[track_caller]
    pub fn try_mount_before<M>(new_child: &mut M, before: &Node) -> bool
    where
        M: Mountable,
    {
        if let Some(parent) =
            Self::get_parent(before).and_then(Element::cast_from)
        {
            new_child.mount(&parent, Some(before));
            true
        } else {
            false
        }
    }

    /// Sets a property or value on an element.
    pub fn set_property_or_value(el: &Element, key: &str, value: &JsValue) {
        if key == "value" {
            queue(Box::new({
                let el = el.clone();
                let value = value.clone();
                move || {
                    Self::set_property(&el, "value", &value);
                }
            }))
        } else {
            Self::set_property(el, key, value);
        }
    }

    /// Sets a property on an element.
    pub fn set_property(el: &Element, key: &str, value: &JsValue) {
        or_debug!(
            js_sys::Reflect::set(
                el,
                &wasm_bindgen::JsValue::from_str(key),
                value,
            ),
            el,
            "setProperty"
        );
    }

    /// Adds an event listener to an element.
    pub fn add_event_listener(
        el: &Element,
        name: &str,
        cb: Box<dyn FnMut(Event)>,
    ) -> RemoveEventHandler<Element> {
        let cb = wasm_bindgen::closure::Closure::wrap(cb);
        let name = intern(name);
        or_debug!(
            el.add_event_listener_with_callback(
                name,
                cb.as_ref().unchecked_ref()
            ),
            el,
            "addEventListener"
        );

        // return the remover
        RemoveEventHandler::new({
            let name = name.to_owned();
            let el = el.clone();
            // safe to construct this here, because it will only run in the browser
            // so it will always be accessed or dropped from the main thread
            let cb = send_wrapper::SendWrapper::new(move || {
                or_debug!(
                    el.remove_event_listener_with_callback(
                        intern(&name),
                        cb.as_ref().unchecked_ref()
                    ),
                    &el,
                    "removeEventListener"
                )
            });
            move || cb()
        })
    }

    /// Adds an event listener to an element using capture.
    pub fn add_event_listener_use_capture(
        el: &Element,
        name: &str,
        cb: Box<dyn FnMut(Event)>,
    ) -> RemoveEventHandler<Element> {
        let cb = wasm_bindgen::closure::Closure::wrap(cb);
        let name = intern(name);
        let options = AddEventListenerOptions::new();
        options.set_capture(true);
        or_debug!(
            el.add_event_listener_with_callback_and_add_event_listener_options(
                name,
                cb.as_ref().unchecked_ref(),
                &options
            ),
            el,
            "addEventListenerUseCapture"
        );

        // return the remover
        RemoveEventHandler::new({
            let name = name.to_owned();
            let el = el.clone();
            // safe to construct this here, because it will only run in the browser
            // so it will always be accessed or dropped from the main thread
            let cb = send_wrapper::SendWrapper::new(move || {
                or_debug!(
                    el.remove_event_listener_with_callback_and_bool(
                        intern(&name),
                        cb.as_ref().unchecked_ref(),
                        true
                    ),
                    &el,
                    "removeEventListener"
                )
            });
            move || cb()
        })
    }

    /// Returns the target of an event.
    pub fn event_target<T>(ev: &Event) -> T
    where
        T: CastFrom<Element>,
    {
        let el = ev
            .unchecked_ref::<web_sys::Event>()
            .target()
            .and_then(|t| t.dyn_into::<Element>().ok())
            .expect("event.target not found or not an element");
        T::cast_from(el).expect("incorrect element type")
    }

    /// Adds a delegated event listener to an element.
    pub fn add_event_listener_delegated(
        el: &Element,
        name: Cow<'static, str>,
        delegation_key: Cow<'static, str>,
        cb: Box<dyn FnMut(Event)>,
    ) -> RemoveEventHandler<Element> {
        let cb = Closure::wrap(cb);
        let key = intern(&delegation_key);
        or_debug!(
            js_sys::Reflect::set(el, &JsValue::from_str(key), cb.as_ref()),
            el,
            "set property"
        );

        GLOBAL_EVENTS.with_borrow_mut(|events| {
            if !events.contains(&name) {
                // create global handler
                let key = JsValue::from_str(key);
                let handler = move |ev: web_sys::Event| {
                    let target = ev.target();
                    let node = ev.composed_path().get(0);
                    let mut node = if node.is_undefined() || node.is_null() {
                        JsValue::from(target)
                    } else {
                        node
                    };

                    // TODO reverse Shadow DOM retargetting
                    // TODO simulate currentTarget

                    while !node.is_null() {
                        let node_is_disabled = js_sys::Reflect::get(
                            &node,
                            &JsValue::from_str("disabled"),
                        )
                        .unwrap()
                        .is_truthy();
                        if !node_is_disabled {
                            let maybe_handler =
                                js_sys::Reflect::get(&node, &key).unwrap();
                            if !maybe_handler.is_undefined() {
                                let f = maybe_handler
                                    .unchecked_ref::<js_sys::Function>();
                                let _ = f.call1(&node, &ev);

                                if ev.cancel_bubble() {
                                    return;
                                }
                            }
                        }

                        // navigate up tree
                        if let Some(parent) =
                            node.unchecked_ref::<web_sys::Node>().parent_node()
                        {
                            node = parent.into()
                        } else if let Some(root) =
                            node.dyn_ref::<web_sys::ShadowRoot>()
                        {
                            node = root.host().unchecked_into();
                        } else {
                            node = JsValue::null()
                        }
                    }
                };

                let handler =
                    Box::new(handler) as Box<dyn FnMut(web_sys::Event)>;
                let handler = Closure::wrap(handler).into_js_value();
                window()
                    .add_event_listener_with_callback(
                        &name,
                        handler.unchecked_ref(),
                    )
                    .unwrap();

                // register that we've created handler
                events.insert(name);
            }
        });

        // return the remover
        RemoveEventHandler::new({
            let key = key.to_owned();
            let el = el.clone();
            // safe to construct this here, because it will only run in the browser
            // so it will always be accessed or dropped from the main thread
            let el_cb = send_wrapper::SendWrapper::new((el, cb));
            move || {
                let (el, cb) = el_cb.take();
                drop(cb);
                or_debug!(
                    js_sys::Reflect::delete_property(
                        &el,
                        &JsValue::from_str(&key)
                    ),
                    &el,
                    "delete property"
                );
            }
        })
    }

    /// Returns the class list of an element.
    pub fn class_list(el: &Element) -> ClassList {
        el.class_list()
    }

    /// Adds a class to a class list.
    pub fn add_class(list: &ClassList, name: &str) {
        or_debug!(list.add_1(name), list.unchecked_ref(), "add()");
    }

    /// Removes a class from a class list.
    pub fn remove_class(list: &ClassList, name: &str) {
        or_debug!(list.remove_1(name), list.unchecked_ref(), "remove()");
    }

    /// Returns the style declaration of an element.
    pub fn style(el: &Element) -> CssStyleDeclaration {
        el.unchecked_ref::<web_sys::HtmlElement>().style()
    }

    /// Sets a CSS property on a style declaration.
    pub fn set_css_property(
        style: &CssStyleDeclaration,
        name: &str,
        value: &str,
    ) {
        or_debug!(
            style.set_property(name, value),
            style.unchecked_ref(),
            "setProperty"
        );
    }

    /// Removes a CSS property from a style declaration.
    pub fn remove_css_property(style: &CssStyleDeclaration, name: &str) {
        or_debug!(
            style.remove_property(name),
            style.unchecked_ref(),
            "removeProperty"
        );
    }

    /// Sets the inner HTML of an element.
    pub fn set_inner_html(el: &Element, html: &str) {
        el.set_inner_html(html);
    }

    /// Returns a template element for a type.
    pub fn get_template<V>() -> TemplateElement
    where
        V: ToTemplate + 'static,
    {
        thread_local! {
            static TEMPLATE_ELEMENT: LazyCell<HtmlTemplateElement> =
                LazyCell::new(|| document().create_element(Dom::intern("template")).unwrap_or_else(|_| {
                    unreachable!("Could not create a <template> element")
                }).unchecked_into());
            static TEMPLATES: RefCell<Vec<(TypeId, HtmlTemplateElement)>> = Default::default();
        }

        TEMPLATES.with_borrow_mut(|t| {
            let id = TypeId::of::<V>();
            t.iter()
                .find_map(|entry| (entry.0 == id).then(|| entry.1.clone()))
                .unwrap_or_else(|| {
                    let tpl = TEMPLATE_ELEMENT.with(|t| {
                        t.clone_node()
                            .unwrap_or_else(|_| {
                                #[cfg(all(
                                    target_arch = "wasm32",
                                    debug_assertions
                                ))]
                                web_sys::console::error_1(
                                    &"Failed to clone template element".into(),
                                );
                                let t: &HtmlTemplateElement = &*t;
                                t.clone().into()
                            })
                            .unchecked_into::<HtmlTemplateElement>()
                    });
                    let mut buf = String::new();
                    V::to_template(
                        &mut buf,
                        &mut String::new(),
                        &mut String::new(),
                        &mut String::new(),
                        &mut Default::default(),
                    );
                    tpl.set_inner_html(&buf);
                    t.push((id, tpl.clone()));
                    tpl
                })
        })
    }

    /// Clones a template element.
    pub fn clone_template(tpl: &TemplateElement) -> Element {
        tpl.content()
            .clone_node_with_deep(true)
            .unwrap_or_else(|_| {
                #[cfg(all(target_arch = "wasm32", debug_assertions))]
                web_sys::console::error_1(
                    &"Failed to clone template content".into(),
                );
                tpl.content().into()
            })
            .unchecked_into()
    }

    /// Creates an element from HTML.
    pub fn create_element_from_html(html: Cow<'static, str>) -> Element {
        let tpl = TEMPLATE_CACHE.with_borrow_mut(|cache| {
            if let Some(tpl_content) = cache.iter().find_map(|(key, tpl)| {
                (html == *key)
                    .then_some(Self::clone_template(tpl.unchecked_ref()))
            }) {
                tpl_content
            } else {
                let tpl = document()
                    .create_element(Self::intern("template"))
                    .unwrap_or_else(|_| {
                        unreachable!("Could not create a <template> element")
                    });
                tpl.set_inner_html(&html);
                let tpl_content = Self::clone_template(tpl.unchecked_ref());
                cache.push((html, tpl));
                tpl_content
            }
        });
        tpl.first_element_child().unwrap_or(tpl)
    }

    /// Creates an SVG element from HTML.
    pub fn create_svg_element_from_html(html: Cow<'static, str>) -> Element {
        let tpl = TEMPLATE_CACHE.with_borrow_mut(|cache| {
            if let Some(tpl_content) = cache.iter().find_map(|(key, tpl)| {
                (html == *key)
                    .then_some(Self::clone_template(tpl.unchecked_ref()))
            }) {
                tpl_content
            } else {
                let tpl = document()
                    .create_element(Self::intern("template"))
                    .unwrap_or_else(|_| {
                        unreachable!("Could not create a <template> element")
                    });
                let svg = document()
                    .create_element_ns(
                        Some(Self::intern("http://www.w3.org/2000/svg")),
                        Self::intern("svg"),
                    )
                    .unwrap_or_else(|_| {
                        document()
                            .create_element(Self::intern("svg"))
                            .unwrap_or_else(|_| {
                                unreachable!(
                                    "Could not even create a non-namespaced \
                                     <svg>"
                                )
                            })
                    });
                let g = document()
                    .create_element_ns(
                        Some(Self::intern("http://www.w3.org/2000/svg")),
                        Self::intern("g"),
                    )
                    .unwrap_or_else(|_| {
                        document()
                            .create_element(Self::intern("g"))
                            .unwrap_or_else(|_| {
                                unreachable!(
                                    "Could not even create a non-namespaced \
                                     <g>"
                                )
                            })
                    });
                g.set_inner_html(&html);
                _ = svg.append_child(&g);
                _ = tpl
                    .unchecked_ref::<TemplateElement>()
                    .content()
                    .append_child(&svg);
                let tpl_content = Self::clone_template(tpl.unchecked_ref());
                cache.push((html, tpl));
                tpl_content
            }
        });

        let svg = tpl.first_element_child().unwrap_or(tpl.clone());
        svg.first_element_child().unwrap_or(svg)
    }
}

impl Mountable for Node {
    fn unmount(&mut self) {
        todo!()
    }

    fn mount(&mut self, parent: &Element, marker: Option<&Node>) {
        Dom::insert_node(parent, self, marker);
    }

    fn try_mount(&mut self, parent: &Element, marker: Option<&Node>) -> bool {
        Dom::try_insert_node(parent, self, marker)
    }

    fn insert_before_this(&self, child: &mut dyn Mountable) -> bool {
        let parent = Dom::get_parent(self).and_then(Element::cast_from);
        if let Some(parent) = parent {
            child.mount(&parent, Some(self));
            return true;
        }
        false
    }

    fn elements(&self) -> Vec<crate::renderer::types::Element> {
        vec![]
    }
}

impl Mountable for Text {
    fn unmount(&mut self) {
        self.remove();
    }

    fn mount(&mut self, parent: &Element, marker: Option<&Node>) {
        Dom::insert_node(parent, self, marker);
    }

    fn try_mount(&mut self, parent: &Element, marker: Option<&Node>) -> bool {
        Dom::try_insert_node(parent, self, marker)
    }

    fn insert_before_this(&self, child: &mut dyn Mountable) -> bool {
        let parent =
            Dom::get_parent(self.as_ref()).and_then(Element::cast_from);
        if let Some(parent) = parent {
            child.mount(&parent, Some(self));
            return true;
        }
        false
    }

    fn elements(&self) -> Vec<crate::renderer::types::Element> {
        vec![]
    }
}

impl Mountable for Comment {
    fn unmount(&mut self) {
        self.remove();
    }

    fn mount(&mut self, parent: &Element, marker: Option<&Node>) {
        Dom::insert_node(parent, self, marker);
    }

    fn try_mount(&mut self, parent: &Element, marker: Option<&Node>) -> bool {
        Dom::try_insert_node(parent, self, marker)
    }

    fn insert_before_this(&self, child: &mut dyn Mountable) -> bool {
        let parent =
            Dom::get_parent(self.as_ref()).and_then(Element::cast_from);
        if let Some(parent) = parent {
            child.mount(&parent, Some(self));
            return true;
        }
        false
    }

    fn elements(&self) -> Vec<crate::renderer::types::Element> {
        vec![]
    }
}

impl Mountable for Element {
    fn unmount(&mut self) {
        self.remove();
    }

    fn mount(&mut self, parent: &Element, marker: Option<&Node>) {
        Dom::insert_node(parent, self, marker);
    }

    fn insert_before_this(&self, child: &mut dyn Mountable) -> bool {
        let parent =
            Dom::get_parent(self.as_ref()).and_then(Element::cast_from);
        if let Some(parent) = parent {
            child.mount(&parent, Some(self));
            return true;
        }
        false
    }

    fn elements(&self) -> Vec<crate::renderer::types::Element> {
        vec![self.clone()]
    }
}

impl CastFrom<Node> for Text {
    fn cast_from(node: Node) -> Option<Text> {
        node.clone().dyn_into().ok()
    }
}

impl CastFrom<Node> for Comment {
    fn cast_from(node: Node) -> Option<Comment> {
        node.clone().dyn_into().ok()
    }
}

impl CastFrom<Node> for Element {
    fn cast_from(node: Node) -> Option<Element> {
        node.clone().dyn_into().ok()
    }
}

impl<T> CastFrom<JsValue> for T
where
    T: JsCast,
{
    fn cast_from(source: JsValue) -> Option<Self> {
        source.dyn_into::<T>().ok()
    }
}

impl<T> CastFrom<Element> for T
where
    T: JsCast,
{
    fn cast_from(source: Element) -> Option<Self> {
        source.dyn_into::<T>().ok()
    }
}
