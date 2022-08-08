use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};

use crate::{Observer, RootContext, Scope, State, SuspenseContext, Update, WeakSignalState};

pub(crate) type Source = WeakSignalState;

pub(crate) trait AnyComputation {
    fn sources_empty(&self) -> bool;

    fn sources_len(&self) -> usize;

    fn set_source_slots(&self, source_slots: Vec<usize>);

    fn set_sources(&self, sources: Vec<Source>);

    fn push_source(&self, source: Source);

    fn push_source_slot(&self, source_slot: usize);

    fn state(&self) -> Option<State>;

    fn t_state(&self) -> Option<State>;

    fn set_state(&self, state: Option<State>);

    fn set_t_state(&self, state: Option<State>);

    fn pure(&self) -> bool;

    fn suspense(&self) -> Option<&SuspenseContext>;

    fn updated_at(&self) -> Option<Update>;

    fn owner(&self) -> Option<Observer>;

    fn sources(&self) -> Vec<Source>;

    fn source_slots(&self) -> Vec<usize>;

    fn pop_source_and_slot(&self) -> Option<(Source, usize)>;

    fn run_computation(self: Rc<Self>, time: Update);
}

pub struct Computation<T> {
    inner: Rc<ComputationInner<T>>,
}

impl<T> Computation<T> {
    pub fn new(
        cx: Scope,
        f: impl FnMut(Option<&T>) -> T + 'static,
        init: Option<T>,
        pure: bool,
    ) -> Self {
        let root = cx.inner.root_context;
        let running_transition = root.running_transition().is_some();
        let state = if running_transition {
            None
        } else {
            Some(State::Stale)
        };
        let t_state = if running_transition {
            None
        } else {
            Some(State::Stale)
        };

        Self {
            inner: Rc::new(ComputationInner {
                root_context: cx.root_context(),
                f: RefCell::new(Box::new(f)),
                owner: RefCell::new(cx.root_context().owner.borrow().clone()),
                state: RefCell::new(state),
                t_state: RefCell::new(t_state),
                sources: Default::default(),
                source_slots: Default::default(),
                value: RefCell::new(init),
                updated_at: Cell::new(Update(0)),
                pure: Cell::new(pure),
                user: Cell::new(false),
                suspense: RefCell::new(None),
            }),
        }
    }

    pub(crate) fn set_user(&self, user: bool) {
        self.inner.user.set(user);
    }
}

pub struct ComputationInner<T> {
    root_context: &'static RootContext,
    f: RefCell<Box<dyn FnMut(Option<&T>) -> T>>,
    owner: RefCell<Option<Observer>>,
    state: RefCell<Option<State>>,
    t_state: RefCell<Option<State>>,
    sources: RefCell<Vec<Source>>,
    source_slots: RefCell<Vec<usize>>,
    value: RefCell<Option<T>>,
    updated_at: Cell<Update>,
    pure: Cell<bool>,
    user: Cell<bool>,
    suspense: RefCell<Option<SuspenseContext>>,
}

impl<T> AnyComputation for Computation<T> {
    fn sources_empty(&self) -> bool {
        self.inner.sources.borrow().is_empty()
    }

    fn sources_len(&self) -> usize {
        self.inner.sources.borrow().len()
    }

    fn set_source_slots(&self, source_slots: Vec<usize>) {
        *self.inner.source_slots.borrow_mut() = source_slots;
    }

    fn set_sources(&self, sources: Vec<Source>) {
        *self.inner.sources.borrow_mut() = sources;
    }

    fn push_source(&self, source: Source) {
        self.inner.sources.borrow_mut().push(source);
    }

    fn push_source_slot(&self, source_slot: usize) {
        self.inner.source_slots.borrow_mut().push(source_slot);
    }

    fn state(&self) -> Option<State> {
        self.inner.state.borrow().clone()
    }

    fn t_state(&self) -> Option<State> {
        self.inner.t_state.borrow().clone()
    }

    fn pure(&self) -> bool {
        self.inner.pure.get()
    }

    fn set_state(&self, state: Option<State>) {
        *self.inner.state.borrow_mut() = state;
    }

    fn set_t_state(&self, state: Option<State>) {
        *self.inner.t_state.borrow_mut() = state;
    }

    fn suspense(&self) -> Option<&SuspenseContext> {
        todo!() //self.inner.suspense.borrow().as_ref()
    }

    fn updated_at(&self) -> Option<Update> {
        Some(self.inner.updated_at.get())
    }

    fn owner(&self) -> Option<Observer> {
        self.inner.owner.borrow().clone()
    }

    fn sources(&self) -> Vec<Source> {
        self.inner.sources.borrow().to_vec()
    }

    fn source_slots(&self) -> Vec<usize> {
        self.inner.source_slots.borrow().to_vec()
    }

    fn pop_source_and_slot(&self) -> Option<(Source, usize)> {
        let source = self.inner.sources.borrow_mut().pop();
        let slot = self.inner.source_slots.borrow_mut().pop();
        match (source, slot) {
            (Some(o), Some(s)) => Some((o, s)),
            _ => None,
        }
    }

    fn run_computation(self: Rc<Self>, time: Update) {
        let next_value = { (self.inner.f.borrow_mut())(self.inner.value.borrow().as_ref()) };
        if self.inner.updated_at.get() <= time {
            if let Some(transition) = self.inner.root_context.running_transition() {
                todo!()
            } else {
                *self.inner.value.borrow_mut() = Some(next_value);
            }

            self.inner.updated_at.set(time);
        }
    }
}

impl PartialEq for &dyn AnyComputation {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
