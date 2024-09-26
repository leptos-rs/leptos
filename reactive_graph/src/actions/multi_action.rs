use crate::{
    diagnostics::is_suppressing_resource_load,
    owner::{ArenaItem, FromLocal, LocalStorage, Storage, SyncStorage},
    signal::{ArcReadSignal, ArcRwSignal, ReadSignal, RwSignal},
    traits::{DefinedAt, Dispose, GetUntracked, Set, Update},
    unwrap_signal,
};
use std::{fmt::Debug, future::Future, panic::Location, pin::Pin, sync::Arc};

/// An action that synchronizes multiple imperative `async` calls to the reactive system,
/// tracking the progress of each one.
///
/// Where an [`Action`](super::Action) fires a single call, a `MultiAction` allows you to
/// keep track of multiple in-flight actions.
///
/// If you’re trying to load data by running an `async` function reactively, you probably
/// want to use an [`AsyncDerived`](crate::computed::AsyncDerived) instead.
/// If you’re trying to occasionally run an `async` function in response to something
/// like a user adding a task to a todo list, you’re in the right place.
///
/// The reference-counted, `Clone` (but not `Copy` version of a `MultiAction` is an [`ArcMultiAction`].
///
/// ```rust
/// # use reactive_graph::actions::*;
/// # use reactive_graph::prelude::*;
/// # tokio_test::block_on(async move {
/// # any_spawner::Executor::init_tokio(); let owner = reactive_graph::owner::Owner::new(); owner.set();
/// # let _guard = reactive_graph::diagnostics::SpecialNonReactiveZone::enter();
/// async fn send_new_todo_to_api(task: String) -> usize {
///   // do something...
///   // return a task id
///   42
/// }
/// let add_todo = MultiAction::new(|task: &String| {
///   // `task` is given as `&String` because its value is available in `input`
///   send_new_todo_to_api(task.clone())
/// });
///
/// add_todo.dispatch("Buy milk".to_string());
/// add_todo.dispatch("???".to_string());
/// add_todo.dispatch("Profit!!!".to_string());
///
/// let submissions = add_todo.submissions();
/// assert_eq!(submissions.with(Vec::len), 3);
/// # });
/// ```
pub struct MultiAction<I, O, S = SyncStorage> {
    inner: ArenaItem<ArcMultiAction<I, O>, S>,
    #[cfg(debug_assertions)]
    defined_at: &'static Location<'static>,
}

impl<I, O, S> Dispose for MultiAction<I, O, S> {
    fn dispose(self) {
        self.inner.dispose()
    }
}

impl<I, O, S> DefinedAt for MultiAction<I, O, S>
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

impl<I, O, S> Copy for MultiAction<I, O, S>
where
    I: 'static,
    O: 'static,
{
}

impl<I, O, S> Clone for MultiAction<I, O, S>
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
    /// Creates a new multi-action.
    ///
    /// The input to the `async` function should always be a single value,
    /// but it can be of any type. The argument is always passed by reference to the
    /// function, because it is stored in [Submission::input] as well.
    ///
    /// ```rust
    /// # use reactive_graph::actions::*;
    /// # use reactive_graph::prelude::*;
    /// # tokio_test::block_on(async move {
    /// # any_spawner::Executor::init_tokio(); let owner = reactive_graph::owner::Owner::new(); owner.set();
    /// # let _guard = reactive_graph::diagnostics::SpecialNonReactiveZone::enter();
    /// // if there's a single argument, just use that
    /// let action1 = MultiAction::new(|input: &String| {
    ///     let input = input.clone();
    ///     async move { todo!() }
    /// });
    ///
    /// // if there are no arguments, use the unit type `()`
    /// let action2 = MultiAction::new(|input: &()| async { todo!() });
    ///
    /// // if there are multiple arguments, use a tuple
    /// let action3 =
    ///     MultiAction::new(|input: &(usize, String)| async { todo!() });
    /// # });
    /// ```
    #[track_caller]
    pub fn new<Fut>(
        action_fn: impl Fn(&I) -> Fut + Send + Sync + 'static,
    ) -> Self
    where
        Fut: Future<Output = O> + Send + 'static,
    {
        Self {
            inner: ArenaItem::new_with_storage(ArcMultiAction::new(action_fn)),
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
        }
    }
}

