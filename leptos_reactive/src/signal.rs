use crate::{Runtime, Scope, ScopeId, Source, Subscriber};
use serde::{Deserialize, Serialize};
use std::{any::Any, cell::RefCell, collections::HashSet, fmt::Debug, marker::PhantomData};

pub fn create_signal<T>(cx: Scope, value: T) -> (ReadSignal<T>, WriteSignal<T>)
where
    T: Clone + Debug,
{
    let state = SignalState::new(value);
    let id = cx.push_signal(state);

    let read = ReadSignal {
        runtime: cx.runtime,
        scope: cx.id,
        id,
        ty: PhantomData,
    };

    let write = WriteSignal {
        runtime: cx.runtime,
        scope: cx.id,
        id,
        ty: PhantomData,
    };

    (read, write)
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct ReadSignal<T>
where
    T: 'static,
{
    runtime: &'static Runtime,
    pub(crate) scope: ScopeId,
    pub(crate) id: SignalId,
    pub(crate) ty: PhantomData<T>,
}

impl<T> ReadSignal<T>
where
    T: Debug,
{
    pub fn get(&self) -> T
    where
        T: Clone,
    {
        self.with(T::clone)
    }

    pub fn with<U>(&self, f: impl Fn(&T) -> U) -> U {
        if let Some(running_subscriber) = self.runtime.running_effect() {
            self.runtime
                .any_effect(running_subscriber.0, |running_effect| {
                    self.add_subscriber(Subscriber(running_subscriber.0));
                    running_effect.subscribe_to(Source((self.scope, self.id)));
                });
        }

        // If transition is running, or contains this as a source, take from t_value
        if let Some(transition) = self.runtime.transition() {
            self.runtime
                .signal((self.scope, self.id), |signal_state: &SignalState<T>| {
                    if transition.running.get()
                        && transition.signals.borrow().contains(&(self.scope, self.id))
                    {
                        f(signal_state
                            .t_value
                            .borrow()
                            .as_ref()
                            .expect("read ReadSignal under transition, without any t_value"))
                    } else {
                        f(&signal_state.value.borrow())
                    }
                })
        } else {
            self.runtime
                .signal((self.scope, self.id), |signal_state: &SignalState<T>| {
                    (f)(&signal_state.value.borrow())
                })
        }
    }

    fn add_subscriber(&self, subscriber: Subscriber) {
        self.runtime
            .signal((self.scope, self.id), |signal_state: &SignalState<T>| {
                signal_state.subscribers.borrow_mut().insert(subscriber);
            })
    }
}

impl<T> Clone for ReadSignal<T> {
    fn clone(&self) -> Self {
        Self {
            runtime: self.runtime,
            scope: self.scope,
            id: self.id,
            ty: PhantomData,
        }
    }
}

impl<T> Copy for ReadSignal<T> {}

impl<T> FnOnce<()> for ReadSignal<T>
where
    T: Debug + Clone,
{
    type Output = T;

    extern "rust-call" fn call_once(self, _args: ()) -> Self::Output {
        self.get()
    }
}

impl<T> FnMut<()> for ReadSignal<T>
where
    T: Debug + Clone,
{
    extern "rust-call" fn call_mut(&mut self, _args: ()) -> Self::Output {
        self.get()
    }
}

impl<T> Fn<()> for ReadSignal<T>
where
    T: Debug + Clone,
{
    extern "rust-call" fn call(&self, _args: ()) -> Self::Output {
        self.get()
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct WriteSignal<T>
where
    T: Clone + 'static,
{
    runtime: &'static Runtime,
    pub(crate) scope: ScopeId,
    pub(crate) id: SignalId,
    pub(crate) ty: PhantomData<T>,
}

impl<T> WriteSignal<T>
where
    T: Clone + 'static,
{
    pub fn update(&self, f: impl FnOnce(&mut T)) {
        self.runtime
            .signal((self.scope, self.id), |signal_state: &SignalState<T>| {
                // update value
                if let Some(transition) = self.runtime.running_transition() {
                    let mut t_value = signal_state.t_value.borrow_mut();
                    if let Some(t_value) = &mut *t_value {
                        (f)(t_value);
                    } else {
                        // fork reality, using the old value as the basis for the transitional value
                        let mut forked = (*signal_state.value.borrow()).clone();
                        (f)(&mut forked);
                        *t_value = Some(forked);

                        // track this signal
                        transition
                            .signals
                            .borrow_mut()
                            .insert((self.scope, self.id));
                    }
                } else {
                    (f)(&mut *signal_state.value.borrow_mut());
                }

                // notify subscribers
                // if any of them are in scopes that have been disposed, unsubscribe
                let subs = { signal_state.subscribers.borrow().clone() };
                let mut dropped_subs = Vec::new();
                for subscriber in subs.iter() {
                    if subscriber.try_run(self.runtime).is_err() {
                        dropped_subs.push(subscriber);
                    }
                }
                for sub in dropped_subs {
                    signal_state.subscribers.borrow_mut().remove(sub);
                }
            })
    }
}

impl<T> Clone for WriteSignal<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self {
            runtime: self.runtime,
            scope: self.scope,
            id: self.id,
            ty: PhantomData,
        }
    }
}

impl<T> Copy for WriteSignal<T> where T: Clone {}

impl<T, F> FnOnce<(F,)> for WriteSignal<T>
where
    F: Fn(&mut T),
    T: Clone + 'static,
{
    type Output = ();

    extern "rust-call" fn call_once(self, args: (F,)) -> Self::Output {
        self.update(args.0)
    }
}

impl<T, F> FnMut<(F,)> for WriteSignal<T>
where
    F: Fn(&mut T),
    T: Clone + 'static,
{
    extern "rust-call" fn call_mut(&mut self, args: (F,)) -> Self::Output {
        self.update(args.0)
    }
}

impl<T, F> Fn<(F,)> for WriteSignal<T>
where
    F: Fn(&mut T),
    T: Clone + 'static,
{
    extern "rust-call" fn call(&self, args: (F,)) -> Self::Output {
        self.update(args.0)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct SignalId(pub(crate) usize);

//#[derive(Debug)]
pub(crate) struct SignalState<T> {
    value: RefCell<T>,
    t_value: RefCell<Option<T>>,
    subscribers: RefCell<HashSet<Subscriber>>,
}

impl<T> Debug for SignalState<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SignalState")
            .field("value", &*self.value.borrow())
            .field("t_value", &*self.t_value.borrow())
            .field("subscribers", &*self.subscribers.borrow())
            .finish()
    }
}

impl<T> SignalState<T>
where
    T: Debug,
{
    pub fn new(value: T) -> Self {
        Self {
            value: RefCell::new(value),
            t_value: Default::default(),
            subscribers: Default::default(),
        }
    }
}

pub(crate) trait AnySignal: Debug {
    fn unsubscribe(&self, subscriber: Subscriber);

    fn as_any(&self) -> &dyn Any;

    fn end_transition(&self, runtime: &'static Runtime);
}

impl<T> AnySignal for SignalState<T>
where
    T: Debug + 'static,
{
    fn unsubscribe(&self, subscriber: Subscriber) {
        self.subscribers.borrow_mut().remove(&subscriber);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn end_transition(&self, runtime: &'static Runtime) {
        let t_value = self.t_value.borrow_mut().take();

        if let Some(value) = t_value {
            *self.value.borrow_mut() = value;

            let subs = { self.subscribers.borrow().clone() };

            // run all its subscribers; if any of them are from scopes that have
            // been disposed, unsubscribe them
            let mut dropped_subs = Vec::new();
            for subscriber in subs.iter() {
                if subscriber.try_run(runtime).is_err() {
                    dropped_subs.push(subscriber);
                }
            }
            for sub in dropped_subs {
                self.subscribers.borrow_mut().remove(sub);
            }
        }
    }
}
