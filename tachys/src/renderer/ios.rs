#![allow(missing_docs)]

//! UIKit-backed implementation of the renderer surface.
//!
//! This is the iOS analogue of [`super::dom`] and [`super::cocoa`].
//! It re-exports the types from the `ios_dom` crate as the names
//! tachys expects and adds the [`Mountable`] / [`CastFrom`] trait
//! impls that have to live here for orphan-rule reasons.

use super::CastFrom;
use crate::view::Mountable;
use ios_dom::{layout::Style, NodeKind, Renderer as IosRenderer};

// Type re-exports: the names the rest of tachys expects under
// `crate::renderer::types::*`.
pub use ios_dom::{
    ClassList, CssStyleDeclaration, Element, Event, Node, Placeholder,
    TemplateElement, Text,
};

/// The renderer surface used by tachys on iOS.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Dom;

impl Dom {
    pub fn intern(text: &str) -> &str {
        IosRenderer::intern(text)
    }

    pub fn create_element(tag: &str, namespace: Option<&str>) -> Element {
        IosRenderer::create_element(tag, namespace)
    }

    pub fn create_text_node(text: &str) -> Text {
        IosRenderer::create_text_node(text)
    }

    pub fn create_placeholder() -> Placeholder {
        IosRenderer::create_placeholder()
    }

    pub fn set_text(node: &Text, text: &str) {
        IosRenderer::set_text(node, text);
    }

    pub fn set_attribute(node: &Element, name: &str, value: &str) {
        IosRenderer::set_attribute(node, name, value);
    }

    pub fn remove_attribute(node: &Element, name: &str) {
        IosRenderer::remove_attribute(node, name);
    }

    pub fn insert_node(
        parent: &Element,
        new_child: &Node,
        anchor: Option<&Node>,
    ) {
        IosRenderer::insert_node(parent, new_child, anchor);
    }

    pub fn try_insert_node(
        parent: &Element,
        new_child: &Node,
        anchor: Option<&Node>,
    ) -> bool {
        IosRenderer::try_insert_node(parent, new_child, anchor)
    }

    pub fn remove_node(parent: &Element, child: &Node) -> Option<Node> {
        IosRenderer::remove_node(parent, child)
    }

    pub fn remove(node: &Node) {
        IosRenderer::remove(node);
    }

    pub fn get_parent(node: &Node) -> Option<Node> {
        IosRenderer::get_parent(node)
    }

    pub fn first_child(node: &Node) -> Option<Node> {
        IosRenderer::first_child(node)
    }

    pub fn next_sibling(node: &Node) -> Option<Node> {
        IosRenderer::next_sibling(node)
    }

    pub fn log_node(node: &Node) {
        IosRenderer::log_node(node);
    }

    pub fn clear_children(parent: &Element) {
        IosRenderer::clear_children(parent);
    }

    pub fn class_list(el: &Element) -> ClassList {
        IosRenderer::class_list(el)
    }
    pub fn add_class(list: &ClassList, name: &str) {
        IosRenderer::add_class(list, name);
    }
    pub fn remove_class(list: &ClassList, name: &str) {
        IosRenderer::remove_class(list, name);
    }

    pub fn style(el: &Element) -> CssStyleDeclaration {
        IosRenderer::style(el)
    }
    pub fn set_css_property(
        style: &CssStyleDeclaration,
        name: &str,
        value: &str,
    ) {
        IosRenderer::set_css_property(style, name, value);
    }
    pub fn remove_css_property(
        style: &CssStyleDeclaration,
        name: &str,
    ) {
        IosRenderer::remove_css_property(style, name);
    }

    pub fn set_inner_html(el: &Element, html: &str) {
        IosRenderer::set_inner_html(el, html);
    }

    pub fn get_template<V: 'static>() -> TemplateElement {
        IosRenderer::get_template::<V>()
    }

    pub fn clone_template(tpl: &TemplateElement) -> Element {
        IosRenderer::clone_template(tpl)
    }

    pub fn mount_before<M>(new_child: &mut M, before: &Node)
    where
        M: Mountable,
    {
        let parent_view = before
            .ui_view()
            .superview()
            .expect("ios_dom: mount_before — node has no superview");
        let parent = synthesise_parent_element(parent_view, before);
        new_child.mount(&parent, Some(before));
    }

    pub fn try_mount_before<M>(new_child: &mut M, before: &Node) -> bool
    where
        M: Mountable,
    {
        let Some(parent_view) = before.ui_view().superview() else {
            return false;
        };
        let parent = synthesise_parent_element(parent_view, before);
        new_child.mount(&parent, Some(before));
        true
    }
}

