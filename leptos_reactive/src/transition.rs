use std::{
    cell::{Cell, RefCell},
    collections::HashSet,
    rc::Rc,
};

use crate::{
    runtime::Runtime, spawn::queue_microtask, EffectId, MemoId, ReadSignal, Scope, ScopeId,
    SignalId, WriteSignal,
};

impl Scope {
    pub fn use_transition(self) -> Transition {
        let (pending, set_pending) = self.create_signal(false);
        Transition {
            runtime: self.runtime,
            scope: self,
            pending,
            set_pending,
        }
    }
}

#[derive(Copy, Clone)]
pub struct Transition {
    runtime: &'static Runtime,
    scope: Scope,
    pending: ReadSignal<bool>,
    set_pending: WriteSignal<bool>,
}

impl Transition {
    pub fn start(&self, f: impl Fn()) {
        if self.runtime.running_transition().is_some() {
            f();
        } else {
            log::debug!("[Transition] starting a transition");
            {
                log::debug!("[Transition] running => true");
                self.set_pending.update(|n| *n = true);
                *self.runtime.transition.borrow_mut() = Some(Rc::new(TransitionState {
                    running: Cell::new(true),
                    resources: Default::default(),
                    signals: Default::default(),
                    memos: Default::default(),
                    effects: Default::default(),
                }));
            }

            log::debug!("[Transition] running start_transition f()");
            f();

            if let Some(running_transition) = self.runtime.running_transition() {
                log::debug!("[Transition] running => false");
                running_transition.running.set(false);

                let runtime = self.runtime;
                let scope = self.scope;
                let resources = running_transition.resources.clone();
                let signals = running_transition.signals.clone();
                let memos = running_transition.memos.clone();
                let effects = running_transition.effects.clone();
                let set_pending = self.set_pending;
                // place this at end of task queue so it doesn't start at 0
                queue_microtask(move || {
                    scope.create_effect(move |_| {
                        let pending = resources.borrow().iter().map(|p| p.get()).sum::<usize>();
                        log::debug!("[Transition] pending: {pending}");

                        if pending == 0 {
                            log::debug!("[Transition] Committing changes.");
                            for signal in signals.borrow().iter() {
                                runtime.any_signal(*signal, |signal| {
                                    signal.end_transition(runtime);
                                });
                            }
                            for memo in memos.borrow().iter() {
                                runtime.any_memo(*memo, |memo| {
                                    memo.end_transition(runtime);
                                });
                            }
                            for effect in effects.borrow().iter() {
                                log::debug!("running deferred effect");
                                runtime.any_effect(*effect, |any_effect| {
                                    any_effect.run(*effect);
                                });
                            }
                            set_pending.update(|n| *n = false);
                        }
                    });
                });
            }
        }
    }

    pub fn pending(&self) -> bool {
        self.pending.get()
    }
}

#[derive(Debug)]
pub(crate) struct TransitionState {
    pub running: Cell<bool>,
    pub resources: Rc<RefCell<HashSet<ReadSignal<usize>>>>,
    pub signals: Rc<RefCell<HashSet<(ScopeId, SignalId)>>>,
    pub memos: Rc<RefCell<HashSet<(ScopeId, MemoId)>>>,
    pub effects: Rc<RefCell<Vec<(ScopeId, EffectId)>>>,
}
