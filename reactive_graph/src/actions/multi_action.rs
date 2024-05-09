use crate::{
    diagnostics::is_suppressing_resource_load,
    owner::StoredValue,
    signal::{ArcReadSignal, ArcRwSignal, ReadSignal, RwSignal},
    traits::{DefinedAt, Dispose, GetUntracked, Set, Update},
    unwrap_signal,
};
use any_spawner::Executor;
use std::{fmt::Debug, future::Future, panic::Location, pin::Pin, sync::Arc};

pub struct MultiAction<I, O>
where
    I: 'static,
    O: 'static,
{
    inner: StoredValue<ArcMultiAction<I, O>>,
    #[cfg(debug_assertions)]
    defined_at: &'static Location<'static>,
}

impl<I: 'static, O: 'static> Dispose for MultiAction<I, O> {
    fn dispose(self) {
        self.inner.dispose()
    }
}

impl<I, O> DefinedAt for MultiAction<I, O>
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

impl<I, O> Copy for MultiAction<I, O>
where
    I: 'static,
    O: 'static,
{
}

impl<I, O> Clone for MultiAction<I, O>
where
    I: 'static,
    O: 'static,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<I, O> MultiAction<I, O>
where
    I: Send + Sync + 'static,
    O: Send + Sync + 'static,
{
    #[track_caller]
    pub fn new<Fut>(
        action_fn: impl Fn(&I) -> Fut + Send + Sync + 'static,
    ) -> Self
    where
        Fut: Future<Output = O> + Send + 'static,
    {
        Self {
            inner: StoredValue::new(ArcMultiAction::new(action_fn)),
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
        }
    }

    /// Calls the `async` function with a reference to the input type as its argument.
    pub fn dispatch(&self, input: I) {
        if !is_suppressing_resource_load() {
            self.inner.with_value(|inner| inner.dispatch(input));
        }
    }

    /// Synchronously adds a submission with the given value.
    ///
    /// This can be useful for use cases like handling errors, where the error can already be known
    /// on the client side.
    pub fn dispatch_sync(&self, value: O) {
        self.inner.with_value(|inner| inner.dispatch_sync(value));
    }

    /// The set of all submissions to this multi-action.
    pub fn submissions(&self) -> ReadSignal<Vec<ArcSubmission<I, O>>> {
        self.inner
            .try_with_value(|inner| inner.submissions())
            .unwrap_or_else(unwrap_signal!(self))
            .into()
    }

    /// How many times an action has successfully resolved.
    pub fn version(&self) -> RwSignal<usize> {
        self.inner
            .try_with_value(|inner| inner.version())
            .unwrap_or_else(unwrap_signal!(self))
            .into()
    }
}

pub struct ArcMultiAction<I, O>
where
    I: 'static,
    O: 'static,
{
    version: ArcRwSignal<usize>,
    submissions: ArcRwSignal<Vec<ArcSubmission<I, O>>>,
    #[allow(clippy::complexity)]
    action_fn: Arc<
        dyn Fn(&I) -> Pin<Box<dyn Future<Output = O> + Send>> + Send + Sync,
    >,
}

impl<I, O> Debug for ArcMultiAction<I, O>
where
    I: 'static,
    O: 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ArcMultiAction")
            .field("version", &self.version)
            .field("submissions", &self.submissions)
            .finish()
    }
}

impl<I, O> Clone for ArcMultiAction<I, O>
where
    I: 'static,
    O: 'static,
{
    fn clone(&self) -> Self {
        Self {
            version: self.version.clone(),
            submissions: self.submissions.clone(),
            action_fn: Arc::clone(&self.action_fn),
        }
    }
}

