use std::{
    cell::RefCell,
    hash::Hash,
    rc::{Rc, Weak},
};

use super::root_context::RootContext;

#[derive(Clone)]
pub struct Effect {
    pub(crate) inner: Rc<EffectInner>,
}

pub(crate) struct EffectInner {
    pub(crate) stack: &'static RootContext,
    pub(crate) f: RefCell<Box<dyn FnMut()>>,
    pub(crate) dependencies: RefCell<Vec<Weak<dyn EffectDependency>>>,
}

pub(crate) trait EffectDependency {
    fn unsubscribe(&self, effect: Rc<EffectInner>);
}

impl std::fmt::Debug for Effect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Effect").finish()
    }
}

impl Effect {
    pub(crate) fn execute(&self) {
        self.inner.execute(Rc::downgrade(&self.inner));
    }
}

impl EffectInner {
    pub(crate) fn execute(&self, for_stack: Weak<EffectInner>) {
        // clear previous dependencies
        // at this point, Effect dependencies have been added to Signal
        // and any Signal changes will call Effect dependency automatically
        if let Some(upgraded) = for_stack.upgrade() {
            self.cleanup(upgraded);
        }

        // add it to the Scope stack, which means any signals called
        // in the effect fn immediately below will add this Effect as a dependency
        self.stack.push(for_stack);

        // actually run the effect, which will re-add Signal dependencies as they're called
        (self.f.borrow_mut())();

        // pop it back off the stack
        self.stack.pop();
    }

    pub(crate) fn cleanup(&self, for_subscriber: Rc<EffectInner>) {
        // remove the Effect from the subscripts of each Signal to which it is subscribed
        // these were called during a previous execution of the Effect
        // they will be resubscribed, if necessary, during the coming execution
        // this kind of dynamic dependency graph reconstruction may seem silly,
        // but is actually more efficient because it avoids resubscribing with Signals
        // if they are excluded by some kind of conditional logic within the Effect fn
        for dep in self.dependencies.borrow().iter() {
            if let Some(dep) = dep.upgrade() {
                dep.unsubscribe(for_subscriber.clone());
            }
        }
        // and clear all our dependencies on Signals; these will be built back up
        // by the Signals if/when they are called again
        self.dependencies.borrow_mut().clear();
    }

    pub(crate) fn add_dependency(&self, dep: Weak<dyn EffectDependency>) {
        self.dependencies.borrow_mut().push(dep);
    }
}

impl PartialEq for Effect {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl PartialEq for EffectInner {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(&self.f, &other.f) && std::ptr::eq(&self.dependencies, &other.dependencies)
    }
}
impl Eq for EffectInner {}

impl Hash for EffectInner {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::ptr::hash(&self.f, state);
        std::ptr::hash(&self.dependencies, state);
    }
}
