use std::{
    cell::RefCell,
    collections::HashSet,
    fmt::Debug,
    marker::PhantomData,
    rc::{Rc, Weak},
};

use crate::{ObserverLink, Scope, ScopeId, SignalId, System, WaitingCount};

pub fn create_signal<T>(cx: Scope, value: T) -> (ReadSignal<T>, WriteSignal<T>)
where
    T: Clone + Debug,
{
    let state = Rc::new(SignalState::new(cx.system, value));
    let id = cx.push_signal(state);

    let read = ReadSignal {
        system: cx.system,
        scope: cx.id,
        id,
        ty: PhantomData,
    };

    let write = WriteSignal {
        system: cx.system,
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
    pub(crate) system: &'static System,
    pub(crate) scope: ScopeId,
    pub(crate) id: SignalId,
    pub(crate) ty: PhantomData<T>,
}

impl<T> ReadSignal<T> {
    pub fn get(self) -> T
    where
        T: Clone,
    {
        self.with(|val| val.clone())
    }

    pub fn with<U>(self, f: impl Fn(&T) -> U) -> U {
        self.system
            .signal((self.scope, self.id), |state| state.with(f))
    }
}

impl<T> Clone for ReadSignal<T> {
    fn clone(&self) -> Self {
        Self {
            system: self.system,
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
    system: &'static System,
    pub(crate) scope: ScopeId,
    pub(crate) id: SignalId,
    pub(crate) ty: PhantomData<T>,
}

impl<T> WriteSignal<T>
where
    T: Clone,
{
    pub fn update(self, f: impl FnOnce(&mut T)) {
        self.system
            .signal((self.scope, self.id), move |state| state.update(f))
    }
}

impl<T> Clone for WriteSignal<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self {
            system: self.system,
            scope: self.scope,
            id: self.id,
            ty: PhantomData,
        }
    }
}

impl<T> Copy for WriteSignal<T> where T: Clone {}

impl<T, F> FnOnce<(F,)> for WriteSignal<T>
where
    F: Fn(&mut T) + 'static,
    T: Clone + 'static,
{
    type Output = ();

    extern "rust-call" fn call_once(self, args: (F,)) -> Self::Output {
        self.update(args.0)
    }
}

impl<T, F> FnMut<(F,)> for WriteSignal<T>
where
    F: Fn(&mut T) + 'static,
    T: Clone + 'static,
{
    extern "rust-call" fn call_mut(&mut self, args: (F,)) -> Self::Output {
        self.update(args.0)
    }
}

impl<T, F> Fn<(F,)> for WriteSignal<T>
where
    F: Fn(&mut T) + 'static,
    T: Clone + 'static,
{
    extern "rust-call" fn call(&self, args: (F,)) -> Self::Output {
        self.update(args.0)
    }
}

pub(crate) struct SignalState<T>
where
    T: 'static,
{
    system: &'static System,
    parent: RefCell<Option<ObserverLink>>,
    value: RefCell<T>,
    t_value: RefCell<Option<T>>,
    subscribers: RefCell<HashSet<ObserverLink>>,
}

impl<T> SignalState<T>
where
    T: 'static,
{
    pub fn new(system: &'static System, value: T) -> Self {
        Self {
            system,
            parent: Default::default(),
            value: RefCell::new(value),
            t_value: Default::default(),
            subscribers: Default::default(),
        }
    }

    pub fn with<U>(self: Rc<Self>, f: impl Fn(&T) -> U) -> U {
        let observer = self.system.observer();

        if let Some(observer) = observer.and_then(|o| o.0.upgrade()) {
            // register the signal as a dependency, if we are tracking and the parent is a Observer
            // (rather than, for example, a root, which does not track)
            if self.system.tracking() {
                self.add_observer(ObserverLink(Rc::downgrade(&observer)));
                observer.add_signal(Rc::clone(&self).as_observable());
            }
        }

        // if there's a stale parent, it needs to be refreshed
        // this may cause other upstream Observers to refresh
        if let Some(parent) = self.parent.borrow().clone() && parent.is_waiting() {
            parent.update();
        }

        f(&self.value.borrow())
    }

    pub fn update(self: Rc<Self>, f: impl FnOnce(&mut T)) {
        if self.system.batching() {
            let this = Rc::clone(&self);
            // TODO transition
            self.system
                .add_to_batch(move || (f)(&mut *this.value.borrow_mut()));
        } else {
            // TODO transition
            (f)(&mut *self.value.borrow_mut());

            // notify observers that there's a new value
            self.stale(WaitingCount::Unchanged, true);
        }
    }

    pub fn add_observer(&self, observer: ObserverLink) {
        self.subscribers.borrow_mut().insert(observer);
    }

    pub fn as_observable(self: Rc<Self>) -> ObservableLink {
        ObservableLink(Rc::downgrade(&self) as Weak<dyn Observable>)
    }

    pub fn stale(&self, delta: WaitingCount, fresh: bool) {
        let subs = { self.subscribers.borrow().clone() };
        for observer in subs {
            observer.stale(delta, fresh);
        }
    }
}

pub(crate) trait Observable {
    fn unsubscribe(&self, observer: ObserverLink);
}

impl<T> Observable for SignalState<T> {
    fn unsubscribe(&self, observer: ObserverLink) {
        self.subscribers.borrow_mut().remove(&observer);
    }
}
pub struct ObservableLink(pub(crate) Weak<dyn Observable>);

impl ObservableLink {
    pub(crate) fn unsubscribe(&self, observer: ObserverLink) {
        if let Some(observable) = self.0.upgrade() {
            observable.unsubscribe(observer);
        }
    }
}

impl std::hash::Hash for ObservableLink {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::ptr::hash(&self.0, state);
    }
}

impl PartialEq for ObservableLink {
    fn eq(&self, other: &Self) -> bool {
        Weak::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for ObservableLink {}