impl<I, O> ArcMultiAction<I, O>
where
    I: 'static,
    O: 'static,
{
    pub fn new<Fut>(
        action_fn: impl Fn(&I) -> Fut + Send + Sync + 'static,
    ) -> Self
    where
        Fut: Future<Output = O> + Send + 'static,
    {
        let action_fn = Arc::new(move |input: &I| {
            let fut = action_fn(input);
            Box::pin(fut) as Pin<Box<dyn Future<Output = O> + Send>>
        });
        Self {
            version: ArcRwSignal::new(0),
            submissions: ArcRwSignal::new(Vec::new()),
            action_fn,
        }
    }

    /// Calls the `async` function with a reference to the input type as its argument.
    pub fn dispatch(&self, input: I) {
        if !is_suppressing_resource_load() {
            let fut = (self.action_fn)(&input);

            let submission = ArcSubmission {
                input: ArcRwSignal::new(Some(input)),
                value: ArcRwSignal::new(None),
                pending: ArcRwSignal::new(true),
                canceled: ArcRwSignal::new(false),
            };

            self.submissions
                .try_update(|subs| subs.push(submission.clone()));

            let version = self.version.clone();

            Executor::spawn_local(async move {
                let new_value = fut.await;
                let canceled = submission.canceled.get_untracked();
                if !canceled {
                    submission.value.try_set(Some(new_value));
                }
                submission.input.try_set(None);
                submission.pending.try_set(false);
                version.try_update(|n| *n += 1);
            })
        }
    }

    /// Synchronously adds a submission with the given value.
    ///
    /// This can be useful for use cases like handling errors, where the error can already be known
    /// on the client side.
    pub fn dispatch_sync(&self, value: O) {
        let submission = ArcSubmission {
            input: ArcRwSignal::new(None),
            value: ArcRwSignal::new(Some(value)),
            pending: ArcRwSignal::new(true),
            canceled: ArcRwSignal::new(false),
        };

        self.submissions
            .try_update(|subs| subs.push(submission.clone()));
        self.version.try_update(|n| *n += 1);
    }

    /// The set of all submissions to this multi-action.
    pub fn submissions(&self) -> ArcReadSignal<Vec<ArcSubmission<I, O>>> {
        self.submissions.read_only()
    }

    /// How many times an action has successfully resolved.
    pub fn version(&self) -> ArcRwSignal<usize> {
        self.version.clone()
    }
}

/// An action that has been submitted by dispatching it to a [MultiAction](crate::MultiAction).
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct ArcSubmission<I, O>
where
    I: 'static,
    O: 'static,
{
    /// The current argument that was dispatched to the `async` function.
    /// `Some` while we are waiting for it to resolve, `None` if it has resolved.
    input: ArcRwSignal<Option<I>>,
    /// The most recent return value of the `async` function.
    value: ArcRwSignal<Option<O>>,
    pending: ArcRwSignal<bool>,
    /// Controls this submission has been canceled.
    canceled: ArcRwSignal<bool>,
}

impl<I, O> ArcSubmission<I, O>
where
    I: 'static,
    O: 'static,
{
    pub fn input(&self) -> ArcReadSignal<Option<I>> {
        self.input.read_only()
    }

    pub fn value(&self) -> ArcReadSignal<Option<O>> {
        self.value.read_only()
    }

    pub fn pending(&self) -> ArcReadSignal<bool> {
        self.pending.read_only()
    }

    pub fn canceled(&self) -> ArcReadSignal<bool> {
        self.canceled.read_only()
    }

    pub fn cancel(&self) {
        self.canceled.try_set(true);
    }
}

impl<I, O> Clone for ArcSubmission<I, O> {
    fn clone(&self) -> Self {
        Self {
            input: self.input.clone(),
            value: self.value.clone(),
            pending: self.pending.clone(),
            canceled: self.canceled.clone(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Submission<I, O>
where
    I: 'static,
    O: 'static,
{
    /// The current argument that was dispatched to the `async` function.
    /// `Some` while we are waiting for it to resolve, `None` if it has resolved.
    input: RwSignal<Option<I>>,
    /// The most recent return value of the `async` function.
    value: RwSignal<Option<O>>,
    pending: RwSignal<bool>,
    /// Controls this submission has been canceled.
    canceled: RwSignal<bool>,
}

impl<I, O> From<ArcSubmission<I, O>> for Submission<I, O>
where
    I: Send + Sync + 'static,
    O: Send + Sync + 'static,
{
    fn from(value: ArcSubmission<I, O>) -> Self {
        let ArcSubmission {
            input,
            value,
            pending,
            canceled,
        } = value;
        Self {
            input: input.into(),
            value: value.into(),
            pending: pending.into(),
            canceled: canceled.into(),
        }
    }
}

impl<I, O> Submission<I, O>
where
    I: Send + Sync + 'static,
    O: Send + Sync + 'static,
{
    pub fn input(&self) -> ReadSignal<Option<I>> {
        self.input.read_only()
    }

    pub fn value(&self) -> ReadSignal<Option<O>> {
        self.value.read_only()
    }

    pub fn pending(&self) -> ReadSignal<bool> {
        self.pending.read_only()
    }

    pub fn canceled(&self) -> ReadSignal<bool> {
        self.canceled.read_only()
    }

    pub fn cancel(&self) {
        self.canceled.try_set(true);
    }
}

impl<I, O> Clone for Submission<I, O> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<I, O> Copy for Submission<I, O> {}
