#![allow(missing_docs)] // Allow missing docs for experimental backend

use super::{CastFrom, DomRenderer, RemoveEventHandler, Renderer};
use crate::{
    dom::window,
    view::{Mountable, ToTemplate},
};
use linear_map::LinearMap;
use rustc_hash::FxHashSet;
use sledgehammer_bindgen::bindgen;
use std::{
    any::TypeId,
    borrow::Cow,
    cell::{Cell, RefCell},
    rc::Rc,
};
use wasm_bindgen::{
    prelude::{wasm_bindgen, Closure},
    JsCast, JsValue,
};
use web_sys::Node;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen]
    fn queueMicrotask(closure: &Closure<dyn Fn() -> ()>);

    type Global;
}

#[bindgen]
mod js {
    //#[extends(NodeInterpreter)]
    struct Channel;

    const JS: &str = r#"
        function Queue() {
            var head, tail;
            return Object.freeze({     
                enqueue(value) { 
                    const link = {value, next: undefined};
                    tail = head ? tail.next = link : head = link;
                },
                dequeue() {
                    if (head) {
                        const value = head.value;
                        head = head.next;
                        return value;
                    }
                },
                peek() { return head?.value }
            });
        }
        this.nodes = [null];
        this.jsvalues = Queue();
    "#;

    fn drop_node(id: u32) {
        "this.nodes[$id$]=null;"
    }

    fn store_body(id: u32) {
        "this.nodes[$id$]=document.body;"
    }

    fn create_text_node(id: u32, data: &str) {
        "this.nodes[$id$]=document.createTextNode($data$);"
    }

    fn create_comment(id: u32) {
        "this.nodes[$id$]=document.createComment();"
    }

    fn create_element(id: u32, name: &'static str<u8, name_cache>) {
        "this.nodes[$id$]=document.createElement($name$);"
    }

    fn set_attribute(
        id: u32,
        name: &str<u8, name_cache>,
        val: impl Writable<u8>,
    ) {
        "this.nodes[$id$].setAttribute($name$,$val$);"
    }

    fn remove_child(parent: u32, child: u32) {
        "this.nodes[$parent$].removeChild(this.nodes[$child$]);"
    }

    fn remove_attribute(id: u32, name: &str<u8, name_cache>) {
        "this.nodes[$id$].removeAttribute($name$);"
    }

    fn append_child(id: u32, id2: u32) {
        "this.nodes[$id$].appendChild(nodes[$id2$]);"
    }

    fn insert_before(parent: u32, child: u32, marker: u32) {
        "this.nodes[$parent$].insertBefore(this.nodes[$child$],this.\
         nodes[$marker$]);"
    }

    fn set_text(id: u32, text: impl Writable<u8>) {
        "this.nodes[$id$].textContent=$text$;"
    }

    fn remove(id: u32) {
        "this.nodes[$id$].remove();"
    }

    fn replace(id: u32, id2: u32) {
        "this.nodes[$id$].replaceWith(this.nodes[$id2$]);"
    }

    fn first_child(parent: u32, child: u32) {
        "this.nodes[$child$]=this.nodes[$parent$].firstChild;"
    }

    fn next_sibling(anchor: u32, sibling: u32) {
        "this.nodes[$sibling$]=this.nodes[$anchor$].nextSibling;"
    }

    fn class_list(el: u32, class_list: u32) {
        "this.nodes[$class_list$]=this.nodes[$el$].classList;"
    }

    fn add_class(class_list: u32, name: &str<u8, class_cache>) {
        "this.nodes[$class_list$].add($name$);"
    }

    fn remove_class(class_list: u32, name: &str<u8, class_cache>) {
        "this.nodes[$class_list$].remove($name$);"
    }

    fn set_inner_html(node: u32, html: &str) {
        "this.nodes[$node$].innerHTML = $html$;"
    }

    fn clone_template(tpl_node: u32, into_node: u32) {
        "this.nodes[$into_node$]=this.nodes[$tpl_node$].content.\
         cloneNode(true);"
    }

    fn set_property(node: u32, name: &str<u8, name_cache>) {
        "{let jsv=this.jsvalues.dequeue();this.nodes[$node$][$name$]=jsv;}"
    }

    fn add_listener(node: u32, name: &str<u8, name_cache>) {
        "{let jsv=this.jsvalues.dequeue();this.nodes[$node$].\
         addEventListener($name$, jsv);}"
    }
}

#[wasm_bindgen(inline_js = "
    export function get_node(channel, id){
        return channel.nodes[id];
    }

    export function store_node(channel, id, node){
        channel.nodes[id] = node;
    }

    export function store_jsvalue(channel, value) {
        channel.jsvalues.enqueue(value);
    }
")]
extern "C" {
    fn get_node(channel: &JSChannel, id: u32) -> Node;

    fn store_node(channel: &JSChannel, id: u32, node: Node);

    fn store_jsvalue(channel: &JSChannel, value: JsValue);
}

#[derive(Debug)]
pub struct Sledgehammer;

