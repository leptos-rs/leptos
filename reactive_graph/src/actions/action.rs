use crate::{
    diagnostics::is_suppressing_resource_load,
    owner::{Owner, StoredValue},
    signal::{ArcRwSignal, RwSignal},
    traits::{DefinedAt, Dispose, GetUntracked, Update},
    unwrap_signal,
};
use any_spawner::Executor;
use futures::{channel::oneshot, select, FutureExt};
use std::{future::Future, panic::Location, pin::Pin, sync::Arc};

pub struct ArcAction<I, O>
where
    I: 'static,
    O: 'static,
{
    in_flight: ArcRwSignal<usize>,
    input: ArcRwSignal<Option<I>>,
    value: ArcRwSignal<Option<O>>,
    version: ArcRwSignal<usize>,
    #[allow(clippy::complexity)]
    action_fn: Arc<
        dyn Fn(&I) -> Pin<Box<dyn Future<Output = O> + Send>> + Send + Sync,
    >,
    #[cfg(debug_assertions)]
    defined_at: &'static Location<'static>,
}

impl<I, O> Clone for ArcAction<I, O>
where
    I: 'static,
    O: 'static,
{
    fn clone(&self) -> Self {
        Self {
            in_flight: self.in_flight.clone(),
            input: self.input.clone(),
            value: self.value.clone(),
            version: self.version.clone(),
            action_fn: self.action_fn.clone(),
            #[cfg(debug_assertions)]
            defined_at: self.defined_at,
        }
    }
}

impl<I, O> ArcAction<I, O>
where
    I: Send + Sync + 'static,
    O: Send + Sync + 'static,
{
    #[track_caller]
    pub fn new<F, Fu>(action_fn: F) -> Self
    where
        F: Fn(&I) -> Fu + Send + Sync + 'static,
        Fu: Future<Output = O> + Send + 'static,
    {
        ArcAction {
            in_flight: ArcRwSignal::new(0),
            input: Default::default(),
            value: Default::default(),
            version: Default::default(),
            action_fn: Arc::new(move |input| Box::pin(action_fn(input))),
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
        }
    }

    #[track_caller]
    pub fn dispatch(&self, input: I) {
        if !is_suppressing_resource_load() {
            let mut fut = (self.action_fn)(&input).fuse();

            // abort this task if the owner is cleaned up
            let (abort_tx, mut abort_rx) = oneshot::channel();
            Owner::on_cleanup(move || {
                abort_tx.send(()).expect(
                    "tried to cancel a future in ArcAction::dispatch(), but \
                     the channel has already closed",
                );
            });

            // Update the state before loading
            self.in_flight.update(|n| *n += 1);
            let current_version =
                self.version.try_get_untracked().unwrap_or_default();
            self.input.try_update(|inp| *inp = Some(input));

            // Spawn the task
            Executor::spawn({
                let input = self.input.clone();
                let version = self.version.clone();
                let value = self.value.clone();
                let in_flight = self.in_flight.clone();
                async move {
                    select! {
                        // if the abort message has been sent, bail and do nothing
                        _ = abort_rx => {
                            in_flight.update(|n| *n = n.saturating_sub(1));
                        },
                        // otherwise, update the value
                        result = fut => {
                            in_flight.update(|n| *n = n.saturating_sub(1));
                            let is_latest = version.get_untracked() <= current_version;
                            if is_latest {
                                version.update(|n| *n += 1);
                                value.update(|n| *n = Some(result));
                            }
                            if in_flight.get_untracked() == 0 {
                                input.update(|inp| *inp = None);
                            }
                        }
                    }
                }
            });
        }
    }
}

impl<I, O> ArcAction<I, O> {
    #[track_caller]
    pub fn version(&self) -> ArcRwSignal<usize> {
        self.version.clone()
    }

    #[track_caller]
    pub fn input(&self) -> ArcRwSignal<Option<I>> {
        self.input.clone()
    }

    #[track_caller]
    pub fn value(&self) -> ArcRwSignal<Option<O>> {
        self.value.clone()
    }
}

impl<I, O> DefinedAt for ArcAction<I, O>
where
    I: 'static,
    O: 'static,
{
    fn defined_at(&self) -> Option<&'static Location<'static>> {
        #[cfg(debug_assertions)]
        {
            Some(self.defined_at)
        }
        #[cfg(not(debug_assertions))]
        {
            None
        }
    }
}

pub struct Action<I, O>
where
    I: 'static,
    O: 'static,
{
    inner: StoredValue<ArcAction<I, O>>,
    #[cfg(debug_assertions)]
    defined_at: &'static Location<'static>,
}

impl<I: 'static, O: 'static> Dispose for Action<I, O> {
    fn dispose(self) {
        self.inner.dispose()
    }
}

impl<I, O> Action<I, O>
where
    I: Send + Sync + 'static,
    O: Send + Sync + 'static,
{
    #[track_caller]
    pub fn new<F, Fu>(action_fn: F) -> Self
    where
        F: Fn(&I) -> Fu + Send + Sync + 'static,
        Fu: Future<Output = O> + Send + 'static,
    {
        Self {
            inner: StoredValue::new(ArcAction::new(action_fn)),
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
        }
    }

    #[track_caller]
    pub fn version(&self) -> RwSignal<usize> {
        let inner = self
            .inner
            .try_with_value(|inner| inner.version())
            .unwrap_or_else(unwrap_signal!(self));
        inner.into()
    }

    #[track_caller]
    pub fn input(&self) -> RwSignal<Option<I>> {
        let inner = self
            .inner
            .try_with_value(|inner| inner.input())
            .unwrap_or_else(unwrap_signal!(self));
        inner.into()
    }

    #[track_caller]
    pub fn value(&self) -> RwSignal<Option<O>> {
        let inner = self
            .inner
            .try_with_value(|inner| inner.value())
            .unwrap_or_else(unwrap_signal!(self));
        inner.into()
    }

    #[track_caller]
    pub fn dispatch(&self, input: I) {
        self.inner.with_value(|inner| inner.dispatch(input));
    }
}

impl<I, O> DefinedAt for Action<I, O>
where
    I: 'static,
    O: 'static,
{
    fn defined_at(&self) -> Option<&'static Location<'static>> {
        #[cfg(debug_assertions)]
        {
            Some(self.defined_at)
        }
        #[cfg(not(debug_assertions))]
        {
            None
        }
    }
}

impl<I, O> Clone for Action<I, O>
where
    I: 'static,
    O: 'static,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<I, O> Copy for Action<I, O>
where
    I: 'static,
    O: 'static,
{
}

#[inline(always)]
#[track_caller]
#[deprecated = "This function is being removed to conform to Rust \
                idioms.Please use `Action::new()` instead."]
pub fn create_action<I, O, F, Fu>(action_fn: F) -> Action<I, O>
where
    I: Send + Sync + 'static,
    O: Send + Sync + 'static,
    F: Fn(&I) -> Fu + Send + Sync + 'static,
    Fu: Future<Output = O> + Send + 'static,
{
    Action::new(action_fn)
}
