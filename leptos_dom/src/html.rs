use crate::{
    components::{DynChild, DynText},
    mount_child, Element, IntoNode, Node, Text,
};
use leptos_reactive::Scope;

/// Trait which allows creating an element tag.
pub trait IntoElement {
    /// The name of the element, i.e., `div`, `p`, `custom-element`.
    fn name(&self) -> String;

    /// Determains if the tag is void, i.e., `<input>` and `<br>`.
    fn is_void(&self) -> bool {
        false
    }
}

/// Represents potentially any element, which you can change
/// at any time before calling [`HtmlElement::into_node`].
pub struct AnyElement {
    name: String,
    is_void: bool,
}

impl IntoElement for AnyElement {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn is_void(&self) -> bool {
        self.is_void
    }
}

/// Represents an HTML `<div>` element.
pub struct Div;

/// Represents a custom HTML element, such as `<my-element>`.
pub struct Custom {
    name: String,
}

impl IntoElement for Custom {
    fn name(&self) -> String {
        self.name.clone()
    }
}

impl IntoElement for Div {
    fn name(&self) -> String {
        "div".into()
    }
}

/// Represents an HTML element.
pub struct HtmlElement<El: IntoElement> {
    cx: Scope,
    element: El,
    children: Vec<Node>,
}

impl<El: IntoElement> HtmlElement<El> {
    fn new(cx: Scope, element: El) -> Self {
        Self {
            cx,
            children: vec![],
            element,
        }
    }

    /// Converts this element into [`HtmlElement<AnyElement>`].
    pub fn into_any(self) -> HtmlElement<AnyElement> {
        let Self {
            cx,
            children,
            element,
        } = self;

        HtmlElement {
            cx,
            children,
            element: AnyElement {
                name: element.name(),
                is_void: element.is_void(),
            },
        }
    }

    /// Inserts a child into this element.
    pub fn child<C: IntoNode>(mut self, child: C) -> Self {
        self.children.push(child.into_node(self.cx));

        self
    }

    /// Creates a child which will automatically re-render when
    /// it's signal dependencies change.
    pub fn dyn_child<CF, N>(mut self, child_fn: CF) -> Self
    where
        CF: Fn(Scope) -> N + 'static,
        N: IntoNode,
    {
        self.children
            .push(DynChild::new(self.cx, child_fn).into_node(self.cx));

        self
    }

    /// Creates a text node on this element.
    pub fn text(mut self, text: impl ToString) -> Self {
        let text = Text::new(&text.to_string());

        let node = Node::Text(text);

        self.children.push(node);

        self
    }

    /// Creates text which will automatically re-render when
    /// it's signal dependencies change.
    pub fn dyn_text<TF, Txt>(mut self, text_fn: TF) -> Self
    where
        TF: Fn(Scope) -> Txt + 'static,
        Txt: ToString,
    {
        let dyn_text = DynText::new(self.cx, text_fn);

        self.children.push(dyn_text.into_node(self.cx));

        self
    }
}

impl<El: IntoElement> IntoNode for HtmlElement<El> {
    fn into_node(self, _: Scope) -> Node {
        let Self {
            cx: _,
            element,
            children,
        } = self;

        let mut element = Element::new(element);

        for child in &children {
            mount_child(crate::MountKind::Element(&element.node), child);
        }

        element.children.extend(children);

        Node::Element(element)
    }
}

/// Creates an HTML `<div>` element.
pub fn div(cx: Scope) -> HtmlElement<Div> {
    HtmlElement::new(cx, Div)
}

/// Creates any custom element, such as `<my-element>`.
pub fn custom<El: IntoElement>(cx: Scope, name: &str) -> HtmlElement<Custom> {
    HtmlElement::new(
        cx,
        Custom {
            name: name.to_owned(),
        },
    )
}
