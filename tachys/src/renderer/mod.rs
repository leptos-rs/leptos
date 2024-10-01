use crate::view::{Mountable, ToTemplate};
use std::{borrow::Cow, fmt::Debug};
use wasm_bindgen::JsValue;

/// A DOM renderer.
pub mod dom;

pub type Rndr = dom::Dom;
pub mod types {
    pub use super::dom::{
        ClassList, CssStyleDeclaration, Element, Event, Node, Placeholder,
        TemplateElement, Text,
    };
}

/* #[cfg(feature = "testing")]
/// A renderer based on a mock DOM.
pub mod mock_dom;
/// A DOM renderer optimized for element creation.
#[cfg(feature = "sledgehammer")]
pub mod sledgehammer; */

/// Implements the instructions necessary to render an interface on some platform.
///
/// By default, this is implemented for the Document Object Model (DOM) in a Web
/// browser, but implementing this trait for some other platform allows you to use
/// the library to render any tree-based UI.
pub trait Renderer: Send + Sized + Debug + 'static {
    /// The basic type of node in the view tree.
    type Node: Mountable + Clone + 'static;
    /// A visible element in the view tree.
    type Element: AsRef<Self::Node>
        + CastFrom<Self::Node>
        + Mountable
        + Clone
        + 'static;
    /// A text node in the view tree.
    type Text: AsRef<Self::Node>
        + CastFrom<Self::Node>
        + Mountable
        + Clone
        + 'static;
    /// A placeholder node, which can be inserted into the tree but does not
    /// appear (e.g., a comment node in the DOM).
    type Placeholder: AsRef<Self::Node>
        + CastFrom<Self::Node>
        + Mountable
        + Clone
        + 'static;

    /// Interns a string slice, if that is available on this platform and useful as an optimization.
    fn intern(text: &str) -> &str;

    /// Creates a new text node.
    fn create_text_node(text: &str) -> Self::Text;

    /// Creates a new placeholder node.
    fn create_placeholder() -> Self::Placeholder;

    /// Sets the text content of the node. If it's not a text node, this does nothing.
    fn set_text(node: &Self::Text, text: &str);

    /// Sets the given attribute on the given node by key and value.
    fn set_attribute(node: &Self::Element, name: &str, value: &str);

    /// Removes the given attribute on the given node.
    fn remove_attribute(node: &Self::Element, name: &str);

    /// Appends the new child to the parent, before the anchor node. If `anchor` is `None`,
    /// append to the end of the parent's children.
    fn insert_node(
        parent: &Self::Element,
        new_child: &Self::Node,
        marker: Option<&Self::Node>,
    );

    /// Removes the child node from the parents, and returns the removed node.
    fn remove_node(
        parent: &Self::Element,
        child: &Self::Node,
    ) -> Option<Self::Node>;

    /// Removes all children from the parent element.
    fn clear_children(parent: &Self::Element);

    /// Removes the node.
    fn remove(node: &Self::Node);

    /// Gets the parent of the given node, if any.
    fn get_parent(node: &Self::Node) -> Option<Self::Node>;

    /// Returns the first child node of the given node, if any.
    fn first_child(node: &Self::Node) -> Option<Self::Node>;

    /// Returns the next sibling of the given node, if any.
    fn next_sibling(node: &Self::Node) -> Option<Self::Node>;

    /// Logs the given node in a platform-appropriate way.
    fn log_node(node: &Self::Node);
}

/// A function that can be called to remove an event handler from an element after it has been added.
#[must_use = "This will invalidate the event handler when it is dropped. You \
              should store it in some other data structure to clean it up \
              later to avoid dropping it immediately, or leak it with \
              std::mem::forget() to never drop it."]
pub struct RemoveEventHandler<T>(Box<dyn FnOnce(&T) + Send + Sync>);

impl<T> RemoveEventHandler<T> {
    /// Creates a new container with a function that will be called when it is dropped.
    pub(crate) fn new(remove: impl FnOnce(&T) + Send + Sync + 'static) -> Self {
        Self(Box::new(remove))
    }

    pub(crate) fn into_inner(self) -> Box<dyn FnOnce(&T) + Send + Sync> {
        self.0
    }
}

/// Additional rendering behavior that applies only to DOM nodes.
pub trait DomRenderer: Renderer {
    /// Generic event type, from which any specific event can be converted.
    type Event;
    /// The list of CSS classes for an element.
    type ClassList: Clone + 'static;
    /// The CSS styles for an element.
    type CssStyleDeclaration: Clone + 'static;
    /// The type of a `<template>` element.
    type TemplateElement;

    /// Sets a JavaScript object property on a DOM element.
    fn set_property(el: &Self::Element, key: &str, value: &JsValue);

    /// Adds an event listener to an element.
    ///
    /// Returns a function to remove the listener.
    fn add_event_listener(
        el: &Self::Element,
        name: &str,
        cb: Box<dyn FnMut(Self::Event)>,
    ) -> RemoveEventHandler<Self::Element>;

    /// Adds an event listener to an element, delegated to the window if possible.
    ///
    /// Returns a function to remove the listener.
    fn add_event_listener_delegated(
        el: &Self::Element,
        name: Cow<'static, str>,
        delegation_key: Cow<'static, str>,
        cb: Box<dyn FnMut(Self::Event)>,
    ) -> RemoveEventHandler<Self::Element>;

    /// Return the `event.target`, cast to the given type.
    fn event_target<T>(ev: &Self::Event) -> T
    where
        T: CastFrom<Self::Element>;

    /// The list of CSS classes for an element.
    fn class_list(el: &Self::Element) -> Self::ClassList;

    /// Add a class to the list.
    fn add_class(class_list: &Self::ClassList, name: &str);

    /// Remove a class from the list.
    fn remove_class(class_list: &Self::ClassList, name: &str);

    /// The set of styles for an element.
    fn style(el: &Self::Element) -> Self::CssStyleDeclaration;

    /// Sets a CSS property.
    fn set_css_property(
        style: &Self::CssStyleDeclaration,
        name: &str,
        value: &str,
    );

    /// Sets the `innerHTML` of a DOM element, without escaping any values.
    fn set_inner_html(el: &Self::Element, html: &str);

    /// Returns a cached template element created from the given type.
    fn get_template<V>() -> Self::TemplateElement
    where
        V: ToTemplate + 'static;

    /// Deeply clones a template.
    fn clone_template(tpl: &Self::TemplateElement) -> Self::Element;

    /// Creates a single element from a string of HTML.
    fn create_element_from_html(html: &str) -> Self::Element;
}

/// Attempts to cast from one type to another.
///
/// This works in a similar way to `TryFrom`. We implement it as a separate trait
/// simply so we don't have to create wrappers for the `web_sys` types; it can't be
/// implemented on them directly because of the orphan rules.

pub trait CastFrom<T>
where
    Self: Sized,
{
    /// Casts a node from one type to another.
    fn cast_from(source: T) -> Option<Self>;
}
