use std::{
    cell::{Ref, RefCell},
    collections::HashSet,
    rc::{Rc, Weak},
};

use crate::EffectInner;

use super::{root_context::RootContext, EffectDependency};

pub struct ReadSignal<T: 'static> {
    pub(crate) stack: &'static RootContext,
    pub(crate) inner: Rc<SignalState<T>>,
}

impl<T> Clone for ReadSignal<T>
where
    T: 'static,
{
    fn clone(&self) -> Self {
        Self {
            stack: self.stack,
            inner: Rc::clone(&self.inner),
        }
    }
}

impl<T> PartialEq for ReadSignal<T> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}

impl<T> Eq for ReadSignal<T> {}

impl<T: 'static> ReadSignal<T> {
    pub fn get(&self) -> ReadSignalRef<T> {
        if let Some(running_effect) = self.stack.stack.borrow().last() {
            if let Some(running_effect) = running_effect.upgrade() {
                // add the Effect to the Signal's subscriptions
                self.add_subscriber(running_effect.clone());

                // add the Signal to the Effect's dependencies
                running_effect
                    .add_dependency(Rc::downgrade(&self.inner) as Weak<dyn EffectDependency>);
            }
        }

        self.value()
    }

    pub fn get_untracked(&self) -> ReadSignalRef<T> {
        self.value()
    }

    fn value(&self) -> ReadSignalRef<T> {
        ReadSignalRef {
            guard: self.inner.value.borrow(),
        }
    }

    fn add_subscriber(&self, subscriber: Rc<EffectInner>) {
        self.inner.subscriptions.borrow_mut().insert(subscriber);
    }
}

impl<T> EffectDependency for SignalState<T> {
    fn unsubscribe(&self, effect: Rc<EffectInner>) {
        self.subscriptions.borrow_mut().remove(&effect);
    }
}

use std::ops::Deref;

pub struct ReadSignalRef<'a, T> {
    guard: Ref<'a, T>,
}

impl<'a, T> Deref for ReadSignalRef<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.guard
    }
}

impl<T> std::fmt::Debug for ReadSignal<T>
where
    T: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Signal")
            .field("value", &self.value())
            .finish()
    }
}

impl<'a, T> FnOnce<()> for &'a ReadSignal<T> {
    type Output = ReadSignalRef<'a, T>;

    extern "rust-call" fn call_once(self, _args: ()) -> Self::Output {
        self.get()
    }
}

impl<'a, T> FnMut<()> for &'a ReadSignal<T> {
    extern "rust-call" fn call_mut(&mut self, _args: ()) -> Self::Output {
        self.get()
    }
}

impl<'a, T> Fn<()> for &'a ReadSignal<T> {
    extern "rust-call" fn call(&self, _args: ()) -> Self::Output {
        self.get()
    }
}

pub struct SignalState<T> {
    pub(crate) value: RefCell<T>,
    pub(crate) subscriptions: RefCell<HashSet<Rc<EffectInner>>>,
}

pub struct WriteSignal<T> {
    pub(crate) inner: Weak<SignalState<T>>,
}

impl<T> Clone for WriteSignal<T>
where
    T: 'static,
{
    fn clone(&self) -> Self {
        Self {
            inner: Weak::clone(&self.inner),
        }
    }
}

impl<T> WriteSignal<T> {
    pub fn update(&self, update_fn: impl FnOnce(&mut T)) {
        if let Some(inner) = self.inner.upgrade() {
            (update_fn)(&mut inner.value.borrow_mut());
            for subscription in inner.subscriptions.take().iter() {
                subscription.execute(Rc::downgrade(&subscription))
            }
        } else {
        }
    }
}

impl<T, F> FnOnce<(F,)> for WriteSignal<T>
where
    F: Fn(&mut T),
{
    type Output = ();

    extern "rust-call" fn call_once(self, args: (F,)) -> Self::Output {
        self.update(args.0)
    }
}

impl<T, F> FnMut<(F,)> for WriteSignal<T>
where
    F: Fn(&mut T),
{
    extern "rust-call" fn call_mut(&mut self, args: (F,)) -> Self::Output {
        self.update(args.0)
    }
}

impl<T, F> Fn<(F,)> for WriteSignal<T>
where
    F: Fn(&mut T),
{
    extern "rust-call" fn call(&self, args: (F,)) -> Self::Output {
        self.update(args.0)
    }
}

impl<T> PartialEq for WriteSignal<T> {
    fn eq(&self, other: &Self) -> bool {
        Weak::ptr_eq(&self.inner, &other.inner)
    }
}

impl<T> Eq for WriteSignal<T> {}

impl<'a, T> std::fmt::Display for ReadSignalRef<'a, T>
where
    T: std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.guard.fmt(f)
    }
}

impl<'a, T> std::fmt::Debug for ReadSignalRef<'a, T>
where
    T: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.guard.fmt(f)
    }
}