impl Sledgehammer {
    pub fn body() -> SNode {
        let node = SNode::new();
        with(|channel| channel.store_body(node.0 .0));
        node
    }

    pub fn store(node: Node) -> SNode {
        let snode = SNode::new();
        with(|channel| store_node(channel.js_channel(), snode.0 .0, node));
        snode
    }

    pub fn element(tag_name: &'static str) -> SNode {
        let node = SNode::new();
        with(|channel| channel.create_element(node.0 .0, tag_name));
        node
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SNode(Rc<SNodeInner>);

#[derive(Debug, PartialEq, Eq, Hash)]
struct SNodeInner(u32);

impl SNode {
    fn new() -> Self {
        let id = if let Some(id) = RECYCLE_IDS.with_borrow_mut(Vec::pop) {
            id
        } else {
            let new_id = NEXT_ID.get();
            NEXT_ID.set(new_id + 1);
            new_id
        };
        Self(Rc::new(SNodeInner(id)))
    }

    pub fn to_node(&self) -> Node {
        CHANNEL.with_borrow(|channel| get_node(channel.js_channel(), self.0 .0))
    }
}

impl Drop for SNodeInner {
    fn drop(&mut self) {
        RECYCLE_IDS.with_borrow_mut(|ids| ids.push(self.0));
        with(|channel| channel.drop_node(self.0));
    }
}

impl AsRef<SNode> for SNode {
    fn as_ref(&self) -> &SNode {
        self
    }
}

impl CastFrom<SNode> for SNode {
    fn cast_from(source: SNode) -> Option<Self> {
        Some(source)
    }
}

thread_local! {
    static CHANNEL: RefCell<Channel> = RefCell::new(Channel::default());
    static FLUSH_PENDING: Cell<bool> = const { Cell::new(false) };
    static FLUSH_CLOSURE: Closure<dyn Fn()> = Closure::new(|| {
        FLUSH_PENDING.set(false);
        CHANNEL.with_borrow_mut(|channel| {
            channel.flush();
        });
    });
    static NEXT_ID: Cell<u32> = const { Cell::new(1) };
    static RECYCLE_IDS: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };

    pub(crate) static GLOBAL_EVENTS: RefCell<FxHashSet<Cow<'static, str>>> = Default::default();
}

fn with(fun: impl FnOnce(&mut Channel)) {
    CHANNEL.with_borrow_mut(fun);
    flush();
}

#[allow(unused)] // might be handy at some point!
fn flush_sync() {
    FLUSH_PENDING.set(false);
    CHANNEL.with_borrow_mut(|channel| channel.flush());
}

fn flush() {
    let was_pending = FLUSH_PENDING.replace(true);
    if !was_pending {
        FLUSH_CLOSURE.with(queueMicrotask);
    }
}

impl Renderer for Sledgehammer {
    type Node = SNode;
    type Text = SNode;
    type Element = SNode;
    type Placeholder = SNode;

    fn intern(text: &str) -> &str {
        text
    }

    fn create_text_node(text: &str) -> Self::Text {
        let node = SNode::new();
        with(|channel| channel.create_text_node(node.0 .0, text));
        node
    }

    fn create_placeholder() -> Self::Placeholder {
        let node = SNode::new();
        with(|channel| channel.create_comment(node.0 .0));
        node
    }

    fn set_text(node: &Self::Text, text: &str) {
        with(|channel| channel.set_text(node.0 .0, text));
    }

    fn set_attribute(node: &Self::Element, name: &str, value: &str) {
        with(|channel| channel.set_attribute(node.0 .0, name, value));
    }

    fn remove_attribute(node: &Self::Element, name: &str) {
        with(|channel| channel.remove_attribute(node.0 .0, name));
    }

    fn insert_node(
        parent: &Self::Element,
        new_child: &Self::Node,
        anchor: Option<&Self::Node>,
    ) {
        with(|channel| {
            channel.insert_before(
                parent.0 .0,
                new_child.0 .0,
                anchor.map(|n| n.0 .0).unwrap_or(0),
            )
        });
    }

    fn remove_node(
        parent: &Self::Element,
        child: &Self::Node,
    ) -> Option<Self::Node> {
        with(|channel| channel.remove_child(parent.0 .0, child.0 .0));
        Some(child.clone())
    }

    fn remove(node: &Self::Node) {
        with(|channel| channel.remove(node.0 .0));
    }

    fn get_parent(_node: &Self::Node) -> Option<Self::Node> {
        todo!() // node.parent_node()
    }

    fn first_child(node: &Self::Node) -> Option<Self::Node> {
        let child = SNode::new();
        with(|channel| channel.first_child(node.0 .0, child.0 .0));
        Some(child)
    }

    fn next_sibling(node: &Self::Node) -> Option<Self::Node> {
        let sibling = SNode::new();
        with(|channel| channel.next_sibling(node.0 .0, sibling.0 .0));
        Some(sibling)
    }

    fn log_node(_node: &Self::Node) {
        todo!()
    }

