#![deny(missing_docs)]

//! The DOM implementation for `leptos`.

mod components;
mod html;

pub use components::*;
pub use html::*;
use leptos_reactive::Scope;
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use wasm_bindgen::JsCast;

/// Converts the value into a [`Node`].
pub trait IntoNode {
    /// Converts the value into [`Node`].
    fn into_node(self, cx: Scope) -> Node;
}

impl IntoNode for () {
    fn into_node(self, cx: Scope) -> Node {
        Unit.into_node(cx)
    }
}

cfg_if::cfg_if! {
    if #[cfg(all(target_arch = "wasm32", feature = "web"))] {
        #[derive(Clone, educe::Educe)]
        #[educe(Deref)]
        // Be careful not to drop this until you want to unmount
        // the node from the DOM. The easiest way to accidentally do
        // this is by cloning `Comment` and letting it go out of scope.
        // Too bad there's no lint for this...
        struct WebSysNode(web_sys::Node);

        impl Drop for WebSysNode {
            fn drop(&mut self) {
                self.0.unchecked_ref::<web_sys::Element>().remove();
            }
        }

        impl From<web_sys::Node> for WebSysNode {
            fn from(node: web_sys::Node) -> Self {
                Self(node)
            }
        }
    } else {
        #[derive(Clone)]
        struct WebSysNode();
    }
}

/// HTML element.
pub struct Element {
    _name: String,
    is_void: bool,
    node: WebSysNode,
    attributes: HashMap<String, String>,
    children: Vec<Node>,
}

impl Element {
    #[track_caller]
    fn new<El: IntoElement>(el: El) -> Self {
        let name = el.name();

        let node = 'label: {
            #[cfg(all(target_arch = "wasm32", feature = "web"))]
            break 'label gloo::utils::document()
                .create_element(&name)
                .expect("element creation to not fail")
                .unchecked_into::<web_sys::Node>()
                .into();

            #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
            break 'label WebSysNode();
        };

        Self {
            _name: name,
            is_void: el.is_void(),
            node,
            attributes: Default::default(),
            children: Default::default(),
        }
    }
}

#[derive(Clone)]
struct Comment {
    node: WebSysNode,
    content: String,
}

impl Comment {
    fn new(content: &str) -> Self {
        let content = content.to_owned();

        let node = 'label: {
            #[cfg(all(target_arch = "wasm32", feature = "web"))]
            break 'label gloo::utils::document()
                .create_comment(&format!(" {content} "))
                .unchecked_into::<web_sys::Node>()
                .into();

            #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
            break 'label WebSysNode();
        };

        Self { node, content }
    }
}

/// HTML text
pub struct Text {
    node: WebSysNode,
    content: String,
}

impl Text {
    /// Creates a new [`Text`].
    pub fn new(content: &str) -> Self {
        let content = content.to_owned();

        let node = 'label: {
            #[cfg(all(target_arch = "wasm32", feature = "web"))]
            break 'label gloo::utils::document()
                .create_text_node(&content)
                .unchecked_into::<web_sys::Node>()
                .into();

            #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
            break 'label WebSysNode();
        };

        Self { content, node }
    }
}

/// Custom leptos component.
pub struct Component {
    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    document_fragment: web_sys::DocumentFragment,
    name: String,
    opening: Comment,
    children: Rc<RefCell<Vec<Node>>>,
    closing: Comment,
}

impl Component {
    /// Creates a new [`Component`].
    pub fn new(name: &str) -> Self {
        let name = name.to_owned();

        #[cfg(all(target_arch = "wasm32", feature = "web"))]
        let document_fragment = gloo::utils::document().create_document_fragment();

        Self {
            #[cfg(all(target_arch = "wasm32", feature = "web"))]
            document_fragment,
            opening: Comment::new(&format!("<{name}>")),
            closing: Comment::new(&format!("</{name}>")),
            name,
            children: Default::default(),
        }
    }
}

/// A leptos Node.
pub enum Node {
    /// HTML element node.
    Element(Element),
    /// HTML text node.
    Text(Text),
    /// Custom leptos component.
    Component(Component),
}

impl IntoNode for Node {
    fn into_node(self, _: Scope) -> Node {
        self
    }
}

impl Node {
    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    fn get_web_sys_node(&self) -> web_sys::Node {
        match self {
            Self::Element(node) => node.node.0.clone(),
            Self::Text(t) => t.node.0.clone(),
            Self::Component(c) => c
                .document_fragment
                .clone()
                .unchecked_into::<web_sys::Node>(),
        }
    }
}

#[track_caller]
fn mount_child(kind: MountKind, child: &Node) {
    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    {
        let child = child.get_web_sys_node();

        match kind {
            MountKind::Component(closing) => {
                closing
                    .node
                    .0
                    .unchecked_ref::<web_sys::Element>()
                    .before_with_node_1(&child)
                    .expect("before to not err");
            }
            MountKind::Element(el) => {
                el.0.append_child(&child)
                    .expect("append operation to not err");
            }
        }

        todo!()
    }
}

enum MountKind<'a> {
    Component(
        // The closing node
        &'a Comment,
    ),
    Element(&'a WebSysNode),
}
