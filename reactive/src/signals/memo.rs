use std::{
    cell::{Cell, Ref, RefCell},
    rc::{Rc, Weak},
};

use crate::{
    Accessor, AnyComputation, AnySignalState, Observer, Pending, ReadSignalRef, RootContext,
    Source, State, SuspenseContext, Update, WeakSignalState,
};

pub struct Memo<T>
where
    T: PartialEq + 'static,
{
    pub(crate) root_context: &'static RootContext,
    pub(crate) state: Rc<MemoInner<T>>,
}

impl<T> Memo<T>
where
    T: PartialEq + 'static,
{
    pub fn get(&self) -> ReadSignalRef<T> {
        self.read()
    }

    fn update(&self) {
        Rc::clone(&self.state).update()
    }
}

impl<T> Clone for Memo<T>
where
    T: PartialEq + 'static,
{
    fn clone(&self) -> Self {
        Self {
            root_context: self.root_context,
            state: Rc::clone(&self.state),
        }
    }
}

pub struct MemoInner<T> {
    pub(crate) root_context: &'static RootContext,
    pub(crate) f: Box<dyn Fn(Option<&T>) -> T>,
    pub(crate) value: RefCell<T>,
    pub(crate) t_value: RefCell<Option<T>>,
    pub(crate) observers: RefCell<Vec<Observer>>,
    pub(crate) observer_slots: RefCell<Vec<usize>>,
    pub(crate) state: Cell<Option<State>>,
    pub(crate) sources: RefCell<Vec<Source>>,
    pub(crate) source_slots: RefCell<Vec<usize>>,
    pub(crate) t_state: Cell<Option<State>>,
    pub(crate) updated_at: Cell<Update>,
    pub(crate) pending: RefCell<Pending<T>>,
}

impl<T> MemoInner<T>
where
    T: PartialEq + 'static,
{
    fn as_weak_signal_state(self: Rc<Self>) -> WeakSignalState {
        WeakSignalState(Rc::downgrade(&self) as Weak<dyn AnySignalState>)
    }

    fn update(self: Rc<Self>) {
        // calculate new value
        {
            if self.root_context.running_transition().is_some() {
                match &*self.pending.borrow() {
                    Pending::NotPending => {
                        let value = { (self.f)(self.t_value.borrow().as_ref()) };
                        *self.t_value.borrow_mut() = Some(value);
                    }
                    Pending::Pending(pending) => {
                        let value = (self.f)(Some(pending));
                        *self.t_value.borrow_mut() = Some(value);
                    }
                }
            } else {
                let value = { (self.f)(Some(&self.value.borrow())) };
                *self.value.borrow_mut() = value;
            };
        }

        // push this onto the list of pending nodes, if it exists
        if self.root_context.pending.borrow().is_some() {
            if *self.pending.borrow() == Pending::NotPending {
                if let Some(pending) = &mut *self.root_context.pending.borrow_mut() {
                    pending.push(self.as_weak_signal_state());
                }
            }
        }
        // otherwise, try to synchronously update
        else {
            let transition = self.root_context.running_transition();

            // if transition value already applies, just return
            if let Some(transition) = &transition {
                if transition.contains_source(&self.clone().as_weak_signal_state())
                    && self.t_value.borrow().as_ref() == Some(&self.value.borrow())
                {
                    return;
                }
            }

            // run update
            let transition_running = transition.as_ref().map(|t| t.running()).unwrap_or(false);
            if let Some(transition) = &transition {
                todo!()
            }

            // notify observers
            let observers_empty = { self.observers.borrow().is_empty() };
            if !observers_empty {
                self.root_context.run_updates(
                    || {
                        for observer in self.observers.borrow().clone() {
                            // if already disposed in transition, ignore
                            if transition_running
                                && transition
                                    .as_ref()
                                    .map(|t| t.disposed_contains(&observer))
                                    .unwrap_or(false)
                            {
                                continue;
                            }

                            //
                            if let Some(observer) = observer.upgrade() {
                                if (transition_running && observer.t_state().is_none())
                                    || (!transition_running && observer.state().is_none())
                                {
                                    if observer.pure() {
                                        /* self.root_context
                                        .push_update(Observer(Rc::downgrade(&observer))); */
                                    } else {
                                        self.root_context
                                            .push_effect(Observer(Rc::downgrade(&observer)));
                                    }

                                    self.mark_downstream();
                                }

                                // mark state as stale
                                if transition_running {
                                    observer.set_t_state(Some(State::Stale));
                                } else {
                                    observer.set_state(Some(State::Stale));
                                }
                            }
                        }
                    },
                    false,
                )
            }
        }
    }

    fn mark_downstream(&self) {
        let running_transition = self.root_context.running_transition().is_some();
        for observer in self.observers.borrow().clone() {
            if let Some(o) = observer.upgrade() {
                if (!running_transition && o.state().is_none())
                    || (running_transition && o.t_state().is_none())
                {
                    if running_transition {
                        o.set_t_state(Some(State::Pending));
                    } else {
                        o.set_state(Some(State::Pending));
                    }

                    if o.pure() {
                        //self.root_context.push_update(observer);
                    } else {
                        self.root_context.push_effect(observer);
                    }

                    // TODO mark downstream if observer is memo
                    /* if let Some(memo) = observer.as_memo() {
                        memo.mark_downstream();
                    } */
                }
            }
        }
    }
}

