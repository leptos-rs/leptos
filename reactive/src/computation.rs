use std::{
    cell::{Cell, RefCell},
    collections::HashSet,
    fmt::Debug,
    rc::{Rc, Weak},
};

use crate::{ObservableLink, Scope, SignalState, System};

pub fn create_render_effect<T>(cx: Scope, f: impl FnMut(Option<T>) -> T + 'static)
where
    T: Debug + Clone + 'static,
{
    let c = Rc::new(Computation::new(cx.system, f, None));
    Rc::clone(&c).run();
    cx.push_computation(c);
}

pub fn create_effect<T>(cx: Scope, f: impl FnMut(Option<T>) -> T + 'static)
where
    T: Debug + Clone + 'static,
{
    let c = Rc::new(Computation::new(cx.system, f, None));
    Rc::clone(&c).run();
    cx.push_computation(c);
}

pub struct Computation<T>
where
    T: Clone + 'static,
{
    system: &'static System,
    // function to execute when dependencies change
    f: Box<RefCell<dyn FnMut(Option<T>) -> T>>,
    // previous value of the effect
    value: RefCell<Option<T>>,
    // internal signal holding value of the function
    signal: Option<Rc<SignalState<Option<T>>>>,
    // how many of our dependencies have changed since the last time we ran the function?
    waiting: Cell<u32>,
    // did something actually change in one of our dependencies?
    fresh: Cell<bool>,
    // all the signals this computation depends on
    signals: RefCell<HashSet<ObservableLink>>,
    // custom cleanup functions to call
    cleanups: RefCell<Vec<Box<dyn FnOnce()>>>,
}

impl<T> Computation<T>
where
    T: Clone,
{
    pub(crate) fn new(
        system: &'static System,
        f: impl FnMut(Option<T>) -> T + 'static,
        signal: Option<Rc<SignalState<Option<T>>>>,
    ) -> Self {
        Self {
            system,
            f: Box::new(RefCell::new(f)),
            value: Default::default(),
            signal,
            waiting: Cell::new(0),
            fresh: Cell::new(false),
            signals: Default::default(),
            cleanups: Default::default(),
        }
    }
}

impl<T> Observer for Computation<T>
where
    T: Clone + 'static,
{
    fn run(self: Rc<Self>) {
        // clean up dependencies and cleanups
        (Rc::clone(&self)).cleanup();

        // run the computation
        self.system.wrap(
            {
                let this = self.clone();
                move || {
                    let curr = { this.value.borrow_mut().take() };
                    let v = { (this.f.borrow_mut())(curr) };
                    *this.value.borrow_mut() = Some(v);
                }
            },
            Some(ObserverLink(Rc::downgrade(&self) as Weak<dyn Observer>)),
            true,
        )
    }

    fn update(self: Rc<Self>) {
        // reset waiting, in case this is a force-refresh
        self.waiting.set(0);

        // run the effect
        Rc::clone(&self).run();

        // set the signal, if there is one
        if let Some(signal) = &self.signal {
            Rc::clone(signal).update(move |n| *n = self.value.borrow().clone());
        }
    }

    fn add_signal(self: Rc<Self>, signal: ObservableLink) {
        self.signals.borrow_mut().insert(signal);
    }

    fn is_waiting(&self) -> bool {
        self.waiting.get() > 0
    }

    fn stale(self: Rc<Self>, increment: WaitingCount, fresh: bool) {
        let waiting = self.waiting.get();

        // If waiting is already 0 but change is -1, the computation has been force-refreshed
        if waiting == 0 && increment == WaitingCount::Decrement {
            return;
        }
        // mark computations that depend on the internal signal stale
        if waiting == 0 && increment == WaitingCount::Increment && let Some(signal) = &self.signal {
            // we don't mark it fresh, because we don't know for a fact that something has changed yet
            signal.stale(WaitingCount::Increment, false);
        }

        match increment {
            WaitingCount::Decrement => {
                if waiting > 0 {
                    self.waiting.set(waiting - 1);
                }
            }
            WaitingCount::Unchanged => {}
            WaitingCount::Increment => {
                self.waiting.set(waiting + 1);
            }
        }

        self.fresh.set(self.fresh.get() || fresh);

        // are we still waiting?
        let waiting = self.waiting.get();
        if waiting == 0 {
            if self.fresh.get() {
                Rc::clone(&self).update();
            }

            // mark any computations that depend on us as not stale
            if let Some(signal) = &self.signal {
                signal.stale(WaitingCount::Decrement, false);
            }
        }
    }
}

impl<T> Computation<T>
where
    T: Clone + 'static,
{
    fn cleanup(self: Rc<Self>) {
        for source in self.signals.borrow().iter() {
            source.unsubscribe(ObserverLink(Rc::downgrade(&self) as Weak<dyn Observer>));
        }

        for cleanup in self.cleanups.take() {
            cleanup();
        }
    }
}

pub trait Observer {
    fn run(self: Rc<Self>);

    fn add_signal(self: Rc<Self>, signal: ObservableLink);

    fn is_waiting(&self) -> bool;

    fn update(self: Rc<Self>);

    fn stale(self: Rc<Self>, increment: WaitingCount, fresh: bool);
}

#[derive(Clone)]
pub struct ObserverLink(pub(crate) Weak<dyn Observer>);

impl std::fmt::Debug for ObserverLink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ObserverLink").finish()
    }
}

impl ObserverLink {
    pub(crate) fn is_waiting(&self) -> bool {
        if let Some(c) = self.0.upgrade() {
            c.is_waiting()
        } else {
            false
        }
    }

    pub(crate) fn update(&self) {
        if let Some(c) = self.0.upgrade() {
            c.update()
        }
    }

    pub(crate) fn stale(&self, increment: WaitingCount, fresh: bool) {
        if let Some(c) = self.0.upgrade() {
            c.stale(increment, fresh);
        }
    }
}

impl std::hash::Hash for ObserverLink {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::ptr::hash(&self.0, state);
    }
}

impl PartialEq for ObserverLink {
    fn eq(&self, other: &Self) -> bool {
        Weak::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for ObserverLink {}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum WaitingCount {
    Decrement,
    Unchanged,
    Increment,
}