impl<I, O, S> MultiAction<I, O, S>
where
    I: Send + Sync + 'static,
    O: Send + Sync + 'static,
    S: Storage<ArcMultiAction<I, O>>,
{
    /// Calls the `async` function with a reference to the input type as its argument.
    ///
    /// This can be called any number of times: each submission will be dispatched, running
    /// concurrently, and its status can be checked via the
    /// [`submissions()`](MultiAction::submissions) signal.
    /// ```rust
    /// # use reactive_graph::actions::*;
    /// # use reactive_graph::prelude::*;
    /// # tokio_test::block_on(async move {
    /// # any_spawner::Executor::init_tokio(); let owner = reactive_graph::owner::Owner::new(); owner.set();
    /// # let _guard = reactive_graph::diagnostics::SpecialNonReactiveZone::enter();
    /// async fn send_new_todo_to_api(task: String) -> usize {
    ///   // do something...
    ///   // return a task id
    ///   42
    /// }
    /// let add_todo = MultiAction::new(|task: &String| {
    ///   // `task` is given as `&String` because its value is available in `input`
    ///   send_new_todo_to_api(task.clone())
    /// });
    ///
    /// let submissions = add_todo.submissions();
    /// let pending_submissions = move || {
    ///   submissions.with(|subs| subs.iter().filter(|sub| sub.pending().get()).count())
    /// };
    ///
    /// add_todo.dispatch("Buy milk".to_string());
    /// assert_eq!(submissions.with(Vec::len), 1);
    /// assert_eq!(pending_submissions(), 1);
    ///
    /// add_todo.dispatch("???".to_string());
    /// add_todo.dispatch("Profit!!!".to_string());
    ///
    /// assert_eq!(submissions.with(Vec::len), 3);
    /// assert_eq!(pending_submissions(), 3);
    ///
    /// // when submissions resolve, they are not removed from the set
    /// // however, their `pending` signal is now `false`, and this can be used to filter them
    /// # any_spawner::Executor::tick().await;
    /// assert_eq!(submissions.with(Vec::len), 3);
    /// assert_eq!(pending_submissions(), 0);
    /// # });
    /// ```
    pub fn dispatch(&self, input: I) {
        if !is_suppressing_resource_load() {
            self.inner.try_with_value(|inner| inner.dispatch(input));
        }
    }

    /// Synchronously adds a submission with the given value.
    ///
    /// This takes the output value, rather than the input, because it is adding a result, not an
    /// input.
    ///
    /// This can be useful for use cases like handling errors, where the error can already be known
    /// on the client side.
    /// ```rust
    /// # use reactive_graph::actions::*;
    /// # use reactive_graph::prelude::*;
    /// # tokio_test::block_on(async move {
    /// # any_spawner::Executor::init_tokio(); let owner = reactive_graph::owner::Owner::new(); owner.set();
    /// # let _guard = reactive_graph::diagnostics::SpecialNonReactiveZone::enter();
    /// async fn send_new_todo_to_api(task: String) -> usize {
    ///   // do something...
    ///   // return a task id
    ///   42
    /// }
    /// let add_todo = MultiAction::new(|task: &String| {
    ///   // `task` is given as `&String` because its value is available in `input`
    ///   send_new_todo_to_api(task.clone())
    /// });
    ///
    /// let submissions = add_todo.submissions();
    /// let pending_submissions = move || {
    ///   submissions.with(|subs| subs.iter().filter(|sub| sub.pending().get()).count())
    /// };
    ///
    /// add_todo.dispatch("Buy milk".to_string());
    /// assert_eq!(submissions.with(Vec::len), 1);
    /// assert_eq!(pending_submissions(), 1);
    ///
    /// add_todo.dispatch_sync(42);
    ///
    /// assert_eq!(submissions.with(Vec::len), 2);
    /// assert_eq!(pending_submissions(), 1);
    /// # });
    /// ```
    pub fn dispatch_sync(&self, value: O) {
        self.inner
            .try_with_value(|inner| inner.dispatch_sync(value));
    }
}

