use crate::{
    computed::AsyncState,
    signal::ArcRwSignal,
    traits::{Set, Update},
};
use std::{
    future::Future,
    pin::Pin,
    sync::{atomic::AtomicUsize, Arc},
};

pub enum ActionState<I, O> {
    Idle,
    Loading(I),
    Complete(O),
    Reloading(I, O),
}

pub struct ArcAction<I, O>
where
    I: 'static,
    O: 'static,
{
    version: ArcRwSignal<usize>,
    state: ArcRwSignal<ActionState<I, O>>,
    pending_dispatches: Arc<AtomicUsize>,
    #[allow(clippy::complexity)]
    action_fn: Arc<dyn Fn(&I) -> Pin<Box<dyn Future<Output = O>>>>,
    #[cfg(debug_assertion)]
    defined_at: &'static Location<'static>,
}

impl<I, O> ArcAction<I, O>
where
    I: 'static,
    O: 'static,
{
    #[track_caller]
    pub fn dispatch(&self, input: I) {
        let fut = (self.action_fn)(&input);

        self.state.update(|prev| {
            *prev = match prev {
                ActionState::Idle => ActionState::Loading(input),
                ActionState::Loading(_) => todo!(),
                ActionState::Complete(_) => todo!(),
                ActionState::Reloading(_, _) => todo!(),
            }
        });
    }
}
