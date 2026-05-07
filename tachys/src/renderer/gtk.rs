#![allow(missing_docs)]

//! GTK4-backed implementation of the renderer surface.
//!
//! This is the Linux analogue of [`super::dom`] (web) and
//! [`super::cocoa`] (macOS). It re-exports the types from the
//! `gtk_dom` crate as the names tachys expects (`Element`, `Node`,
//! `Text`, `Placeholder`, etc.) and adds the [`Mountable`] /
//! [`CastFrom`] trait impls that have to live here for orphan-rule
//! reasons.
//!
//! `Dom` is a unit struct rather than a type alias so we can attach
//! tachys-specific methods (`mount_before`, `try_mount_before`) that
//! depend on the [`Mountable`] trait — orphan rules prevent us from
//! adding those directly to `gtk_dom::Renderer`.
//!
//! Compared with the macOS renderer in [`super::cocoa`], this module
//! is much shorter: there is no Taffy tree to register against, so
//! `mount_before` is a straightforward `widget.parent()` lookup
//! followed by an ordinary `mount` call. None of cocoa's
//! `synthesise_parent_element` + `LayoutHandle` propagation is
//! required.

use super::CastFrom;
use crate::view::Mountable;
use gtk_dom::{NodeKind, Renderer as GtkRenderer};

// Type re-exports: the names the rest of tachys expects under
// `crate::renderer::types::*`.
pub use gtk_dom::{
    ClassList, CssStyleDeclaration, Element, Event, Node, Placeholder,
    TemplateElement, Text,
};

/// The renderer surface used by tachys on Linux.
///
/// Forwards every method to [`gtk_dom::Renderer`]; adds `mount_before`
/// / `try_mount_before` which need tachys' [`Mountable`] trait in
/// scope.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Dom;

impl Dom {
    pub fn intern(text: &str) -> &str {
        GtkRenderer::intern(text)
    }

    pub fn create_element(tag: &str, namespace: Option<&str>) -> Element {
        GtkRenderer::create_element(tag, namespace)
    }

    pub fn create_text_node(text: &str) -> Text {
        GtkRenderer::create_text_node(text)
    }

    pub fn create_placeholder() -> Placeholder {
        GtkRenderer::create_placeholder()
    }

    pub fn set_text(node: &Text, text: &str) {
        GtkRenderer::set_text(node, text);
    }

    pub fn set_attribute(node: &Element, name: &str, value: &str) {
        GtkRenderer::set_attribute(node, name, value);
    }

    pub fn remove_attribute(node: &Element, name: &str) {
        GtkRenderer::remove_attribute(node, name);
    }

    pub fn insert_node(
        parent: &Element,
        new_child: &Node,
        anchor: Option<&Node>,
    ) {
        GtkRenderer::insert_node(parent, new_child, anchor);
    }

    pub fn try_insert_node(
        parent: &Element,
        new_child: &Node,
        anchor: Option<&Node>,
    ) -> bool {
        GtkRenderer::try_insert_node(parent, new_child, anchor)
    }

    pub fn remove_node(parent: &Element, child: &Node) -> Option<Node> {
        GtkRenderer::remove_node(parent, child)
    }

    pub fn remove(node: &Node) {
        GtkRenderer::remove(node);
    }

    pub fn get_parent(node: &Node) -> Option<Node> {
        GtkRenderer::get_parent(node)
    }

    pub fn first_child(node: &Node) -> Option<Node> {
        GtkRenderer::first_child(node)
    }

    pub fn next_sibling(node: &Node) -> Option<Node> {
        GtkRenderer::next_sibling(node)
    }

    pub fn log_node(node: &Node) {
        GtkRenderer::log_node(node);
    }

    pub fn clear_children(parent: &Element) {
        GtkRenderer::clear_children(parent);
    }

    pub fn class_list(el: &Element) -> ClassList {
        GtkRenderer::class_list(el)
    }
    pub fn add_class(list: &ClassList, name: &str) {
        GtkRenderer::add_class(list, name);
    }
    pub fn remove_class(list: &ClassList, name: &str) {
        GtkRenderer::remove_class(list, name);
    }

    pub fn style(el: &Element) -> CssStyleDeclaration {
        GtkRenderer::style(el)
    }
    pub fn set_css_property(
        style: &CssStyleDeclaration,
        name: &str,
        value: &str,
    ) {
        GtkRenderer::set_css_property(style, name, value);
    }
    pub fn remove_css_property(style: &CssStyleDeclaration, name: &str) {
        GtkRenderer::remove_css_property(style, name);
    }

