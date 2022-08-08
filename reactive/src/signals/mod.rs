mod accessor;
mod memo;
mod signal;

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

pub use accessor::*;
pub use memo::*;
pub use signal::*;

use crate::{AnyComputation, BoundedScope, Computation, Scope, Update};

pub type Signal<'a, T> = (&'a ReadSignal<T>, &'a WriteSignal<T>);

impl<'a, 'b> BoundedScope<'a, 'b> {
    /// Creates a reactive [Signal](crate::Signal) consisting of a
    /// `([ReadSignal](crate::ReadSignal), [WriteSignal](crate::WriteSignal)`
    /// pair. This is the basic building block of reactivity.
    /// ```
    /// # use reactive::{create_scope, RootContext, Scope};
    /// # let root = Box::leak(Box::new(RootContext::new()));
    /// # let _ = create_scope(root, |cx| {
    /// let (a, set_a) = cx.create_signal(0);
    /// assert_eq!(a(), 0);
    /// set_a(|a| *a += 1);
    /// assert_eq!(a(), 1);
    /// # });
    /// ```
    pub fn create_signal<T>(self, value: T) -> (&'a ReadSignal<T>, &'a WriteSignal<T>)
    where
        T: 'static,
    {
        let (read, write) = self.create_signal_owned(value);
        (self.create_ref(read), self.create_ref(write))
    }
    ///
    pub fn create_signal_owned<T>(self, value: T) -> (ReadSignal<T>, WriteSignal<T>)
    where
        T: 'static,
    {
        // create a SignalState that will be shared between the Setter & Accessor
        let signal_state = SignalState::new(value);
        let root_context = self.inner.root_context;

        let getter = ReadSignal {
            root_context,
            state: signal_state.inner.clone(),
        };

        let setter = WriteSignal {
            root_context,
            state: Rc::downgrade(&signal_state.inner),
        };

        // store the signal state in the Scope's arena, so it will live as long as the Scope
        (getter, setter)
    }

    /// Creates a read-only, memoized signal derived from the given function. The function
    /// takes the previous value of the memo as its argument (as `Option<T>`, as it is `None`
    /// the first time the memo is read.)
    /// ```
    /// # use reactive::{create_scope, RootContext, Scope};
    /// # let root = Box::leak(Box::new(RootContext::new()));
    /// # let _ = create_scope(root, |cx| {
    /// let (a, set_a) = cx.create_signal(0);
    /// let b = cx.create_memo(move |_| a() * 2);
    /// assert_eq!(a(), 0);
    /// set_a(|a| *a = 2);
    /// assert_eq!(b(), 4);
    /// # });
    /// ```
    pub fn create_memo<T>(self, f: impl Fn(Option<&T>) -> T + 'a) -> &'a Memo<T>
    where
        T: PartialEq + 'static,
    {
        let f: Box<dyn Fn(Option<&T>) -> T + 'a> = Box::new(f);
        // SAFETY: Memo will be cleaned up when the Scope lifetime 'a is over,
        // and will no longer be accessible; for its purposes, 'a: 'static
        // This is necessary to allow &'a Signal<_> etc. to be moved into F
        let f: Box<dyn Fn(Option<&T>) -> T + 'static> = unsafe { std::mem::transmute(f) };

        let init = f(None);

        let m = Memo {
            root_context: self.root_context(),
            state: Rc::new(MemoInner {
                root_context: self.root_context(),
                f,
                value: RefCell::new(init),
                t_value: RefCell::new(None),
                observers: Default::default(),
                observer_slots: Default::default(),
                state: Default::default(),
                sources: Default::default(),
                source_slots: Default::default(),
                t_state: Default::default(),
                updated_at: Cell::new(Update(0)),
                pending: RefCell::new(Pending::NotPending),
            }),
        };

        let computation = Rc::clone(&m.state) as Rc<dyn AnyComputation>;
        self.root_context()
            .update_computation(Rc::downgrade(&computation));

        self.create_ref(m)
    }
}
