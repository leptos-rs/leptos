use std::{cell::RefCell, rc::Weak};

use crate::{EffectInner, ReadSignal, TransitionState, WriteSignal};

pub struct RootContext {
    pub(crate) stack: RefCell<Vec<Weak<EffectInner>>>,
    pub(crate) transition_pending: RefCell<Option<(ReadSignal<bool>, WriteSignal<bool>)>>,
    pub(crate) transition: RefCell<Option<TransitionState>>,
}

impl RootContext {
    pub fn new() -> Self {
        Self {
            stack: Default::default(),
            transition_pending: Default::default(),
            transition: Default::default(),
        }
    }

    pub(crate) fn push(&self, effect: Weak<EffectInner>) {
        self.stack.borrow_mut().push(effect);
    }

    pub(crate) fn pop(&self) {
        self.stack.borrow_mut().pop();
    }

    pub(crate) fn untrack<T>(&self, f: impl FnOnce() -> T) -> T {
        let prev_stack = self.stack.replace(Vec::new());
        let untracked_result = f();
        self.stack.replace(prev_stack);
        untracked_result
    }
}

impl Default for RootContext {
    fn default() -> Self {
        Self::new()
    }
}
