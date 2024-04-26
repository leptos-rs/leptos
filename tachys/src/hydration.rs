use crate::{
    renderer::{CastFrom, Renderer},
    view::{Position, PositionState},
};
use std::{cell::RefCell, rc::Rc};

#[derive(Debug)]
pub struct Cursor<R: Renderer>(Rc<RefCell<R::Node>>);

impl<R: Renderer> Clone for Cursor<R> {
    fn clone(&self) -> Self {
        Self(Rc::clone(&self.0))
    }
}

impl<R> Cursor<R>
where
    R: Renderer,

    R::Element: AsRef<R::Node>,
{
    pub fn new(root: R::Element) -> Self {
        Self(Rc::new(RefCell::new(root.as_ref().clone())))
    }

    pub fn current(&self) -> R::Node {
        self.0.borrow().clone()
    }

    pub fn child(&self) {
        //crate::log("advancing to next child of ");
        //R::log_node(&self.current());
        let mut inner = self.0.borrow_mut();
        if let Some(node) = R::first_child(&*inner) {
            *inner = node;
        }
        //drop(inner);
        //crate::log(">> which is ");
        //R::log_node(&self.current());
    }

    pub fn sibling(&self) {
        //crate::log("advancing to next sibling of ");
        //R::log_node(&self.current());
        let mut inner = self.0.borrow_mut();
        if let Some(node) = R::next_sibling(&*inner) {
            *inner = node;
        }
        //drop(inner);
        //crate::log(">> which is ");
        //R::log_node(&self.current());
    }

    pub fn parent(&self) {
        let mut inner = self.0.borrow_mut();
        if let Some(node) = R::get_parent(&*inner) {
            *inner = node;
        }
    }

    pub fn set(&self, node: R::Node) {
        *self.0.borrow_mut() = node;
    }

    pub fn next_placeholder(&self, position: &PositionState) -> R::Placeholder {
        //crate::dom::log("looking for placeholder after");
        //R::log_node(&self.current());
        if position.get() == Position::FirstChild {
            self.child();
        } else {
            self.sibling();
        }
        let marker = self.current();
        position.set(Position::NextChild);
        R::Placeholder::cast_from(marker)
            .expect("could not convert current node into marker node")
        /*let marker2 = marker.clone();
        R::Placeholder::cast_from(marker).unwrap_or_else(|| {
            crate::dom::log("expecting to find a marker. instead, found");
            R::log_node(&marker2);
            panic!("oops.");
        })*/
    }
}