// Memos are a computation, like an Effect
impl<T> AnyComputation for MemoInner<T>
where
    T: PartialEq + 'static,
{
    fn sources_empty(&self) -> bool {
        self.sources.borrow().is_empty()
    }

    fn sources_len(&self) -> usize {
        self.sources.borrow().len()
    }

    fn set_source_slots(&self, source_slots: Vec<usize>) {
        *self.source_slots.borrow_mut() = source_slots;
    }

    fn set_sources(&self, sources: Vec<Source>) {
        *self.sources.borrow_mut() = sources;
    }

    fn push_source(&self, source: Source) {
        self.sources.borrow_mut().push(source);
    }

    fn push_source_slot(&self, source_slot: usize) {
        self.source_slots.borrow_mut().push(source_slot);
    }

    fn state(&self) -> Option<State> {
        self.state.get()
    }

    fn t_state(&self) -> Option<State> {
        self.t_state.get()
    }

    fn pure(&self) -> bool {
        true
    }

    fn set_state(&self, state: Option<State>) {
        self.state.set(state);
    }

    fn set_t_state(&self, state: Option<State>) {
        self.t_state.set(state);
    }

    fn suspense(&self) -> Option<&SuspenseContext> {
        None
    }

    fn updated_at(&self) -> Option<crate::Update> {
        None
    }

    fn sources(&self) -> Vec<Source> {
        self.sources.borrow().to_vec()
    }

    fn source_slots(&self) -> Vec<usize> {
        self.source_slots.borrow().to_vec()
    }

    fn owner(&self) -> Option<Observer> {
        None
    }

    fn pop_source_and_slot(&self) -> Option<(Source, usize)> {
        let source = self.sources.borrow_mut().pop();
        let slot = self.source_slots.borrow_mut().pop();
        match (source, slot) {
            (Some(o), Some(s)) => Some((o, s)),
            _ => None,
        }
    }

    fn run_computation(self: Rc<Self>, time: Update) {
        let next_value = { (self.f)(Some(&self.value.borrow())) };
        if self.updated_at.get() <= time {
            if let Some(transition) = self.root_context.running_transition() {
                todo!()
            } else {
                *self.value.borrow_mut() = next_value;
            }

            self.clone().update();

            self.updated_at.set(time);
        }
    }
}

