use crate::{
    renderer::{CastFrom, Rndr},
    view::{Position, PositionState},
};
use std::{cell::RefCell, rc::Rc};

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

    /// Advances to the next placeholder node.
    pub fn next_placeholder(
        &self,
        position: &PositionState,
    ) -> crate::renderer::types::Placeholder {
        //crate::dom::log("looking for placeholder after");
        //Rndr::log_node(&self.current());
        if position.get() == Position::FirstChild {
            self.child();
        } else {
            self.sibling();
        }
        let marker = self.current();
        position.set(Position::NextChild);
        crate::renderer::types::Placeholder::cast_from(marker)
            .expect("could not convert current node into marker node")
        /*let marker2 = marker.clone();
        Rndr::Placeholder::cast_from(marker).unwrap_or_else(|| {
            crate::dom::log("expecting to find a marker. instead, found");
            Rndr::log_node(&marker2);
            panic!("oops.");
        })*/
    }
}
