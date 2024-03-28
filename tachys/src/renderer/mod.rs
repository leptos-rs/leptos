use crate::{
    html::element::CreateElement,
    view::{Mountable, ToTemplate},
};
use std::{borrow::Cow, fmt::Debug};
use wasm_bindgen::JsValue;

pub mod dom;
#[cfg(feature = "testing")]
pub mod mock_dom;
#[cfg(feature = "sledgehammer")]
pub mod sledgehammer;

/// Implements the instructions necessary to render an interface on some platform.
/// By default, this is implemented for the Document Object Model (DOM) in a Web
/// browser, but implementing this trait for some other platform allows you to use
/// the library to render any tree-based UI.
pub trait Renderer: Sized + Debug {
    /// The basic type of node in the view tree.
    type Node: Mountable<Self> + Clone + 'static;
    /// A visible element in the view tree.
    type Element: AsRef<Self::Node>
        + CastFrom<Self::Node>
        + Mountable<Self>
        + Clone
        + 'static;
    /// A text node in the view tree.
    type Text: AsRef<Self::Node>
        + CastFrom<Self::Node>
        + Mountable<Self>
        + Clone
        + 'static;
    /// A placeholder node, which can be inserted into the tree but does not
    /// appear (e.g., a comment node in the DOM).
    type Placeholder: AsRef<Self::Node>
        + CastFrom<Self::Node>
        + Mountable<Self>
        + Clone
        + 'static;

    fn intern(text: &str) -> &str;

    /// Creates a new element node.
    fn create_element<E: CreateElement<Self>>(tag: E) -> Self::Element {
        tag.create_element()
    }

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

    /// Mounts the new child before the marker as its sibling.
    ///
    /// ## Panics
    /// The default implementation panics if `before` does not have a parent [`R::Element`].
    fn mount_before<M>(new_child: &mut M, before: &Self::Node)
    where
        M: Mountable<Self>,
    {
        let parent = Self::Element::cast_from(
            Self::get_parent(before).expect("node should have parent"),
        )
        .expect("placeholder parent should be Element");
        new_child.mount(&parent, Some(before));
    }

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

    fn log_node(node: &Self::Node);
}

/// A function that can be called to remove an event handler from an element after it has been added.
pub type RemoveEventHandler<T> = Box<dyn FnOnce(&T) + Send>;
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

    fn clone_template(tpl: &Self::TemplateElement) -> Self::Element;
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
    fn cast_from(source: T) -> Option<Self>;
}
