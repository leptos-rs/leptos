use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};

use crate::{
    owner::Owner, Accessor, AnyComputation, AnySignalState, Observer, SignalState, TransitionState,
    WeakSignalState,
};

//use crate::{EffectInner, ReadSignal, TransitionState, WriteSignal};

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) enum State {
    Stale,
    Pending,
    NotPending,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct Update(pub(crate) usize);

pub struct RootContext {
    pub(crate) transition_pending: SignalState<bool>,
    pub(crate) owner: RefCell<Option<Observer>>,
    pub(crate) transition: RefCell<Option<TransitionState>>,
    pub(crate) listener: RefCell<Option<Observer>>,
    pub(crate) pending: RefCell<Option<Vec<WeakSignalState>>>,
    pub(crate) updates: RefCell<Option<Vec<Observer>>>,
    pub(crate) effects: RefCell<Option<Vec<Observer>>>,
    exec_count: Cell<Update>,
}

impl RootContext {
    pub fn new() -> Self {
        Self {
            transition_pending: SignalState::new(false),
            owner: Default::default(),
            transition: Default::default(),
            listener: Default::default(),
            pending: Default::default(),
            updates: Default::default(),
            effects: Default::default(),
            exec_count: Cell::new(Update(0)),
        }
    }

    pub(crate) fn untrack<T>(&self, f: impl FnOnce() -> T) -> T {
        let listener = { self.listener.borrow_mut().take() };
        let untracked_result = f();
        *self.listener.borrow_mut() = listener;
        untracked_result
    }

    pub(crate) fn running_transition(&self) -> Option<TransitionState> {
        self.transition
            .borrow()
            .as_ref()
            .filter(|t| t.inner.borrow().running)
            .cloned()
    }

    pub(crate) fn push_update(&self, update: Observer) {
        let mut updates = self.updates.borrow_mut();
        match &mut *updates {
            Some(updates) => updates.push(update),
            None => *updates = Some(vec![update]),
        }
    }

    pub(crate) fn push_effect(&self, effect: Observer) {
        let mut effects = self.effects.borrow_mut();
        match &mut *effects {
            Some(effects) => effects.push(effect),
            None => *effects = Some(vec![effect]),
        }
    }

    pub(crate) fn take_updates(&self) -> Option<Vec<Observer>> {
        self.updates.borrow_mut().take()
    }

    pub(crate) fn set_updates(&self, updates: Option<Vec<Observer>>) {
        *self.updates.borrow_mut() = updates;
    }

    pub(crate) fn run_updates<T>(&self, update_fn: impl Fn() -> T, init: bool) -> T {
        let updates = { self.updates.borrow().clone() };
        /* if updates.is_some() {
            if !self.infinite_loop() {
                update_fn()
            } else {
                panic!()
            }
        } else { */
        let mut wait = false;
        if !init {
            *self.updates.borrow_mut() = Some(vec![]);
        }
        if self.effects.borrow().is_some() {
            wait = true;
        } else {
            *self.effects.borrow_mut() = Some(vec![]);
        }
        self.exec_count.set(Update(self.exec_count.get().0 + 1));

        let res = { update_fn() };
        self.complete_updates(wait);
        res
        //}
    }

    fn complete_updates(&self, wait: bool) {
        if let Some(updates) = { self.updates.borrow_mut().take() } {
            self.run_queue(updates);
        }

        if wait {
            return;
        } else {
            if let Some(transition) = self.running_transition() {
                todo!()
            }

            if let Some(effects) = self.effects.take() {
                if !effects.is_empty() {
                    self.batch(|| self.run_effects(effects));
                }
            }
        }
    }

    fn run_queue(&self, updates: Vec<Observer>) {
        for update in updates {
            self.run_top(update);
        }
    }

    fn run_top(&self, update: Observer) {
        if let Some(node) = update.upgrade() {
            let running_transition = self.running_transition().is_some();
            if (!running_transition && node.state().is_none())
                || (running_transition && node.t_state().is_none())
            {
                return;
            }

            if (!running_transition && node.state() == Some(State::Pending))
                || (running_transition && node.t_state() == Some(State::Pending))
            {
                return self.look_upstream(node.as_ref(), None);
            }

            // TODO suspense check
            let mut ancestors = vec![Rc::clone(&node)];
            let node = Some(node);
            loop {
                let node = node.as_ref().and_then(|node| node.owner());
                match node {
                    Some(node) => {
                        if let Some(node) = node.upgrade() {
                            match node.updated_at() {
                                Some(updated) => {
                                    if updated < self.exec_count.get() {
                                        if running_transition
                                            && self
                                                .running_transition()
                                                .unwrap()
                                                .disposed_contains(&Observer(Rc::downgrade(&node)))
                                        {
                                            return;
                                        } else if (!running_transition && node.state().is_some())
                                            || (running_transition && node.t_state().is_some())
                                        {
                                            ancestors.push(node);
                                        }
                                    } else {
                                        break;
                                    }
                                }
                                None => break,
                            }
                        } else {
                            break;
                        }
                    }
                    None => break,
                }
            }

            for node in ancestors.iter().rev() {
                if running_transition {
                    todo!()
                } else if (!running_transition && node.state() == Some(State::Stale))
                    || (running_transition && node.t_state() == Some(State::Stale))
                {
                    self.update_computation(Rc::downgrade(&node));
                } else if (!running_transition && node.state() == Some(State::Pending))
                    || (running_transition && node.t_state() == Some(State::Pending))
                {
                    let updates = self.take_updates();
                    self.look_upstream(node.as_ref(), Some(ancestors[0].as_ref()));
                    *self.updates.borrow_mut() = updates;
                }
            }
        }
    }

    fn batch<T>(&self, batch_fn: impl FnOnce() -> T) -> T {
        if self.pending.borrow().is_some() {
            batch_fn()
        } else {
            *self.pending.borrow_mut() = Some(vec![]);
            let result = batch_fn();
            let q = self.pending.borrow_mut().take().unwrap_or_default();

            // TODO
            self.run_updates(
                move || {
                    /* for data in &q {
                        if let Some(data) = data.upgrade() {}
                    } */
                },
                false,
            );

            result
        }
    }

    fn run_effects(&self, effects: Vec<Observer>) {
        self.run_queue(effects);
    }

    pub(crate) fn update_computation(&self, node: Weak<dyn AnyComputation>) {
        let weak_node = node.clone();
        if let Some(node) = node.upgrade() {
            self.clean_node(node.as_ref());

            let owner = { self.owner.borrow_mut().take() };
            let listener = { self.listener.borrow_mut().take() };
            let time = { self.exec_count.get() };
            {
                *self.owner.borrow_mut() = Some(Observer(weak_node.clone()));
                *self.listener.borrow_mut() = Some(Observer(weak_node.clone()));
            }

            node.clone().run_computation(time);

            if let Some(_) = self.running_transition() {
                todo!()
            }

            {
                *self.listener.borrow_mut() = listener;
                *self.owner.borrow_mut() = owner;
            }
        }
    }

    pub(crate) fn look_upstream(
        &self,
        node: &dyn AnyComputation,
        ignore: Option<&dyn AnyComputation>,
    ) {
        let running_transition = self.running_transition().is_some();
        if running_transition {
            node.set_t_state(None);
        } else {
            node.set_state(None);
        }

        let sources = node.sources();
        for source in sources.iter() {
            if let Some(source) = source.upgrade() {
                if let Some(source) = source.as_computation() {
                    if let Some(source) = source.upgrade() {
                        if ((!running_transition && source.state() == Some(State::Stale))
                            || (running_transition && source.t_state() == Some(State::Stale)))
                            && Some(source.as_ref()) != ignore
                        {
                            self.run_top(Observer(Rc::downgrade(&source)));
                        }
                    }
                }
            }
        }
    }

    fn clean_node(&self, node: &dyn AnyComputation) {
        while let Some((source, index)) = node.pop_source_and_slot() {
            if let Some(source) = source.upgrade() {
                while let Some((n, s)) = source.pop_observer_and_slot() {
                    // TODO
                }
            }
        }

        let running_transition = self.running_transition();

        // TODO transition

        // TODO node.owned

        // TODO cleanups
        /* for cleanup in node.take_cleanups() {
            cleanup()
        } */

        if running_transition.is_some() {
            todo!()
        } else {
            node.set_state(None);
        }
    }

    pub(crate) fn infinite_loop(&self) -> bool {
        if self.updates.borrow().as_ref().map(|u| u.len()).unwrap_or(0) > 10000 {
            log::error!("potential infinite loop detected");
            true
        } else {
            false
        }
    }
}

impl Default for RootContext {
    fn default() -> Self {
        Self::new()
    }
}
