use super::{CastFrom, RemoveEventHandler};
use crate::{
    dom::{document, window},
    ok_or_debug, or_debug,
    view::{Mountable, ToTemplate},
};
use linear_map::LinearMap;
use once_cell::unsync::Lazy;
use rustc_hash::FxHashSet;
use std::{any::TypeId, borrow::Cow, cell::RefCell};
use wasm_bindgen::{intern, prelude::Closure, JsCast, JsValue};
use web_sys::{Comment, HtmlTemplateElement};

/// A [`Renderer`] that uses `web-sys` to manipulate DOM elements in the browser.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Dom;

thread_local! {
    pub(crate) static GLOBAL_EVENTS: RefCell<FxHashSet<Cow<'static, str>>> = Default::default();
}

pub type Node = web_sys::Node;
pub type Text = web_sys::Text;
pub type Element = web_sys::Element;
pub type Placeholder = web_sys::Comment;
pub type Event = wasm_bindgen::JsValue;
pub type ClassList = web_sys::DomTokenList;
pub type CssStyleDeclaration = web_sys::CssStyleDeclaration;
pub type TemplateElement = web_sys::HtmlTemplateElement;

impl Dom {
    pub fn intern(text: &str) -> &str {
        intern(text)
    }

    pub fn create_element(tag: &str, namespace: Option<&str>) -> Element {
        if let Some(namespace) = namespace {
            document()
                .create_element_ns(
                    Some(Self::intern(namespace)),
                    Self::intern(tag),
                )
                .unwrap()
        } else {
            document().create_element(Self::intern(tag)).unwrap()
        }
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(level = "trace"))]
    pub fn create_text_node(text: &str) -> Text {
        document().create_text_node(text)
    }

    pub fn create_placeholder() -> Placeholder {
        thread_local! {
            static COMMENT: Lazy<Comment> = Lazy::new(|| {
                document().create_comment("")
            });
        }
        COMMENT.with(|n| n.clone_node().unwrap().unchecked_into())
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(level = "trace"))]
    pub fn set_text(node: &Text, text: &str) {
        node.set_node_value(Some(text));
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(level = "trace"))]
    pub fn set_attribute(node: &Element, name: &str, value: &str) {
        or_debug!(node.set_attribute(name, value), node, "setAttribute");
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(level = "trace"))]
    pub fn remove_attribute(node: &Element, name: &str) {
        or_debug!(node.remove_attribute(name), node, "removeAttribute");
    }

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

    #[cfg_attr(feature = "tracing", tracing::instrument(level = "trace"))]
    pub fn remove_node(parent: &Element, child: &Node) -> Option<Node> {
        ok_or_debug!(parent.remove_child(child), parent, "removeNode")
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(level = "trace"))]
    pub fn remove(node: &Node) {
        node.unchecked_ref::<Element>().remove();
    }

    pub fn get_parent(node: &Node) -> Option<Node> {
        node.parent_node()
    }

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

    pub fn log_node(node: &Node) {
        web_sys::console::log_1(node);
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(level = "trace"))]
    pub fn clear_children(parent: &Element) {
        parent.set_text_content(Some(""));
    }

    /// Mounts the new child before the marker as its sibling.
    ///
    /// ## Panics
    /// The default implementation panics if `before` does not have a parent [`crate::renderer::types::Element`].
    pub fn mount_before<M>(new_child: &mut M, before: &Node)
    where
        M: Mountable,
    {
        let parent = Element::cast_from(
            Self::get_parent(before).expect("could not find parent element"),
        )
        .expect("placeholder parent should be Element");
        new_child.mount(&parent, Some(before));
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
            // safe to construct this here, because it will only run in the browser
            // so it will always be accessed or dropped from the main thread
            let cb = send_wrapper::SendWrapper::new(cb);
            move |el: &Element| {
                or_debug!(
                    el.remove_event_listener_with_callback(
                        intern(&name),
                        cb.as_ref().unchecked_ref()
                    ),
                    el,
                    "removeEventListener"
                )
            }
        })
    }

    pub fn event_target<T>(ev: &Event) -> T
    where
        T: CastFrom<Element>,
    {
        let el = ev
            .unchecked_ref::<web_sys::Event>()
            .target()
            .expect("event.target not found")
            .unchecked_into::<Element>();
        T::cast_from(el).expect("incorrect element type")
    }

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

        GLOBAL_EVENTS.with(|global_events| {
            let mut events = global_events.borrow_mut();
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
            // safe to construct this here, because it will only run in the browser
            // so it will always be accessed or dropped from the main thread
            let cb = send_wrapper::SendWrapper::new(cb);
            move |el: &Element| {
                drop(cb.take());
                or_debug!(
                    js_sys::Reflect::delete_property(
                        el,
                        &JsValue::from_str(&key)
                    ),
                    el,
                    "delete property"
                );
            }
        })
    }

    pub fn class_list(el: &Element) -> ClassList {
        el.class_list()
    }

    pub fn add_class(list: &ClassList, name: &str) {
        or_debug!(list.add_1(name), list.unchecked_ref(), "add()");
    }

    pub fn remove_class(list: &ClassList, name: &str) {
        or_debug!(list.remove_1(name), list.unchecked_ref(), "remove()");
    }

    pub fn style(el: &Element) -> CssStyleDeclaration {
        el.unchecked_ref::<web_sys::HtmlElement>().style()
    }

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

    pub fn set_inner_html(el: &Element, html: &str) {
        el.set_inner_html(html);
    }

    pub fn get_template<V>() -> TemplateElement
    where
        V: ToTemplate + 'static,
    {
        thread_local! {
            static TEMPLATE_ELEMENT: Lazy<HtmlTemplateElement> =
                Lazy::new(|| document().create_element("template").unwrap().unchecked_into());
            static TEMPLATES: RefCell<LinearMap<TypeId, HtmlTemplateElement>> = Default::default();
        }

        TEMPLATES.with(|t| {
            t.borrow_mut()
                .entry(TypeId::of::<V>())
                .or_insert_with(|| {
                    let tpl = TEMPLATE_ELEMENT.with(|t| {
                        t.clone_node()
                            .unwrap()
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
                    tpl
                })
                .clone()
        })
    }

    pub fn clone_template(tpl: &TemplateElement) -> Element {
        tpl.content()
            .clone_node_with_deep(true)
            .unwrap()
            .unchecked_into()
    }

    pub fn create_element_from_html(html: &str) -> Element {
        // TODO can be optimized to cache HTML strings or cache <template>?
        let tpl = document().create_element("template").unwrap();
        tpl.set_inner_html(html);
        Self::clone_template(tpl.unchecked_ref())
    }
}

impl Mountable for Node {
    fn unmount(&mut self) {
        todo!()
    }

    fn mount(&mut self, parent: &Element, marker: Option<&Node>) {
        Dom::insert_node(parent, self, marker);
    }

    fn insert_before_this(&self, child: &mut dyn Mountable) -> bool {
        let parent = Dom::get_parent(self).and_then(Element::cast_from);
        if let Some(parent) = parent {
            child.mount(&parent, Some(self));
            return true;
        }
        false
    }
}

impl Mountable for Text {
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
}

impl Mountable for Comment {
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
