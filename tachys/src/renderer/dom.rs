use super::{CastFrom, DomRenderer, Renderer};
use crate::{
    dom::{document, window},
    ok_or_debug, or_debug,
    view::Mountable,
};
use rustc_hash::FxHashSet;
use std::{borrow::Cow, cell::RefCell};
use wasm_bindgen::{intern, prelude::Closure, JsCast, JsValue};
use web_sys::{
    Comment, CssStyleDeclaration, DocumentFragment, DomTokenList, Element,
    Event, HtmlElement, Node, Text,
};

#[derive(Debug)]
pub struct Dom;

thread_local! {
    pub(crate) static GLOBAL_EVENTS: RefCell<FxHashSet<Cow<'static, str>>> = Default::default();
}

impl Renderer for Dom {
    type Node = Node;
    type Text = Text;
    type Element = Element;
    type Placeholder = Comment;

    fn create_text_node(text: &str) -> Self::Text {
        document().create_text_node(text)
    }

    fn create_placeholder() -> Self::Placeholder {
        document().create_comment("")
    }

    fn set_text(node: &Self::Text, text: &str) {
        node.set_node_value(Some(text));
    }

    fn set_attribute(node: &Self::Element, name: &str, value: &str) {
        or_debug!(
            node.set_attribute(intern(name), value),
            node,
            "setAttribute"
        );
    }

    fn remove_attribute(node: &Self::Element, name: &str) {
        or_debug!(node.remove_attribute(intern(name)), node, "removeAttribute");
    }

    fn insert_node(
        parent: &Self::Element,
        new_child: &Self::Node,
        anchor: Option<&Self::Node>,
    ) {
        ok_or_debug!(
            parent.insert_before(new_child, anchor),
            parent,
            "insertNode"
        );
    }

    fn remove_node(
        parent: &Self::Element,
        child: &Self::Node,
    ) -> Option<Self::Node> {
        ok_or_debug!(parent.remove_child(child), parent, "removeNode")
    }

    fn remove(node: &Self::Node) {
        node.unchecked_ref::<Element>().remove();
    }

    fn get_parent(node: &Self::Node) -> Option<Self::Node> {
        node.parent_node()
    }

    fn first_child(node: &Self::Node) -> Option<Self::Node> {
        node.first_child()
    }

    fn next_sibling(node: &Self::Node) -> Option<Self::Node> {
        node.next_sibling()
    }

    fn log_node(node: &Self::Node) {
        web_sys::console::log_1(node);
    }

    fn clear_children(parent: &Self::Element) {
        parent.set_text_content(Some(""));
    }
}

impl DomRenderer for Dom {
    type Event = JsValue;
    type ClassList = DomTokenList;
    type CssStyleDeclaration = CssStyleDeclaration;

    fn set_property(el: &Self::Element, key: &str, value: &JsValue) {
        or_debug!(
            js_sys::Reflect::set(
                el,
                &wasm_bindgen::JsValue::from_str(intern(key)),
                value,
            ),
            el,
            "setProperty"
        );
    }

    fn add_event_listener(
        el: &Self::Element,
        name: &str,
        cb: Box<dyn FnMut(Self::Event)>,
    ) -> Box<dyn FnOnce(&Self::Element)> {
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
        Box::new({
            let name = name.to_owned();
            move |el| {
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

    fn event_target<T>(ev: &Self::Event) -> T
    where
        T: CastFrom<Self::Element>,
    {
        let el = ev
            .unchecked_ref::<Event>()
            .target()
            .expect("event.target not found")
            .unchecked_into::<Element>();
        T::cast_from(el).expect("incorrect element type")
    }

    fn add_event_listener_delegated(
        el: &Self::Element,
        name: Cow<'static, str>,
        delegation_key: Cow<'static, str>,
        cb: Box<dyn FnMut(Self::Event)>,
    ) -> Box<dyn FnOnce(&Self::Element)> {
        let cb = Closure::wrap(cb).into_js_value();
        let key = intern(&delegation_key);
        or_debug!(
            js_sys::Reflect::set(el, &JsValue::from_str(key), &cb),
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
        Box::new(move |el| {
            or_debug!(
                js_sys::Reflect::delete_property(
                    el,
                    &JsValue::from_str(intern(&delegation_key))
                ),
                el,
                "delete property"
            );
        })
    }

    fn class_list(el: &Self::Element) -> Self::ClassList {
        el.class_list()
    }

    fn add_class(list: &Self::ClassList, name: &str) {
        or_debug!(list.add_1(intern(name)), list.unchecked_ref(), "add()");
    }

    fn remove_class(list: &Self::ClassList, name: &str) {
        or_debug!(
            list.remove_1(intern(name)),
            list.unchecked_ref(),
            "remove()"
        );
    }

    fn style(el: &Self::Element) -> Self::CssStyleDeclaration {
        el.unchecked_ref::<HtmlElement>().style()
    }

    fn set_css_property(
        style: &Self::CssStyleDeclaration,
        name: &str,
        value: &str,
    ) {
        or_debug!(
            style.set_property(intern(name), value),
            style.unchecked_ref(),
            "setProperty"
        );
    }

    fn set_inner_html(el: &Self::Element, html: &str) {
        el.set_inner_html(html);
    }
}

impl Mountable<Dom> for Node {
    fn unmount(&mut self) {
        todo!()
    }

    fn mount(&mut self, parent: &Element, marker: Option<&Node>) {
        Dom::insert_node(parent, self, marker);
    }

    fn insert_before_this(
        &self,
        parent: &<Dom as Renderer>::Element,
        child: &mut dyn Mountable<Dom>,
    ) -> bool {
        child.mount(parent, Some(self));
        true
    }
}

impl Mountable<Dom> for Text {
    fn unmount(&mut self) {
        self.remove();
    }

    fn mount(&mut self, parent: &Element, marker: Option<&Node>) {
        Dom::insert_node(parent, self, marker);
    }

    fn insert_before_this(
        &self,
        parent: &<Dom as Renderer>::Element,
        child: &mut dyn Mountable<Dom>,
    ) -> bool {
        child.mount(parent, Some(self.as_ref()));
        true
    }
}

impl Mountable<Dom> for Comment {
    fn unmount(&mut self) {
        self.remove();
    }

    fn mount(&mut self, parent: &Element, marker: Option<&Node>) {
        Dom::insert_node(parent, self, marker);
    }

    fn insert_before_this(
        &self,
        parent: &<Dom as Renderer>::Element,
        child: &mut dyn Mountable<Dom>,
    ) -> bool {
        child.mount(parent, Some(self.as_ref()));
        true
    }
}

impl Mountable<Dom> for Element {
    fn unmount(&mut self) {
        self.remove();
    }

    fn mount(&mut self, parent: &Element, marker: Option<&Node>) {
        Dom::insert_node(parent, self, marker);
    }

    fn insert_before_this(
        &self,
        parent: &<Dom as Renderer>::Element,
        child: &mut dyn Mountable<Dom>,
    ) -> bool {
        child.mount(parent, Some(self.as_ref()));
        true
    }
}

impl Mountable<Dom> for DocumentFragment {
    fn unmount(&mut self) {
        todo!()
    }

    fn mount(&mut self, parent: &Element, marker: Option<&Node>) {
        Dom::insert_node(parent, self, marker);
    }

    fn insert_before_this(
        &self,
        parent: &<Dom as Renderer>::Element,
        child: &mut dyn Mountable<Dom>,
    ) -> bool {
        child.mount(parent, Some(self.as_ref()));
        true
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