impl<I, O> MultiAction<I, O>
where
    I: Send + Sync + 'static,
    O: Send + Sync + 'static,
{
    /// The set of all submissions to this multi-action.
    /// ```rust
    /// # use reactive_graph::actions::*;
    /// # use reactive_graph::prelude::*;
    /// # tokio_test::block_on(async move {
    /// # any_spawner::Executor::init_tokio(); let owner = reactive_graph::owner::Owner::new(); owner.set();
    /// # let _guard = reactive_graph::diagnostics::SpecialNonReactiveZone::enter();
    /// async fn send_new_todo_to_api(task: String) -> usize {
    ///   // do something...
    ///   // return a task id
    ///   42
    /// }
    /// let add_todo = MultiAction::new(|task: &String| {
    ///   // `task` is given as `&String` because its value is available in `input`
    ///   send_new_todo_to_api(task.clone())
    /// });
    ///
    /// let submissions = add_todo.submissions();
    ///
    /// add_todo.dispatch("Buy milk".to_string());
    /// add_todo.dispatch("???".to_string());
    /// add_todo.dispatch("Profit!!!".to_string());
    ///
    /// assert_eq!(submissions.with(Vec::len), 3);
    /// # });
    /// ```
    pub fn submissions(&self) -> ReadSignal<Vec<ArcSubmission<I, O>>> {
        self.inner
            .try_with_value(|inner| inner.submissions())
            .unwrap_or_else(unwrap_signal!(self))
            .into()
    }
}

impl<I, O, S> MultiAction<I, O, S>
where
    I: 'static,
    O: 'static,
    S: Storage<ArcMultiAction<I, O>>
        + Storage<ArcReadSignal<Vec<ArcSubmission<I, O>>>>,
{
    /// How many times an action has successfully resolved.
    /// ```rust
    /// # use reactive_graph::actions::*;
    /// # use reactive_graph::prelude::*;
    /// # tokio_test::block_on(async move {
    /// # any_spawner::Executor::init_tokio(); let owner = reactive_graph::owner::Owner::new(); owner.set();
    /// # let _guard = reactive_graph::diagnostics::SpecialNonReactiveZone::enter();
    /// async fn send_new_todo_to_api(task: String) -> usize {
    ///   // do something...
    ///   // return a task id
    ///   42
    /// }
    /// let add_todo = MultiAction::new(|task: &String| {
    ///   // `task` is given as `&String` because its value is available in `input`
    ///   send_new_todo_to_api(task.clone())
    /// });
    ///
    /// let version = add_todo.version();
    ///
    /// add_todo.dispatch("Buy milk".to_string());
    /// add_todo.dispatch("???".to_string());
    /// add_todo.dispatch("Profit!!!".to_string());
    ///
    /// assert_eq!(version.get(), 0);
    /// # any_spawner::Executor::tick().await;
    ///
    /// // when they've all resolved
    /// assert_eq!(version.get(), 3);
    /// # });
    /// ```
    pub fn version(&self) -> RwSignal<usize> {
        self.inner
            .try_with_value(|inner| inner.version())
            .unwrap_or_else(unwrap_signal!(self))
            .into()
    }
}