fn synthesise_parent_element(
    parent_view: ios_dom::Retained<ios_dom::UIView>,
    before: &Node,
) -> Element {
    use ios_dom::layout::LayoutHandle;

    let parent_handle: Option<LayoutHandle> = {
        let layout = before.layout_slot().borrow();
        layout.handle.as_ref().and_then(|h| {
            let parent_id = h.tree.tree.borrow().parent(h.node_id)?;
            Some(LayoutHandle {
                tree: h.tree.clone(),
                node_id: parent_id,
            })
        })
    };

    let parent_node = match parent_handle {
        Some(handle) => Node::from_view_with_handle(
            parent_view,
            NodeKind::Element,
            handle,
        ),
        None => Node::from_view(
            parent_view,
            NodeKind::Element,
            Style::default(),
        ),
    };
    Element::from_node_unchecked(parent_node)
}

// ---------------------------------------------------------------------
// Mountable
// ---------------------------------------------------------------------

impl Mountable for Node {
    fn unmount(&mut self) {
        self.teardown();
    }

    fn mount(&mut self, parent: &Element, marker: Option<&Node>) {
        Dom::insert_node(parent, self, marker);
    }

    fn try_mount(&mut self, parent: &Element, marker: Option<&Node>) -> bool {
        Dom::try_insert_node(parent, self, marker)
    }

    fn insert_before_this(&self, _child: &mut dyn Mountable) -> bool {
        false
    }

    fn elements(&self) -> Vec<Element> {
        Vec::new()
    }
}

impl Mountable for Element {
    fn unmount(&mut self) {
        self.as_node().teardown();
    }

    fn mount(&mut self, parent: &Element, marker: Option<&Node>) {
        Dom::insert_node(parent, self.as_node(), marker);
    }

    fn insert_before_this(&self, _child: &mut dyn Mountable) -> bool {
        false
    }

    fn elements(&self) -> Vec<Element> {
        vec![self.clone()]
    }
}

impl Mountable for Text {
    fn unmount(&mut self) {
        self.as_node().teardown();
    }

    fn mount(&mut self, parent: &Element, marker: Option<&Node>) {
        Dom::insert_node(parent, self.as_node(), marker);
    }

    fn insert_before_this(&self, _child: &mut dyn Mountable) -> bool {
        false
    }

    fn elements(&self) -> Vec<Element> {
        Vec::new()
    }
}

impl Mountable for Placeholder {
    fn unmount(&mut self) {
        self.as_node().teardown();
    }

    fn mount(&mut self, parent: &Element, marker: Option<&Node>) {
        Dom::insert_node(parent, self.as_node(), marker);
    }

    fn insert_before_this(&self, _child: &mut dyn Mountable) -> bool {
        false
    }

    fn elements(&self) -> Vec<Element> {
        Vec::new()
    }
}

// ---------------------------------------------------------------------
// CastFrom
// ---------------------------------------------------------------------

impl CastFrom<Node> for Element {
    fn cast_from(node: Node) -> Option<Element> {
        match node.kind() {
            NodeKind::Element => Some(Element::from_node_unchecked(node)),
            _ => None,
        }
    }
}

impl CastFrom<Node> for Text {
    fn cast_from(node: Node) -> Option<Text> {
        match node.kind() {
            NodeKind::Text => Some(Text::from_node_unchecked(node)),
            _ => None,
        }
    }
}

impl CastFrom<Node> for Placeholder {
    fn cast_from(node: Node) -> Option<Placeholder> {
        match node.kind() {
            NodeKind::Placeholder => {
                Some(Placeholder::from_node_unchecked(node))
            }
            _ => None,
        }
    }
}

impl CastFrom<Element> for Element {
    fn cast_from(source: Element) -> Option<Element> {
        Some(source)
    }
}
