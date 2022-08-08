use std::{
    cell::RefCell,
    collections::HashSet,
    rc::{Rc, Weak},
};

use crate::{AnyComputation, AnySignalState, Observer, WeakSignalState};

#[derive(Clone)]
pub struct TransitionState {
    pub(crate) inner: Rc<RefCell<TransitionStateInner>>,
}

impl TransitionState {
    pub(crate) fn contains_source(&self, source: &WeakSignalState) -> bool {
        self.inner.borrow().sources.contains(source)
    }

    pub(crate) fn running(&self) -> bool {
        self.inner.borrow().running
    }

    pub(crate) fn disposed_contains(&self, observer: &Observer) -> bool {
        self.inner.borrow().disposed.contains(observer)
    }
}

pub(crate) struct TransitionStateInner {
    pub(crate) sources: HashSet<WeakSignalState>,
    pub(crate) effects: Vec<Observer>,
    // promises — TODO
    pub(crate) disposed: HashSet<Observer>,
    pub(crate) queue: HashSet<Observer>,
    pub(crate) running: bool,
}