/// An action that synchronizes multiple imperative `async` calls to the reactive system,
/// tracking the progress of each one.
///
/// Where an [`Action`](super::Action) fires a single call, a `MultiAction` allows you to
/// keep track of multiple in-flight actions.
///
/// If you’re trying to load data by running an `async` function reactively, you probably
/// want to use an [`AsyncDerived`](crate::computed::AsyncDerived) instead.
/// If you’re trying to occasionally run an `async` function in response to something
/// like a user adding a task to a todo list, you’re in the right place.
///
/// The arena-allocated, `Copy` version of an `ArcMultiAction` is a [`MultiAction`].
///
/// ```rust
/// # use reactive_graph::actions::*;
/// # use reactive_graph::prelude::*;
/// # tokio_test::block_on(async move {
/// # any_spawner::Executor::init_tokio(); let owner = reactive_graph::owner::Owner::new(); owner.set();
/// # let _guard = reactive_graph::diagnostics::SpecialNonReactiveZone::enter();
/// async fn send_new_todo_to_api(task: String) -> usize {
///   // do something...
///   // return a task id
///   42
/// }
/// let add_todo = ArcMultiAction::new(|task: &String| {
///   // `task` is given as `&String` because its value is available in `input`
///   send_new_todo_to_api(task.clone())
/// });
///
/// add_todo.dispatch("Buy milk".to_string());
/// add_todo.dispatch("???".to_string());
/// add_todo.dispatch("Profit!!!".to_string());
///
/// let submissions = add_todo.submissions();
/// assert_eq!(submissions.with(Vec::len), 3);
/// # });
/// ```
pub struct ArcMultiAction<I, O> {
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

impl<I, O> ArcMultiAction<I, O> {
    /// Creates a new multi-action.
    ///
    /// The input to the `async` function should always be a single value,
    /// but it can be of any type. The argument is always passed by reference to the
    /// function, because it is stored in [Submission::input] as well.
    ///
    /// ```rust
    /// # use reactive_graph::actions::*;
    /// # use reactive_graph::prelude::*;
    /// # tokio_test::block_on(async move {
    /// # any_spawner::Executor::init_tokio(); let owner = reactive_graph::owner::Owner::new(); owner.set();
    /// # let _guard = reactive_graph::diagnostics::SpecialNonReactiveZone::enter();
    /// // if there's a single argument, just use that
    /// let action1 = ArcMultiAction::new(|input: &String| {
    ///     let input = input.clone();
    ///     async move { todo!() }
    /// });
    ///
    /// // if there are no arguments, use the unit type `()`
    /// let action2 = ArcMultiAction::new(|input: &()| async { todo!() });
    ///
    /// // if there are multiple arguments, use a tuple
    /// let action3 =
    ///     ArcMultiAction::new(|input: &(usize, String)| async { todo!() });
    /// # });
    /// ```
    #[track_caller]
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
}

impl<I, O> ArcMultiAction<I, O>
where
    I: Send + Sync + 'static,
    O: Send + Sync + 'static,
{
    /// Calls the `async` function with a reference to the input type as its argument.
    ///
    /// This can be called any number of times: each submission will be dispatched, running
    /// concurrently, and its status can be checked via the
    /// [`submissions()`](MultiAction::submissions) signal.
    /// ```rust
    /// # use reactive_graph::actions::*;
    /// # use reactive_graph::prelude::*;
    /// # tokio_test::block_on(async move {
    /// # any_spawner::Executor::init_tokio(); let owner = reactive_graph::owner::Owner::new(); owner.set();
    /// # let _guard = reactive_graph::diagnostics::SpecialNonReactiveZone::enter();
    /// async fn send_new_todo_to_api(task: String) -> usize {
    ///   // do something...
    ///   // return a task id
    ///   42
    /// }
    /// let add_todo = ArcMultiAction::new(|task: &String| {
    ///   // `task` is given as `&String` because its value is available in `input`
    ///   send_new_todo_to_api(task.clone())
    /// });
    ///
    /// let submissions = add_todo.submissions();
    /// let pending_submissions = {
    ///     let submissions = submissions.clone();
    ///     move || {
    ///         submissions.with(|subs| subs.iter().filter(|sub| sub.pending().get()).count())
    ///     }
    /// };
    ///
    /// add_todo.dispatch("Buy milk".to_string());
    /// assert_eq!(submissions.with(Vec::len), 1);
    /// assert_eq!(pending_submissions(), 1);
    ///
    /// add_todo.dispatch("???".to_string());
    /// add_todo.dispatch("Profit!!!".to_string());
    ///
    /// assert_eq!(submissions.with(Vec::len), 3);
    /// assert_eq!(pending_submissions(), 3);
    ///
    /// // when submissions resolve, they are not removed from the set
    /// // however, their `pending` signal is now `false`, and this can be used to filter them
    /// # any_spawner::Executor::tick().await;
    /// assert_eq!(submissions.with(Vec::len), 3);
    /// assert_eq!(pending_submissions(), 0);
    /// # });
    /// ```
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

            crate::spawn(async move {
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
    /// This takes the output value, rather than the input, because it is adding a result, not an
    /// input.
    ///
    /// This can be useful for use cases like handling errors, where the error can already be known
    /// on the client side.
    /// ```rust
    /// # use reactive_graph::actions::*;
    /// # use reactive_graph::prelude::*;
    /// # tokio_test::block_on(async move {
    /// # any_spawner::Executor::init_tokio(); let owner = reactive_graph::owner::Owner::new(); owner.set();
    /// # let _guard = reactive_graph::diagnostics::SpecialNonReactiveZone::enter();
    /// async fn send_new_todo_to_api(task: String) -> usize {
    ///   // do something...
    ///   // return a task id
    ///   42
    /// }
    /// let add_todo = ArcMultiAction::new(|task: &String| {
    ///   // `task` is given as `&String` because its value is available in `input`
    ///   send_new_todo_to_api(task.clone())
    /// });
    ///
    /// let submissions = add_todo.submissions();
    /// let pending_submissions = {
    ///     let submissions = submissions.clone();
    ///     move || {
    ///         submissions.with(|subs| subs.iter().filter(|sub| sub.pending().get()).count())
    ///     }
    /// };
    ///
    /// add_todo.dispatch("Buy milk".to_string());
    /// assert_eq!(submissions.with(Vec::len), 1);
    /// assert_eq!(pending_submissions(), 1);
    ///
    /// add_todo.dispatch_sync(42);
    ///
    /// assert_eq!(submissions.with(Vec::len), 2);
    /// assert_eq!(pending_submissions(), 1);
    /// # });
    /// ```
    pub fn dispatch_sync(&self, value: O) {
        let submission = ArcSubmission {
            input: ArcRwSignal::new(None),
            value: ArcRwSignal::new(Some(value)),
            pending: ArcRwSignal::new(false),
            canceled: ArcRwSignal::new(false),
        };

        self.submissions
            .try_update(|subs| subs.push(submission.clone()));
        self.version.try_update(|n| *n += 1);
    }
}

impl<I, O> ArcMultiAction<I, O> {
    /// The set of all submissions to this multi-action.
    /// ```rust
    /// # use reactive_graph::actions::*;
    /// # use reactive_graph::prelude::*;
    /// # tokio_test::block_on(async move {
    /// # any_spawner::Executor::init_tokio(); let owner = reactive_graph::owner::Owner::new(); owner.set();
    /// # let _guard = reactive_graph::diagnostics::SpecialNonReactiveZone::enter();
    /// async fn send_new_todo_to_api(task: String) -> usize {
    ///   // do something...
    ///   // return a task id
    ///   42
    /// }
    /// let add_todo = ArcMultiAction::new(|task: &String| {
    ///   // `task` is given as `&String` because its value is available in `input`
    ///   send_new_todo_to_api(task.clone())
    /// });
    ///
    /// let submissions = add_todo.submissions();
    ///
    /// add_todo.dispatch("Buy milk".to_string());
    /// add_todo.dispatch("???".to_string());
    /// add_todo.dispatch("Profit!!!".to_string());
    ///
    /// assert_eq!(submissions.with(Vec::len), 3);
    /// # });
    /// ```
    pub fn submissions(&self) -> ArcReadSignal<Vec<ArcSubmission<I, O>>> {
        self.submissions.read_only()
    }

    /// How many times an action has successfully resolved.
    /// ```rust
    /// # use reactive_graph::actions::*;
    /// # use reactive_graph::prelude::*;
    /// # tokio_test::block_on(async move {
    /// # any_spawner::Executor::init_tokio(); let owner = reactive_graph::owner::Owner::new(); owner.set();
    /// # let _guard = reactive_graph::diagnostics::SpecialNonReactiveZone::enter();
    /// async fn send_new_todo_to_api(task: String) -> usize {
    ///   // do something...
    ///   // return a task id
    ///   42
    /// }
    /// let add_todo = ArcMultiAction::new(|task: &String| {
    ///   // `task` is given as `&String` because its value is available in `input`
    ///   send_new_todo_to_api(task.clone())
    /// });
    ///
    /// let version = add_todo.version();
    ///
    /// add_todo.dispatch("Buy milk".to_string());
    /// add_todo.dispatch("???".to_string());
    /// add_todo.dispatch("Profit!!!".to_string());
    ///
    /// assert_eq!(version.get(), 0);
    /// # any_spawner::Executor::tick().await;
    ///
    /// // when they've all resolved
    /// assert_eq!(version.get(), 3);
    /// # });
    /// ```
    pub fn version(&self) -> ArcRwSignal<usize> {
        self.version.clone()
    }
}

/// An action that has been submitted by dispatching it to a [`MultiAction`].
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct ArcSubmission<I, O> {
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
    /// The current argument that was dispatched to the `async` function.
    /// `Some` while we are waiting for it to resolve, `None` if it has resolved.
    #[track_caller]
    pub fn input(&self) -> ArcReadSignal<Option<I>> {
        self.input.read_only()
    }

    /// The most recent return value of the `async` function.
    #[track_caller]
    pub fn value(&self) -> ArcReadSignal<Option<O>> {
        self.value.read_only()
    }

    /// Whether this submision is still waiting to resolve.
    #[track_caller]
    pub fn pending(&self) -> ArcReadSignal<bool> {
        self.pending.read_only()
    }

    /// Whether this submission has been canceled.
    #[track_caller]
    pub fn canceled(&self) -> ArcReadSignal<bool> {
        self.canceled.read_only()
    }

    /// Cancels the submission. This will not necessarily prevent the `Future`
    /// from continuing to run, but it will update the returned value.
    #[track_caller]
    pub fn cancel(&self) {
        // TODO if we set these up to race against a cancel signal, we could actually drop the
        // futures
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

/// An action that has been submitted by dispatching it to a [`MultiAction`].
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Submission<I, O, S = SyncStorage>
where
    I: 'static,
    O: 'static,
{
    /// The current argument that was dispatched to the `async` function.
    /// `Some` while we are waiting for it to resolve, `None` if it has resolved.
    input: RwSignal<Option<I>, S>,
    /// The most recent return value of the `async` function.
    value: RwSignal<Option<O>, S>,
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

impl<I, O> FromLocal<ArcSubmission<I, O>> for Submission<I, O, LocalStorage>
where
    I: 'static,
    O: 'static,
{
    fn from_local(value: ArcSubmission<I, O>) -> Self {
        let ArcSubmission {
            input,
            value,
            pending,
            canceled,
        } = value;
        Self {
            input: RwSignal::from_local(input),
            value: RwSignal::from_local(value),
            pending: pending.into(),
            canceled: canceled.into(),
        }
    }
}

impl<I, O, S> Submission<I, O, S>
where
    S: Storage<ArcRwSignal<Option<I>>> + Storage<ArcReadSignal<Option<I>>>,
{
    /// The current argument that was dispatched to the `async` function.
    /// `Some` while we are waiting for it to resolve, `None` if it has resolved.
    #[track_caller]
    pub fn input(&self) -> ReadSignal<Option<I>, S> {
        self.input.read_only()
    }
}

impl<I, O, S> Submission<I, O, S>
where
    S: Storage<ArcRwSignal<Option<O>>> + Storage<ArcReadSignal<Option<O>>>,
{
    /// The most recent return value of the `async` function.
    #[track_caller]
    pub fn value(&self) -> ReadSignal<Option<O>, S> {
        self.value.read_only()
    }
}

impl<I, O, S> Submission<I, O, S> {
    /// Whether this submision is still waiting to resolve.
    #[track_caller]
    pub fn pending(&self) -> ReadSignal<bool> {
        self.pending.read_only()
    }

    /// Whether this submission has been canceled.
    #[track_caller]
    pub fn canceled(&self) -> ReadSignal<bool> {
        self.canceled.read_only()
    }

    /// Cancels the submission. This will not necessarily prevent the `Future`
    /// from continuing to run, but it will update the returned value.
    #[track_caller]
    pub fn cancel(&self) {
        self.canceled.try_set(true);
    }
}

impl<I, O, S> Clone for Submission<I, O, S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<I, O, S> Copy for Submission<I, O, S> {}
