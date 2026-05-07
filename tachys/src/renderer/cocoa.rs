#![allow(missing_docs)]

//! Cocoa/AppKit-backed implementation of the renderer surface.
//!
//! This is the macOS analogue of [`super::dom`]. It re-exports the
//! types from the `cocoa_dom` crate as the names tachys expects
//! (`Element`, `Node`, `Text`, `Placeholder`, etc.) and adds the
//! [`Mountable`] / [`CastFrom`] trait impls that have to live here for
//! orphan-rule reasons.
//!
//! `Dom` is a unit struct rather than a type alias so we can attach
//! tachys-specific methods (`mount_before`, `try_mount_before`) that
//! depend on the [`Mountable`] trait — orphan rules prevent us from
//! adding those directly to `cocoa_dom::Renderer`.

use super::CastFrom;
use crate::view::Mountable;
use cocoa_dom::{layout::Style, NodeKind, Renderer as CocoaRenderer};

// Type re-exports: the names the rest of tachys expects under
// `crate::renderer::types::*`.
pub use cocoa_dom::{
    ClassList, CssStyleDeclaration, Element, Event, Node, Placeholder,
    TemplateElement, Text,
};

/// The renderer surface used by tachys on macOS.
///
/// Forwards every method to [`cocoa_dom::Renderer`]; adds
/// `mount_before` / `try_mount_before` which need tachys' [`Mountable`]
/// trait in scope.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Dom;

impl Dom {
    pub fn intern(text: &str) -> &str {
        CocoaRenderer::intern(text)
    }

    pub fn create_element(tag: &str, namespace: Option<&str>) -> Element {
        CocoaRenderer::create_element(tag, namespace)
    }

    pub fn create_text_node(text: &str) -> Text {
        CocoaRenderer::create_text_node(text)
    }

    pub fn create_placeholder() -> Placeholder {
        CocoaRenderer::create_placeholder()
    }

    pub fn set_text(node: &Text, text: &str) {
        CocoaRenderer::set_text(node, text);
    }

    pub fn set_attribute(node: &Element, name: &str, value: &str) {
        CocoaRenderer::set_attribute(node, name, value);
    }

    pub fn remove_attribute(node: &Element, name: &str) {
        CocoaRenderer::remove_attribute(node, name);
    }

    pub fn insert_node(
        parent: &Element,
        new_child: &Node,
        anchor: Option<&Node>,
    ) {
        CocoaRenderer::insert_node(parent, new_child, anchor);
    }

    pub fn try_insert_node(
        parent: &Element,
        new_child: &Node,
        anchor: Option<&Node>,
    ) -> bool {
        CocoaRenderer::try_insert_node(parent, new_child, anchor)
    }

    pub fn remove_node(parent: &Element, child: &Node) -> Option<Node> {
        CocoaRenderer::remove_node(parent, child)
    }

    pub fn remove(node: &Node) {
        CocoaRenderer::remove(node);
    }

    pub fn get_parent(node: &Node) -> Option<Node> {
        CocoaRenderer::get_parent(node)
    }

    pub fn first_child(node: &Node) -> Option<Node> {
        CocoaRenderer::first_child(node)
    }

    pub fn next_sibling(node: &Node) -> Option<Node> {
        CocoaRenderer::next_sibling(node)
    }

    pub fn log_node(node: &Node) {
        CocoaRenderer::log_node(node);
    }

    pub fn clear_children(parent: &Element) {
        CocoaRenderer::clear_children(parent);
    }

    pub fn class_list(el: &Element) -> ClassList {
        CocoaRenderer::class_list(el)
    }
    pub fn add_class(list: &ClassList, name: &str) {
        CocoaRenderer::add_class(list, name);
    }
    pub fn remove_class(list: &ClassList, name: &str) {
        CocoaRenderer::remove_class(list, name);
    }

