use std::{
    borrow::Borrow,
    cell::{Ref, RefCell},
    rc::{Rc, Weak},
};

use crate::{Accessor, AnySignalState, Observer, RootContext, State, WeakSignalState};

pub struct ReadSignal<T> {
    pub(crate) root_context: &'static RootContext,
    pub(crate) state: Rc<SignalStateInner<T>>,
}

impl<T> ReadSignal<T>
where
    T: 'static,
{
    pub fn get(&self) -> ReadSignalRef<T> {
        self.read()
    }
}

impl<T> Clone for ReadSignal<T> {
    fn clone(&self) -> Self {
        Self {
            root_context: self.root_context,
            state: Rc::clone(&self.state),
        }
    }
}

impl<T> FnOnce<()> for ReadSignal<T>
where
    T: Clone + 'static,
{
    type Output = T;

    extern "rust-call" fn call_once(self, _args: ()) -> Self::Output {
        self.get().clone()
    }
}

impl<'a, T> FnMut<()> for ReadSignal<T>
where
    T: Clone + 'static,
{
    extern "rust-call" fn call_mut(&mut self, _args: ()) -> Self::Output {
        self.get().clone()
    }
}

impl<'a, T> Fn<()> for ReadSignal<T>
where
    T: Clone + 'static,
{
    extern "rust-call" fn call(&self, _args: ()) -> Self::Output {
        self.get().clone()
    }
}

pub struct WriteSignal<T> {
    pub(crate) root_context: &'static RootContext,
    pub(crate) state: Weak<SignalStateInner<T>>,
}

impl<T> Clone for WriteSignal<T> {
    fn clone(&self) -> Self {
        Self {
            root_context: self.root_context,
            state: Weak::clone(&self.state),
        }
    }
}

impl<T, F> FnOnce<(F,)> for WriteSignal<T>
where
    F: Fn(&mut T),
    T: PartialEq + 'static,
{
    type Output = ();

    extern "rust-call" fn call_once(self, args: (F,)) -> Self::Output {
        self.update(args.0)
    }
}

impl<T, F> FnMut<(F,)> for WriteSignal<T>
where
    F: Fn(&mut T),
    T: PartialEq + 'static,
{
    extern "rust-call" fn call_mut(&mut self, args: (F,)) -> Self::Output {
        self.update(args.0)
    }
}

impl<T, F> Fn<(F,)> for WriteSignal<T>
where
    F: Fn(&mut T),
    T: PartialEq + 'static,
{
    extern "rust-call" fn call(&self, args: (F,)) -> Self::Output {
        self.update(args.0)
    }
}

pub struct ReadSignalRef<'a, T> {
    pub(crate) guard: Ref<'a, T>,
}

impl<'a, T> ReadSignalRef<'a, T> {
    pub fn guard(&self) -> &Ref<'a, T> {
        &self.guard
    }
}

impl<'a, T> std::ops::Deref for ReadSignalRef<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.guard
    }
}

pub struct SignalState<T> {
    pub(crate) inner: Rc<SignalStateInner<T>>,
}

impl<T> SignalState<T> {
    pub(crate) fn new(value: T) -> Self {
        Self {
            inner: Rc::new(SignalStateInner::new(value)),
        }
    }
}

impl<T> std::hash::Hash for SignalState<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::ptr::hash(&self.inner, state)
    }
}

impl<T> PartialEq for SignalState<T> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}

#[derive(PartialEq)]
pub(crate) enum Pending<T> {
    NotPending,
    Pending(T),
}

impl<T> Eq for SignalState<T> {}

pub struct SignalStateInner<T> {
    pub(crate) value: RefCell<T>,
    pub(crate) t_value: RefCell<Option<T>>,
    pub(crate) observers: RefCell<Vec<Observer>>,
    pub(crate) observer_slots: RefCell<Vec<usize>>,
    pub(crate) pending: RefCell<Pending<T>>,
}

impl<T> SignalStateInner<T> {
    pub(crate) fn new(value: T) -> Self {
        Self {
            value: RefCell::new(value),
            t_value: RefCell::new(None),
            observers: Default::default(),
            observer_slots: Default::default(),
            pending: RefCell::new(Pending::NotPending),
        }
    }
}

// Read Signal
impl<T> Accessor<T> for ReadSignal<T>
where
    T: 'static,
{
    fn context(&self) -> &'static RootContext {
        self.root_context
    }

    fn is_stale_computation(&self) -> bool {
        false
    }

    fn update_computation(&self) {}

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
        self.state.observers.borrow_mut().push(observer);
    }

    fn push_observer_slot(&self, observer_slot: usize) {
        self.state.observer_slots.borrow_mut().push(observer_slot);
    }
}

// Write Signal
impl<T> WriteSignal<T>
where
    T: 'static + PartialEq,
{
    pub fn update(&self, update_fn: impl FnOnce(&mut T)) {
        if let Some(state) = self.state.upgrade() {
            // calculate new value
            {
                if self.root_context.running_transition().is_some() {
                    match &mut *state.pending.borrow_mut() {
                        Pending::NotPending => match &mut *state.t_value.borrow_mut() {
                            Some(value) => (update_fn)(value),
                            None => {
                                log::error!("WriteSignal::update no t_value to update...")
                            }
                        },
                        Pending::Pending(pending) => (update_fn)(pending),
                    }
                } else {
                    (update_fn)(&mut state.value.borrow_mut());
                }
            }

            // push this onto the list of pending nodes, if it exists
            if self.root_context.pending.borrow().is_some() {
                if *state.pending.borrow() == Pending::NotPending {
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
                    if transition.contains_source(&self.as_weak_signal_state())
                        && state.t_value.borrow().as_ref() == Some(&state.value.borrow())
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
                self.root_context.run_updates(
                    || {
                        for observer in state.observers.borrow().clone() {
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
                                    || (!transition_running
                                        && (observer.state().is_none()
                                            || observer.state() == Some(State::Stale)))
                                {
                                    if observer.pure() {
                                        // TODO
                                        /* self.root_context
                                        .push_update(Observer(Rc::downgrade(&observer))); */
                                    } else {
                                        self.root_context
                                            .push_effect(Observer(Rc::downgrade(&observer)));
                                    }

                                    // TODO for Memo: markDownstream
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

    fn as_weak_signal_state(&self) -> WeakSignalState {
        WeakSignalState(self.state.clone())
    }
}

impl<T> PartialEq for ReadSignal<T> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.state, &other.state)
    }
}

impl<T> Eq for ReadSignal<T> {}

impl<T> PartialEq for WriteSignal<T> {
    fn eq(&self, other: &Self) -> bool {
        Weak::ptr_eq(&self.state, &other.state)
    }
}

impl<T> Eq for WriteSignal<T> {}
