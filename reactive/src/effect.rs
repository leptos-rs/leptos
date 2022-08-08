use std::rc::Rc;

use crate::{AnyComputation, BoundedScope, Computation, Observer};

impl<'a, 'b> BoundedScope<'a, 'b> {
    pub fn create_effect<T>(self, effect_fn: impl FnMut(Option<&T>) -> T + 'a)
    where
        T: 'static,
    {
        let f: Box<dyn FnMut(Option<&T>) -> T + 'a> = Box::new(effect_fn);
        // SAFETY: Memo will be cleaned up when the Scope lifetime 'a is over,
        // and will no longer be accessible; for its purposes, 'a: 'static
        // This is necessary to allow &'a Signal<_> etc. to be moved into F
        let f: Box<dyn FnMut(Option<&T>) -> T + 'static> = unsafe { std::mem::transmute(f) };

        let c = Computation::new(self, f, None, false);
        // TODO suspense piece here
        c.set_user(true);
        let root = self.root_context();

        // TODO leak because not bumpalo::Box
        let c = self.create_ref(Rc::new(c) as Rc<dyn AnyComputation>);

        if let Some(effects) = &mut *root.effects.borrow_mut() {
            effects.push(Observer(Rc::downgrade(c)));
        } else {
            root.update_computation(Rc::downgrade(c))
        }
    }
}
