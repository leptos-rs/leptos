use std::rc::{Rc, Weak};

use crate::{AnyComputation, ReadSignalRef, RootContext, SignalStateInner};

pub(crate) struct Observer(pub(crate) Weak<dyn AnyComputation>);

impl Clone for Observer {
    fn clone(&self) -> Self {
        Self(Weak::clone(&self.0))
    }
}

impl Observer {
    pub(crate) fn upgrade(&self) -> Option<Rc<dyn AnyComputation>> {
        self.0.upgrade()
    }
}

impl std::hash::Hash for Observer {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::ptr::hash(&self.0, state)
    }
}

impl PartialEq for Observer {
    fn eq(&self, other: &Self) -> bool {
        Weak::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for Observer {}
pub(crate) trait Accessor<T>
where
    T: 'static,
{
    fn context(&self) -> &'static RootContext;

    fn is_stale_computation(&self) -> bool;

    fn update_computation(&self);

    fn value(&self) -> ReadSignalRef<T>;

    fn t_value_unchecked(&self) -> ReadSignalRef<T>;

    fn as_weak_signal_state(&self) -> WeakSignalState;

    fn observers_len(&self) -> usize;

    fn push_observer(&self, observer: Observer);

    fn push_observer_slot(&self, observer_slot: usize);

    fn read(&self) -> ReadSignalRef<T> {
        let context = self.context();
        let running_transition = context.running_transition();

        // if this is a Memo (i.e., has sources)
        // and either
        //   a) there's no running transition and this has a state, or
        //   b) there's a running transition and this has a t_state
        // then update the Memo as needed
        if self.is_stale_computation() {
            self.update_computation();
        }

        // if there is a Listener (i.e., we're running inside an Effect)
        {
            if let Some(listener) = &*context.listener.borrow() {
                if let Some(listener) = listener.upgrade() {
                    let s_slot = self.observers_len();

                    // register self with listener
                    listener.push_source(self.as_weak_signal_state());
                    listener.push_source_slot(s_slot);

                    // register listener with self
                    self.push_observer(Observer(Rc::downgrade(&listener)));
                    self.push_observer_slot(listener.sources_len() - 1);
                }
            }
        }

        // if there's a running transition, and it has this as a source
        // then return the transition value
        if running_transition
            .map(|transition| transition.contains_source(&self.as_weak_signal_state()))
            .unwrap_or(false)
        {
            self.t_value_unchecked()
        }
        // otherwise just return the value
        else {
            self.value()
        }
    }
}

pub(crate) trait AnySignalState {
    fn as_computation(self: Rc<Self>) -> Option<Observer>;

    fn observers(&self) -> Vec<Observer>;

    fn pop_observer_and_slot(&self) -> Option<(Observer, usize)>;
}

impl<T> AnySignalState for SignalStateInner<T> {
    fn as_computation(self: Rc<Self>) -> Option<Observer> {
        None
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

pub struct WeakSignalState(pub(crate) Weak<dyn AnySignalState>);

impl std::hash::Hash for WeakSignalState {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::ptr::hash(&self.0, state)
    }
}

impl PartialEq for WeakSignalState {
    fn eq(&self, other: &Self) -> bool {
        Weak::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for WeakSignalState {}

impl Clone for WeakSignalState {
    fn clone(&self) -> Self {
        Self(Weak::clone(&self.0))
    }
}

impl WeakSignalState {
    pub(crate) fn upgrade(&self) -> Option<Rc<dyn AnySignalState>> {
        self.0.upgrade()
    }
}