// Memos are also a read-only Signal, i.e., they can be read reactively
impl<T> Accessor<T> for Memo<T>
where
    T: PartialEq + 'static,
{
    fn context(&self) -> &'static RootContext {
        self.root_context
    }

    fn is_stale_computation(&self) -> bool {
        let running_transition = self.root_context.running_transition();
        !self.state.sources.borrow().is_empty()
            && (running_transition.is_none()
                || (running_transition.is_some() && self.state.t_value.borrow().is_some()))
    }

    fn update_computation(&self) {
        let running_transition = self.root_context.running_transition();

        let updates = self.root_context.take_updates();
        if (running_transition.is_none() && self.state.state.get() == Some(State::Stale))
            || (running_transition.is_some() && self.state.t_state.get() == Some(State::Stale))
        {
            let computation = Rc::clone(&self.state) as Rc<dyn AnyComputation>;
            self.root_context
                .update_computation(Rc::downgrade(&computation));
        } else {
            self.root_context.look_upstream(self.state.as_ref(), None);
        }
        self.root_context.set_updates(updates);
    }

    fn value(&self) -> ReadSignalRef<T> {
        ReadSignalRef {
            guard: self.state.value.borrow(),
        }
    }

    fn t_value_unchecked(&self) -> ReadSignalRef<T> {
        ReadSignalRef {
            guard: Ref::map(self.state.t_value.borrow(), |s| match &s {
                Some(value) => value,
                None => panic!(),
            }),
        }
    }

    fn as_weak_signal_state(&self) -> WeakSignalState {
        WeakSignalState(Rc::downgrade(&self.state) as Weak<dyn AnySignalState>)
    }

    fn observers_len(&self) -> usize {
        self.state.observers.borrow().len()
    }

    fn push_observer(&self, observer: Observer) {
        self.state.observers.borrow_mut().push(observer)
    }

    fn push_observer_slot(&self, observer_slot: usize) {
        self.state.observer_slots.borrow_mut().push(observer_slot)
    }
}

impl<T> AnySignalState for Memo<T>
where
    T: PartialEq,
{
    fn as_computation(self: Rc<Self>) -> Option<Observer> {
        let computation = Rc::clone(&self.state) as Rc<dyn AnyComputation>;
        Some(Observer(Rc::downgrade(&computation)))
    }

    fn observers(&self) -> Vec<Observer> {
        self.state.observers()
    }

    fn pop_observer_and_slot(&self) -> Option<(Observer, usize)> {
        self.state.pop_observer_and_slot()
    }
}

impl<T> AnySignalState for MemoInner<T>
where
    T: PartialEq + 'static,
{
    fn as_computation(self: Rc<Self>) -> Option<Observer> {
        let computation = Rc::clone(&self) as Rc<dyn AnyComputation>;
        Some(Observer(Rc::downgrade(&computation)))
    }

    fn observers(&self) -> Vec<Observer> {
        self.observers.borrow().to_vec()
    }

    fn pop_observer_and_slot(&self) -> Option<(Observer, usize)> {
        let observer = self.observers.borrow_mut().pop();
        let slot = self.observer_slots.borrow_mut().pop();
        match (observer, slot) {
            (Some(o), Some(s)) => Some((o, s)),
            _ => None,
        }
    }
}

impl<T> FnOnce<()> for Memo<T>
where
    T: PartialEq + Clone + 'static,
{
    type Output = T;

    extern "rust-call" fn call_once(self, _args: ()) -> Self::Output {
        self.get().clone()
    }
}

impl<T> FnMut<()> for Memo<T>
where
    T: PartialEq + Clone + 'static,
{
    extern "rust-call" fn call_mut(&mut self, _args: ()) -> Self::Output {
        self.get().clone()
    }
}

impl<T> Fn<()> for Memo<T>
where
    T: PartialEq + Clone + 'static,
{
    extern "rust-call" fn call(&self, _args: ()) -> Self::Output {
        self.get().clone()
    }
}