    pub fn style(el: &Element) -> CssStyleDeclaration {
        CocoaRenderer::style(el)
    }
    pub fn set_css_property(
        style: &CssStyleDeclaration,
        name: &str,
        value: &str,
    ) {
        CocoaRenderer::set_css_property(style, name, value);
    }
    pub fn remove_css_property(style: &CssStyleDeclaration, name: &str) {
        CocoaRenderer::remove_css_property(style, name);
    }

    pub fn set_inner_html(el: &Element, html: &str) {
        CocoaRenderer::set_inner_html(el, html);
    }

    pub fn get_template<V: 'static>() -> TemplateElement {
        CocoaRenderer::get_template::<V>()
    }

    pub fn clone_template(tpl: &TemplateElement) -> Element {
        CocoaRenderer::clone_template(tpl)
    }

    /// Mount `new_child` immediately before `before` in `before`'s
    /// parent. Used by dynamic-children diffing in tachys
    /// (`<For>`, keyed iteration, etc.).
    ///
    /// We synthesise a parent `Element` wrapper around `before`'s
    /// `superview()`. The wrapper's `LayoutHandle` is borrowed from
    /// the marker's tree (every node in the same Taffy tree shares
    /// the tree handle, and Taffy can name the parent's NodeId via
    /// `tree.parent(child_id)`) so the new child registers in the
    /// right tree and gets a real layout slot.
    pub fn mount_before<M>(new_child: &mut M, before: &Node)
    where
        M: Mountable,
    {
        let parent_view = unsafe { before.ns_view().superview() }
            .expect("cocoa_dom: mount_before — node has no superview");
        let parent = synthesise_parent_element(parent_view, before);
        new_child.mount(&parent, Some(before));
    }

    pub fn try_mount_before<M>(new_child: &mut M, before: &Node) -> bool
    where
        M: Mountable,
    {
        let superview = unsafe { before.ns_view().superview() };
        let Some(parent_view) = superview else {
            return false;
        };
        let parent = synthesise_parent_element(parent_view, before);
        new_child.mount(&parent, Some(before));
        true
    }
}

/// Build an `Element` wrapper around `parent_view` whose
/// `LayoutHandle` references the same Taffy tree + the parent
/// `NodeId` that `before` lives under. If `before` isn't registered
/// in any tree, the parent wrapper also has no handle — falls back
/// to NSView-only mounting (the new child won't be in any layout
/// tree, but will at least appear as a subview).
fn synthesise_parent_element(
    parent_view: cocoa_dom::Retained<cocoa_dom::NSView>,
    before: &Node,
) -> Element {
    use cocoa_dom::layout::LayoutHandle;

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
// Mountable — wires our DOM-shaped wrappers into the tachys view tree.
// ---------------------------------------------------------------------

impl Mountable for Node {
    fn unmount(&mut self) {
        // Teardown drops the Taffy node + handler-store entry, then
        // removes the NSView from its superview.
        self.teardown();
    }

    fn mount(&mut self, parent: &Element, marker: Option<&Node>) {
        Dom::insert_node(parent, self, marker);
    }

    fn try_mount(&mut self, parent: &Element, marker: Option<&Node>) -> bool {
        Dom::try_insert_node(parent, self, marker)
    }

    fn insert_before_this(&self, _child: &mut dyn Mountable) -> bool {
        // Used by hydration / dynamic-children diffing to splice a new
        // node in just before this one. On native we don't have a good
        // way back from a raw NSView to a typed parent Element, so this
        // is unimplemented for now (see implementation_log.md).
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
// CastFrom — used by hydration (stubbed) and event_target casts.
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

// Identity cast on Element. The web target has a JsCast-bounded blanket
// impl `CastFrom<Element> for T: JsCast`; on native there's no JsCast
// equivalent and the only callers live in the html module (currently
// disabled — see implementation_log.md).
impl CastFrom<Element> for Element {
    fn cast_from(source: Element) -> Option<Element> {
        Some(source)
    }
}
