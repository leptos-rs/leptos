use crate::renderer::Renderer;
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
        let mut inner = self.0.borrow_mut();
        if let Some(node) = R::first_child(&*inner) {
            *inner = node;
        }
    }

    pub fn sibling(&self) {
        let mut inner = self.0.borrow_mut();
        if let Some(node) = R::next_sibling(&*inner) {
            *inner = node;
        }
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
}