    pub fn set_inner_html(el: &Element, html: &str) {
        GtkRenderer::set_inner_html(el, html);
    }

    pub fn get_template<V: 'static>() -> TemplateElement {
        GtkRenderer::get_template::<V>()
    }

    pub fn clone_template(tpl: &TemplateElement) -> Element {
        GtkRenderer::clone_template(tpl)
    }

    /// Mount `new_child` immediately before `before` in `before`'s
    /// parent. Used by dynamic-children diffing in tachys (`<For>`,
    /// keyed iteration, etc.).
    ///
    /// Unlike the macOS renderer's equivalent, this is a one-liner:
    /// GTK has no Taffy tree to register against, so we just look up
    /// `before`'s parent widget, wrap it as a synthetic Element, and
    /// call `mount`. The actual insert-before-marker logic lives in
    /// [`gtk_dom::Element::insert_node`] (it uses
    /// `gtk::Box::insert_child_after` on the marker's previous
    /// sibling, so the new child lands immediately before the
    /// marker).
    pub fn mount_before<M>(new_child: &mut M, before: &Node)
    where
        M: Mountable + ?Sized,
    {
        let parent = synthesise_parent_element(before)
            .expect("gtk_dom: mount_before — node has no parent");
        new_child.mount(&parent, Some(before));
    }

    pub fn try_mount_before<M>(new_child: &mut M, before: &Node) -> bool
    where
        M: Mountable + ?Sized,
    {
        let Some(parent) = synthesise_parent_element(before) else {
            return false;
        };
        new_child.try_mount(&parent, Some(before))
    }
}

/// Build an `Element` wrapper around the parent of `before`. Returns
/// `None` if `before` has no parent (i.e. isn't currently mounted).
///
/// On macOS the equivalent helper additionally has to thread a
/// `LayoutHandle` borrowed from the marker so the new child registers
/// in the right Taffy tree. On GTK there's nothing to thread — the
/// parent widget downcasts inside `Element::insert_node` figure out
/// the correct insertion call (Box::insert_child_after, Window::set_child,
/// …), and child layout is the parent's responsibility once the
/// widget is parented.
fn synthesise_parent_element(before: &Node) -> Option<Element> {
    use gtk_dom::gtk::prelude::*;

    let parent_widget = before.widget().parent()?;
    let node = Node::from_widget(parent_widget, NodeKind::Element);
    Some(Element::from_node_unchecked(node))
}

// ---------------------------------------------------------------------
// Mountable — wires our DOM-shaped wrappers into the tachys view tree.
// ---------------------------------------------------------------------

impl Mountable for Node {
    fn unmount(&mut self) {
        // Detach from parent; gobject ref-counting cleans up the
        // widget when both the wrapping Node clones and any
        // remaining parent refs drop.
        self.teardown();
    }

    fn mount(&mut self, parent: &Element, marker: Option<&Node>) {
        Dom::insert_node(parent, self, marker);
    }

    fn try_mount(&mut self, parent: &Element, marker: Option<&Node>) -> bool {
        Dom::try_insert_node(parent, self, marker)
    }

    fn insert_before_this(&self, child: &mut dyn Mountable) -> bool {
        Dom::try_mount_before(child, self)
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

    fn try_mount(&mut self, parent: &Element, marker: Option<&Node>) -> bool {
        Dom::try_insert_node(parent, self.as_node(), marker)
    }

    fn insert_before_this(&self, child: &mut dyn Mountable) -> bool {
        Dom::try_mount_before(child, self.as_node())
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

    fn try_mount(&mut self, parent: &Element, marker: Option<&Node>) -> bool {
        Dom::try_insert_node(parent, self.as_node(), marker)
    }

    fn insert_before_this(&self, child: &mut dyn Mountable) -> bool {
        Dom::try_mount_before(child, self.as_node())
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

    fn try_mount(&mut self, parent: &Element, marker: Option<&Node>) -> bool {
        Dom::try_insert_node(parent, self.as_node(), marker)
    }

    fn insert_before_this(&self, child: &mut dyn Mountable) -> bool {
        Dom::try_mount_before(child, self.as_node())
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

// Identity cast on Element. The web target has a JsCast-bounded
// blanket impl `CastFrom<Element> for T: JsCast`; on native there's
// no JsCast equivalent and the only callers live in the html module
// (currently disabled on native — see implementation_log.md).
impl CastFrom<Element> for Element {
    fn cast_from(source: Element) -> Option<Element> {
        Some(source)
    }
}
