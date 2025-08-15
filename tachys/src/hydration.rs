use crate::{
    renderer::{CastFrom, Rndr},
    view::{Position, PositionState},
};
#[cfg(any(debug_assertions, leptos_debuginfo))]
use std::cell::Cell;
use std::{cell::RefCell, panic::Location, rc::Rc};
use web_sys::{Comment, Element, Node, Text};

/// Hydration works by walking over the DOM, adding interactivity as needed.
///
/// This cursor tracks the location in the DOM that is currently being hydrated. Each that type
/// implements [`RenderHtml`](crate::view::RenderHtml) knows how to advance the cursor to access
/// the nodes it needs.
#[derive(Debug)]
pub struct Cursor(Rc<RefCell<crate::renderer::types::Node>>);

impl Clone for Cursor {
    fn clone(&self) -> Self {
        Self(Rc::clone(&self.0))
    }
}

impl Cursor
where
    crate::renderer::types::Element: AsRef<crate::renderer::types::Node>,
{
    /// Creates a new cursor starting at the root element.
    pub fn new(root: crate::renderer::types::Element) -> Self {
        let root = <crate::renderer::types::Element as AsRef<
            crate::renderer::types::Node,
        >>::as_ref(&root)
        .clone();
        Self(Rc::new(RefCell::new(root)))
    }

    /// Returns the node at which the cursor is currently located.
    pub fn current(&self) -> crate::renderer::types::Node {
        self.0.borrow().clone()
    }

    /// Advances to the next child of the node at which the cursor is located.
    ///
    /// Does nothing if there is no child.
    pub fn child(&self) {
        //crate::log("advancing to next child of ");
        //Rndr::log_node(&self.current());
        let mut inner = self.0.borrow_mut();
        if let Some(node) = Rndr::first_child(&inner) {
            *inner = node;
        }
        //drop(inner);
        //crate::log(">> which is ");
        //Rndr::log_node(&self.current());
    }

    /// Advances to the next sibling of the node at which the cursor is located.
    ///
    /// Does nothing if there is no sibling.
    pub fn sibling(&self) {
        //crate::log("advancing to next sibling of ");
        //Rndr::log_node(&self.current());
        let mut inner = self.0.borrow_mut();
        if let Some(node) = Rndr::next_sibling(&inner) {
            *inner = node;
        }
        //drop(inner);
        //crate::log(">> which is ");
        //Rndr::log_node(&self.current());
    }

    /// Moves to the parent of the node at which the cursor is located.
    ///
    /// Does nothing if there is no parent.
    pub fn parent(&self) {
        let mut inner = self.0.borrow_mut();
        if let Some(node) = Rndr::get_parent(&inner) {
            *inner = node;
        }
    }

    /// Sets the cursor to some node.
    pub fn set(&self, node: crate::renderer::types::Node) {
        *self.0.borrow_mut() = node;
    }

    /// Advances to the next placeholder node and returns it
    pub fn next_placeholder(
        &self,
        position: &PositionState,
    ) -> crate::renderer::types::Placeholder {
        //crate::dom::log("looking for placeholder after");
        //Rndr::log_node(&self.current());
        self.advance_to_placeholder(position);
        let marker = self.current();
        crate::renderer::types::Placeholder::cast_from(marker.clone())
            .unwrap_or_else(|| failed_to_cast_marker_node(marker))
    }

    /// Advances to the next placeholder node.
    pub fn advance_to_placeholder(&self, position: &PositionState) {
        if position.get() == Position::FirstChild {
            self.child();
        } else {
            self.sibling();
        }
        position.set(Position::NextChild);
    }
}

#[cfg(any(debug_assertions, leptos_debuginfo))]
thread_local! {
    static CURRENTLY_HYDRATING: Cell<Option<&'static Location<'static>>> = const { Cell::new(None) };
}

pub(crate) fn set_currently_hydrating(
    location: Option<&'static Location<'static>>,
) {
    #[cfg(any(debug_assertions, leptos_debuginfo))]
    {
        CURRENTLY_HYDRATING.set(location);
    }
    #[cfg(not(any(debug_assertions, leptos_debuginfo)))]
    {
        _ = location;
    }
}

pub(crate) fn failed_to_cast_element(tag_name: &str, node: Node) -> Element {
    #[cfg(not(any(debug_assertions, leptos_debuginfo)))]
    {
        _ = node;
        unreachable!();
    }
    #[cfg(any(debug_assertions, leptos_debuginfo))]
    {
        let hydrating = CURRENTLY_HYDRATING
            .take()
            .map(|n| n.to_string())
            .unwrap_or_else(|| "{unknown}".to_string());
        web_sys::console::error_3(
            &wasm_bindgen::JsValue::from_str(&format!(
                "A hydration error occurred while trying to hydrate an \
                 element defined at {hydrating}.\n\nThe framework expected an \
                 HTML <{tag_name}> element, but found this instead: ",
            )),
            &node,
            &wasm_bindgen::JsValue::from_str(
                "\n\nThe hydration mismatch may have occurred slightly \
                 earlier, but this is the first time the framework found a \
                 node of an unexpected type.",
            ),
        );
        panic!(
            "Unrecoverable hydration error. Please read the error message \
             directly above this for more details."
        );
    }
}

pub(crate) fn failed_to_cast_marker_node(node: Node) -> Comment {
    #[cfg(not(any(debug_assertions, leptos_debuginfo)))]
    {
        _ = node;
        unreachable!();
    }
    #[cfg(any(debug_assertions, leptos_debuginfo))]
    {
        let hydrating = CURRENTLY_HYDRATING
            .take()
            .map(|n| n.to_string())
            .unwrap_or_else(|| "{unknown}".to_string());
        web_sys::console::error_3(
            &wasm_bindgen::JsValue::from_str(&format!(
                "A hydration error occurred while trying to hydrate an \
                 element defined at {hydrating}.\n\nThe framework expected a \
                 marker node, but found this instead: ",
            )),
            &node,
            &wasm_bindgen::JsValue::from_str(
                "\n\nThe hydration mismatch may have occurred slightly \
                 earlier, but this is the first time the framework found a \
                 node of an unexpected type.",
            ),
        );
        panic!(
            "Unrecoverable hydration error. Please read the error message \
             directly above this for more details."
        );
    }
}

pub(crate) fn failed_to_cast_text_node(node: Node) -> Text {
    #[cfg(not(any(debug_assertions, leptos_debuginfo)))]
    {
        _ = node;
        unreachable!();
    }
    #[cfg(any(debug_assertions, leptos_debuginfo))]
    {
        let hydrating = CURRENTLY_HYDRATING
            .take()
            .map(|n| n.to_string())
            .unwrap_or_else(|| "{unknown}".to_string());
        web_sys::console::error_3(
            &wasm_bindgen::JsValue::from_str(&format!(
                "A hydration error occurred while trying to hydrate an \
                 element defined at {hydrating}.\n\nThe framework expected a \
                 text node, but found this instead: ",
            )),
            &node,
            &wasm_bindgen::JsValue::from_str(
                "\n\nThe hydration mismatch may have occurred slightly \
                 earlier, but this is the first time the framework found a \
                 node of an unexpected type.",
            ),
        );
        panic!(
            "Unrecoverable hydration error. Please read the error message \
             directly above this for more details."
        );
    }
}
