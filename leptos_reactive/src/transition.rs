use std::{cell::RefCell, collections::HashSet, pin::Pin, rc::Rc};

use futures::{
    channel::oneshot::{Canceled, Sender},
    Future,
};

use crate::{BoundedScope, Effect, EffectDependency, ReadSignal, SuspenseContext};

impl<'a, 'b> BoundedScope<'a, 'b> {
    /* pub fn use_transition(self) -> (ReadSignal<bool>, impl Fn()) {
        todo!()
        /* if let Some(transition) = self.inner.root_context.transition {
            transition
        } */
    } */

    /*     pub fn start_transition(self, f: impl Fn()) {
        // If a transition is already running, run this function
        // and then return when the existing transition is done
        if let Some(transition) = self.inner.root_context.transition.take() {
            if transition.running {
                f();
            }
        }

        // Otherwise, if we're inside a Suspense context, create a transition and run it
        if self.use_context::<SuspenseContext>().is_some() {
            let t = TransitionState {
                running: true,
                sources: Default::default(),
                effects: Default::default(),
                queue: Default::default(),
                pending_resources: Default::default(),
            };
            *self.inner.root_context.transition.borrow_mut() = Some(t);
            f();
        }
    } */
}

pub struct TransitionState {
    inner: Rc<RefCell<TransitionStateInner>>,
    /*   sources: Set<SignalState<any>>;
    effects: Computation<any>[];
    promises: Set<Promise<any>>;
    disposed: Set<Computation<any>>;
    queue: Set<Computation<any>>;
    scheduler?: (fn: () => void) => unknown;
    running: boolean;
    done?: Promise<void>;
    resolve?: () => void; */
}

pub(crate) struct TransitionStateInner {
    pub(crate) running: bool,
    pub(crate) sources: HashSet<Box<dyn EffectDependency>>,
    pub(crate) effects: Vec<Effect>,
    pub(crate) queue: HashSet<Effect>,
    pub(crate) pending_resources: Option<HashSet<ReadSignal<usize>>>,
}

impl TransitionState {
    pub fn pending(&self) -> bool {
        match &self.inner.borrow().pending_resources {
            Some(suspenses) => {
                suspenses
                    .iter()
                    .map(|pending| *pending.get())
                    .sum::<usize>()
                    > 0
            }
            None => false,
        }
    }

    pub fn complete(&self) -> bool {
        todo!()
        /* match &self.pending_resources {
            Some(_) => !self.pending(),
            None => false,
        } */
    }
}
