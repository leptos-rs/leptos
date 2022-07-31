use std::{cell::RefCell, rc::Weak};

use crate::EffectInner;

pub struct RootContext {
    pub(crate) stack: RefCell<Vec<Weak<EffectInner>>>,
}

impl RootContext {
    pub fn new() -> Self {
        Self {
            stack: RefCell::new(Vec::new()),
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