    fn clear_children(parent: &Self::Element) {
        with(|channel| channel.set_text(parent.0 .0, ""));
    }
}

#[derive(Debug, Clone)]
pub struct ClassList(SNode);

#[derive(Debug, Clone)]
#[allow(dead_code)] // this will be used, it's just all unimplemented
pub struct CssStyle(SNode);

impl DomRenderer for Sledgehammer {
    type Event = JsValue;
    type ClassList = ClassList;
    type CssStyleDeclaration = CssStyle;
    type TemplateElement = SNode;

    fn set_property(_el: &Self::Element, _key: &str, _value: &JsValue) {
        todo!()
    }

    fn add_event_listener(
        el: &Self::Element,
        name: &str,
        cb: Box<dyn FnMut(Self::Event)>,
    ) -> RemoveEventHandler<Self::Element> {
        let cb = wasm_bindgen::closure::Closure::wrap(cb).into_js_value();
        CHANNEL.with_borrow_mut(|channel| {
            channel.add_listener(el.0 .0, name);
            let channel = channel.js_channel();
            store_jsvalue(channel, cb);
        });

        // return the remover
        RemoveEventHandler(Box::new(move |_el| todo!()))
    }

    fn event_target<T>(_ev: &Self::Event) -> T
    where
        T: CastFrom<Self::Element>,
    {
        todo!()
        /*let el = ev
            .unchecked_ref::<Event>()
            .target()
            .expect("event.target not found")
            .unchecked_into::<Element>();
        T::cast_from(el).expect("incorrect element type")*/
    }

    fn add_event_listener_delegated(
        el: &Self::Element,
        name: Cow<'static, str>,
        delegation_key: Cow<'static, str>,
        cb: Box<dyn FnMut(Self::Event)>,
    ) -> RemoveEventHandler<Self::Element> {
        let cb = Closure::wrap(cb).into_js_value();
        CHANNEL.with_borrow_mut(|channel| {
            channel.set_property(el.0 .0, &delegation_key);
            let channel = channel.js_channel();
            store_jsvalue(channel, cb);
        });

        GLOBAL_EVENTS.with(|global_events| {
            let mut events = global_events.borrow_mut();
            if !events.contains(&name) {
                // create global handler
                let key = JsValue::from_str(&delegation_key);
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
        RemoveEventHandler(Box::new(move |_el| todo!()))
    }

    fn class_list(el: &Self::Element) -> Self::ClassList {
        let class_list = SNode::new();
        with(|channel| channel.class_list(el.0 .0, class_list.0 .0));
        ClassList(class_list)
    }

    fn add_class(list: &Self::ClassList, name: &str) {
        with(|channel| channel.add_class(list.0 .0 .0, name));
    }

    fn remove_class(list: &Self::ClassList, name: &str) {
        with(|channel| channel.remove_class(list.0 .0 .0, name));
    }

    fn style(_el: &Self::Element) -> Self::CssStyleDeclaration {
        todo!()
        //el.unchecked_ref::<HtmlElement>().style()
    }

    fn set_css_property(
        _style: &Self::CssStyleDeclaration,
        _name: &str,
        _value: &str,
    ) {
        todo!()
        /*or_debug!(
            style.set_property(name, value),
            style.unchecked_ref(),
            "setProperty"
        );*/
    }

    fn set_inner_html(el: &Self::Element, html: &str) {
        with(|channel| channel.set_inner_html(el.0 .0, html))
    }

    fn get_template<V>() -> Self::TemplateElement
    where
        V: ToTemplate + 'static,
    {
        thread_local! {
            static TEMPLATES: RefCell<LinearMap<TypeId, SNode>> = Default::default();
        }

        TEMPLATES.with(|t| {
            t.borrow_mut()
                .entry(TypeId::of::<V>())
                .or_insert_with(|| {
                    let mut buf = String::new();
                    V::to_template(
                        &mut buf,
                        &mut String::new(),
                        &mut String::new(),
                        &mut String::new(),
                        &mut Default::default(),
                    );
                    let node = SNode::new();
                    with(|channel| {
                        channel.create_element(node.0 .0, "template");
                        channel.set_inner_html(node.0 .0, &buf)
                    });
                    node
                })
                .clone()
        })
    }

    fn clone_template(tpl: &Self::TemplateElement) -> Self::Element {
        let node = SNode::new();
        with(|channel| {
            channel.clone_template(tpl.0 .0, node.0 .0);
        });
        node
    }

    fn create_element_from_html(_html: &str) -> Self::Element {
        todo!()
    }
}

impl Mountable<Sledgehammer> for SNode {
    fn unmount(&mut self) {
        with(|channel| channel.remove(self.0 .0));
    }

    fn mount(&mut self, parent: &SNode, marker: Option<&SNode>) {
        Sledgehammer::insert_node(parent, self, marker);
    }

    fn insert_before_this(
        &self,
        _child: &mut dyn Mountable<Sledgehammer>,
    ) -> bool {
        todo!()
    }
}
