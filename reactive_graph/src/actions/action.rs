use crate::{
    computed::{ArcMemo, Memo},
    diagnostics::is_suppressing_resource_load,
    owner::{LocalStorage, Owner, Storage, StoredValue, SyncStorage},
    signal::{ArcRwSignal, RwSignal},
    traits::{DefinedAt, Dispose, Get, GetUntracked, Update},
    unwrap_signal,
};
use any_spawner::Executor;
use futures::{channel::oneshot, select, FutureExt};
use send_wrapper::SendWrapper;
use std::{future::Future, panic::Location, pin::Pin, sync::Arc};

/// An action runs some asynchronous code when you dispatch a new value to it, and gives you
/// reactive access to the result.
///
/// Actions are intended for mutating or updating data, not for loading data. If you find yourself
/// creating an action and immediately dispatching a value to it, this is probably the wrong
/// primitive.
///
/// The arena-allocated, `Copy` version of an `ArcAction` is an [`Action`].
///
/// ```rust
/// # use reactive_graph::actions::*;
/// # use reactive_graph::prelude::*;
/// # tokio_test::block_on(async move {
/// # any_spawner::Executor::init_tokio();
/// # let _guard = reactive_graph::diagnostics::SpecialNonReactiveZone::enter();
/// async fn send_new_todo_to_api(task: String) -> usize {
///     // do something...
///     // return a task id
///     42
/// }
/// let save_data = ArcAction::new(|task: &String| {
///   // `task` is given as `&String` because its value is available in `input`
///   send_new_todo_to_api(task.clone())
/// });
///
/// // the argument currently running
/// let input = save_data.input();
/// // the most recent returned result
/// let result_of_call = save_data.value();
/// // whether the call is pending
/// let pending = save_data.pending();
/// // how many times the action has run
/// // useful for reactively updating something else in response to a `dispatch` and response
/// let version = save_data.version();
///
/// // before we do anything
/// assert_eq!(input.get(), None); // no argument yet
/// assert_eq!(pending.get(), false); // isn't pending a response
/// assert_eq!(result_of_call.get(), None); // there's no "last value"
/// assert_eq!(version.get(), 0);
///
/// // dispatch the action
/// save_data.dispatch("My todo".to_string());
///
/// // when we're making the call
/// assert_eq!(input.get(), Some("My todo".to_string()));
/// assert_eq!(pending.get(), true); // is pending
/// assert_eq!(result_of_call.get(), None); // has not yet gotten a response
///
/// # any_spawner::Executor::tick().await;
///
/// // after call has resolved
/// assert_eq!(input.get(), None); // input clears out after resolved
/// assert_eq!(pending.get(), false); // no longer pending
/// assert_eq!(result_of_call.get(), Some(42));
/// assert_eq!(version.get(), 1);
/// # });
/// ```
///
/// The input to the `async` function should always be a single value,
/// but it can be of any type. The argument is always passed by reference to the
/// function, because it is stored in [Action::input] as well.
///
/// ```rust
/// # use reactive_graph::actions::*;
/// // if there's a single argument, just use that
/// let action1 = ArcAction::new(|input: &String| {
///     let input = input.clone();
///     async move { todo!() }
/// });
///
/// // if there are no arguments, use the unit type `()`
/// let action2 = ArcAction::new(|input: &()| async { todo!() });
///
/// // if there are multiple arguments, use a tuple
/// let action3 = ArcAction::new(|input: &(usize, String)| async { todo!() });
/// ```
pub struct ArcAction<I, O> {
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

impl<I, O> Clone for ArcAction<I, O> {
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
    I: 'static,
    O: 'static,
{
    /// Creates a new action. This is lazy: it does not run the action function until some value
    /// is dispatched.
    ///
    /// The constructor takes a function which will create a new `Future` from some input data.
    /// When the action is dispatched, this `action_fn` will run, and the `Future` it returns will
    /// be spawned.
    ///
    /// The `action_fn` must be `Send + Sync` so that the `ArcAction` is `Send + Sync`. The
    /// `Future` must be `Send` so that it can be moved across threads by the async executor as
    /// needed.
    ///
    /// ```rust
    /// # use reactive_graph::actions::*;
    /// # use reactive_graph::prelude::*;
    /// # tokio_test::block_on(async move {
    /// # any_spawner::Executor::init_tokio();
    /// # let _guard = reactive_graph::diagnostics::SpecialNonReactiveZone::enter();
    /// let act = ArcAction::new(|n: &u8| {
    ///     let n = n.to_owned();
    ///     async move { n * 2 }
    /// });
    ///
    /// act.dispatch(3);
    /// assert_eq!(act.input().get(), Some(3));
    ///
    /// // Remember that async functions already return a future if they are
    /// // not `await`ed. You can save keystrokes by leaving out the `async move`
    ///
    /// let act2 = Action::new(|n: &String| yell(n.to_owned()));
    /// act2.dispatch(String::from("i'm in a doctest"));
    /// # tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    ///
    /// // after it resolves
    /// assert_eq!(act2.value().get(), Some("I'M IN A DOCTEST".to_string()));
    ///
    /// async fn yell(n: String) -> String {
    ///     n.to_uppercase()
    /// }
    /// # });
    /// ```
    #[track_caller]
    pub fn new<F, Fu>(action_fn: F) -> Self
    where
        F: Fn(&I) -> Fu + Send + Sync + 'static,
        Fu: Future<Output = O> + Send + 'static,
    {
        Self::new_with_value(None, action_fn)
    }

    /// Creates a new action, initializing it with the given value.
    ///
    /// This is lazy: it does not run the action function until some value is dispatched.
    ///
    /// The constructor takes a function which will create a new `Future` from some input data.
    /// When the action is dispatched, this `action_fn` will run, and the `Future` it returns will
    /// be spawned.
    ///
    /// The `action_fn` must be `Send + Sync` so that the `ArcAction` is `Send + Sync`. The
    /// `Future` must be `Send` so that it can be moved across threads by the async executor as
    /// needed.
    #[track_caller]
    pub fn new_with_value<F, Fu>(value: Option<O>, action_fn: F) -> Self
    where
        F: Fn(&I) -> Fu + Send + Sync + 'static,
        Fu: Future<Output = O> + Send + 'static,
    {
        ArcAction {
            in_flight: ArcRwSignal::new(0),
            input: Default::default(),
            value: ArcRwSignal::new(value),
            version: Default::default(),
            action_fn: Arc::new(move |input| Box::pin(action_fn(input))),
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
        }
    }
}

impl<I, O> ArcAction<I, O>
where
    I: Send + Sync + 'static,
    O: Send + Sync + 'static,
{
    /// Calls the `async` function with a reference to the input type as its argument.
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

impl<I, O> ArcAction<I, O>
where
    I: 'static,
    O: 'static,
{
    /// Calls the `async` function with a reference to the input type as its argument,
    /// ensuring that it is spawned on the current thread.
    #[track_caller]
    pub fn dispatch_local(&self, input: I) {
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
            Executor::spawn_local({
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

impl<I, O> ArcAction<I, O>
where
    I: 'static,
    O: 'static,
{
    /// Creates a new action, which will only be run on the thread in which it is created.
    ///
    /// In all other ways, this is identical to [`ArcAction::new`].
    #[track_caller]
    pub fn new_unsync<F, Fu>(action_fn: F) -> Self
    where
        F: Fn(&I) -> Fu + 'static,
        Fu: Future<Output = O> + 'static,
    {
        let action_fn = move |inp: &I| SendWrapper::new(action_fn(inp));
        Self::new_unsync_with_value(None, action_fn)
    }

    /// Creates a new action that will only run on the current thread, initializing it with the given value.
    ///
    /// In all other ways, this is identical to [`ArcAction::new_with_value`].
    #[track_caller]
    pub fn new_unsync_with_value<F, Fu>(value: Option<O>, action_fn: F) -> Self
    where
        F: Fn(&I) -> Fu + 'static,
        Fu: Future<Output = O> + 'static,
    {
        let action_fn = SendWrapper::new(action_fn);
        ArcAction {
            in_flight: ArcRwSignal::new(0),
            input: Default::default(),
            value: ArcRwSignal::new(value),
            version: Default::default(),
            action_fn: Arc::new(move |input| {
                Box::pin(SendWrapper::new(action_fn(input)))
            }),
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
        }
    }
}

impl<I, O> ArcAction<I, O> {
    /// The number of times the action has successfully completed.
    ///
    /// ```rust
    /// # use reactive_graph::actions::*;
    /// # use reactive_graph::prelude::*;
    /// # tokio_test::block_on(async move {
    /// # any_spawner::Executor::init_tokio();
    /// # let _guard = reactive_graph::diagnostics::SpecialNonReactiveZone::enter();
    /// let act = ArcAction::new(|n: &u8| {
    ///     let n = n.to_owned();
    ///     async move { n * 2 }
    /// });
    ///
    /// let version = act.version();
    /// act.dispatch(3);
    /// assert_eq!(version.get(), 0);
    ///
    /// # tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    /// // after it resolves
    /// assert_eq!(version.get(), 1);
    /// # });
    /// ```
    #[track_caller]
    pub fn version(&self) -> ArcRwSignal<usize> {
        self.version.clone()
    }

    /// The current argument that was dispatched to the async function. This value will
    /// be `Some` while we are waiting for it to resolve, and `None` after it has resolved.
    ///
    /// ```rust
    /// # use reactive_graph::actions::*;
    /// # use reactive_graph::prelude::*;
    /// # tokio_test::block_on(async move {
    /// # any_spawner::Executor::init_tokio();
    /// # let _guard = reactive_graph::diagnostics::SpecialNonReactiveZone::enter();
    /// let act = ArcAction::new(|n: &u8| {
    ///     let n = n.to_owned();
    ///     async move { n * 2 }
    /// });
    ///
    /// let input = act.input();
    /// assert_eq!(input.get(), None);
    /// act.dispatch(3);
    /// assert_eq!(input.get(), Some(3));
    ///
    /// # tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    /// // after it resolves
    /// assert_eq!(input.get(), None);
    /// # });
    /// ```
    #[track_caller]
    pub fn input(&self) -> ArcRwSignal<Option<I>> {
        self.input.clone()
    }

    /// The most recent return value of the `async` function. This will be `None` before
    /// the action has ever run successfully, and subsequently will always be `Some(_)`,
    /// holding the old value until a new value has been received.
    ///
    /// ```rust
    /// # use reactive_graph::actions::*;
    /// # use reactive_graph::prelude::*;
    /// # tokio_test::block_on(async move {
    /// # any_spawner::Executor::init_tokio();
    /// # let _guard = reactive_graph::diagnostics::SpecialNonReactiveZone::enter();
    /// let act = ArcAction::new(|n: &u8| {
    ///     let n = n.to_owned();
    ///     async move { n * 2 }
    /// });
    ///
    /// let value = act.value();
    /// assert_eq!(value.get(), None);
    /// act.dispatch(3);
    /// assert_eq!(value.get(), None);
    ///
    /// # tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    /// // after it resolves
    /// assert_eq!(value.get(), Some(6));
    /// // dispatch another value, and it still holds the old value
    /// act.dispatch(3);
    /// assert_eq!(value.get(), Some(6));
    /// # });
    /// ```
    #[track_caller]
    pub fn value(&self) -> ArcRwSignal<Option<O>> {
        self.value.clone()
    }

    /// Whether the action has been dispatched and is currently waiting to resolve.
    ///
    /// ```rust
    /// # use reactive_graph::actions::*;
    /// # use reactive_graph::prelude::*;
    /// # tokio_test::block_on(async move {
    /// # any_spawner::Executor::init_tokio();
    /// # let _guard = reactive_graph::diagnostics::SpecialNonReactiveZone::enter();
    /// let act = ArcAction::new(|n: &u8| {
    ///     let n = n.to_owned();
    ///     async move { n * 2 }
    /// });
    ///
    /// let pending = act.pending();
    /// assert_eq!(pending.get(), false);
    /// act.dispatch(3);
    /// assert_eq!(pending.get(), true);
    ///
    /// # tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    /// // after it resolves
    /// assert_eq!(pending.get(), false);
    /// # });
    /// ```
    #[track_caller]
    pub fn pending(&self) -> ArcMemo<bool> {
        let in_flight = self.in_flight.clone();
        ArcMemo::new(move |_| in_flight.get() > 0)
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

/// An action runs some asynchronous code when you dispatch a new value to it, and gives you
/// reactive access to the result.
///
/// Actions are intended for mutating or updating data, not for loading data. If you find yourself
/// creating an action and immediately dispatching a value to it, this is probably the wrong
/// primitive.
///
/// The reference-counted, `Clone` (but not `Copy` version of an `Action` is an [`ArcAction`].
///
/// ```rust
/// # use reactive_graph::actions::*;
/// # use reactive_graph::prelude::*;
/// # tokio_test::block_on(async move {
/// # any_spawner::Executor::init_tokio();
/// # let _guard = reactive_graph::diagnostics::SpecialNonReactiveZone::enter();
/// async fn send_new_todo_to_api(task: String) -> usize {
///     // do something...
///     // return a task id
///     42
/// }
/// let save_data = Action::new(|task: &String| {
///   // `task` is given as `&String` because its value is available in `input`
///   send_new_todo_to_api(task.clone())
/// });
///
/// // the argument currently running
/// let input = save_data.input();
/// // the most recent returned result
/// let result_of_call = save_data.value();
/// // whether the call is pending
/// let pending = save_data.pending();
/// // how many times the action has run
/// // useful for reactively updating something else in response to a `dispatch` and response
/// let version = save_data.version();
///
/// // before we do anything
/// assert_eq!(input.get(), None); // no argument yet
/// assert_eq!(pending.get(), false); // isn't pending a response
/// assert_eq!(result_of_call.get(), None); // there's no "last value"
/// assert_eq!(version.get(), 0);
///
/// // dispatch the action
/// save_data.dispatch("My todo".to_string());
///
/// // when we're making the call
/// assert_eq!(input.get(), Some("My todo".to_string()));
/// assert_eq!(pending.get(), true); // is pending
/// assert_eq!(result_of_call.get(), None); // has not yet gotten a response
///
/// # any_spawner::Executor::tick().await;
///
/// // after call has resolved
/// assert_eq!(input.get(), None); // input clears out after resolved
/// assert_eq!(pending.get(), false); // no longer pending
/// assert_eq!(result_of_call.get(), Some(42));
/// assert_eq!(version.get(), 1);
/// # });
/// ```
///
/// The input to the `async` function should always be a single value,
/// but it can be of any type. The argument is always passed by reference to the
/// function, because it is stored in [Action::input] as well.
///
/// ```rust
/// # use reactive_graph::actions::*;
/// // if there's a single argument, just use that
/// let action1 = Action::new(|input: &String| {
///     let input = input.clone();
///     async move { todo!() }
/// });
///
/// // if there are no arguments, use the unit type `()`
/// let action2 = Action::new(|input: &()| async { todo!() });
///
/// // if there are multiple arguments, use a tuple
/// let action3 = Action::new(|input: &(usize, String)| async { todo!() });
/// ```
pub struct Action<I, O, S = SyncStorage> {
    inner: StoredValue<ArcAction<I, O>, S>,
    #[cfg(debug_assertions)]
    defined_at: &'static Location<'static>,
}

impl<I, O, S> Dispose for Action<I, O, S> {
    fn dispose(self) {
        self.inner.dispose()
    }
}

impl<I, O> Action<I, O>
where
    I: Send + Sync + 'static,
    O: Send + Sync + 'static,
{
    /// Creates a new action. This is lazy: it does not run the action function until some value
    /// is dispatched.
    ///
    /// The constructor takes a function which will create a new `Future` from some input data.
    /// When the action is dispatched, this `action_fn` will run, and the `Future` it returns will
    /// be spawned.
    ///
    /// The `action_fn` must be `Send + Sync` so that the `ArcAction` is `Send + Sync`. The
    /// `Future` must be `Send` so that it can be moved across threads by the async executor as
    /// needed. In order to be stored in the `Copy` arena, the input and output types should also
    /// be `Send + Sync`.
    ///
    /// ```rust
    /// # use reactive_graph::actions::*;
    /// # use reactive_graph::prelude::*;
    /// # tokio_test::block_on(async move {
    /// # any_spawner::Executor::init_tokio();
    /// # let _guard = reactive_graph::diagnostics::SpecialNonReactiveZone::enter();
    /// let act = Action::new(|n: &u8| {
    ///     let n = n.to_owned();
    ///     async move { n * 2 }
    /// });
    ///
    /// act.dispatch(3);
    /// assert_eq!(act.input().get(), Some(3));
    ///
    /// // Remember that async functions already return a future if they are
    /// // not `await`ed. You can save keystrokes by leaving out the `async move`
    ///
    /// let act2 = Action::new(|n: &String| yell(n.to_owned()));
    /// act2.dispatch(String::from("i'm in a doctest"));
    /// # tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    ///
    /// // after it resolves
    /// assert_eq!(act2.value().get(), Some("I'M IN A DOCTEST".to_string()));
    ///
    /// async fn yell(n: String) -> String {
    ///     n.to_uppercase()
    /// }
    /// # });
    /// ```
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

    /// Creates a new action, initializing it with the given value.
    ///
    /// This is lazy: it does not run the action function until some value is dispatched.
    ///
    /// The constructor takes a function which will create a new `Future` from some input data.
    /// When the action is dispatched, this `action_fn` will run, and the `Future` it returns will
    /// be spawned.
    ///
    /// The `action_fn` must be `Send + Sync` so that the `ArcAction` is `Send + Sync`. The
    /// `Future` must be `Send` so that it can be moved across threads by the async executor as
    /// needed. In order to be stored in the `Copy` arena, the input and output types should also
    /// be `Send + Sync`.
    #[track_caller]
    pub fn new_with_value<F, Fu>(value: Option<O>, action_fn: F) -> Self
    where
        F: Fn(&I) -> Fu + Send + Sync + 'static,
        Fu: Future<Output = O> + Send + 'static,
    {
        Self {
            inner: StoredValue::new(ArcAction::new_with_value(
                value, action_fn,
            )),
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
        }
    }
}

impl<I, O> Action<I, O, LocalStorage>
where
    I: 'static,
    O: 'static,
{
    /// Creates a new action, which does not require its inputs or outputs to be `Send`. In all other
    /// ways, this is the same as [`Action::new`]. If this action is accessed from outside the
    /// thread on which it was created, it panics.
    #[track_caller]
    pub fn new_local<F, Fu>(action_fn: F) -> Self
    where
        F: Fn(&I) -> Fu + 'static,
        Fu: Future<Output = O> + Send + 'static,
    {
        Self {
            inner: StoredValue::new_local(ArcAction::new_unsync(action_fn)),
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
        }
    }

    /// Creates a new action with the initial value, which does not require its inputs or outputs to be `Send`. In all other
    /// ways, this is the same as [`Action::new_with_value`]. If this action is accessed from outside the
    /// thread on which it was created, it panics.
    #[track_caller]
    pub fn new_local_with_value<F, Fu>(value: Option<O>, action_fn: F) -> Self
    where
        F: Fn(&I) -> Fu + 'static,
        Fu: Future<Output = O> + Send + 'static,
    {
        Self {
            inner: StoredValue::new_local(ArcAction::new_unsync_with_value(
                value, action_fn,
            )),
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
        }
    }
}

impl<I, O, S> Action<I, O, S>
where
    S: Storage<ArcAction<I, O>>,
{
    /// The number of times the action has successfully completed.
    ///
    /// ```rust
    /// # use reactive_graph::actions::*;
    /// # use reactive_graph::prelude::*;
    /// # tokio_test::block_on(async move {
    /// # any_spawner::Executor::init_tokio();
    /// # let _guard = reactive_graph::diagnostics::SpecialNonReactiveZone::enter();
    /// let act = Action::new(|n: &u8| {
    ///     let n = n.to_owned();
    ///     async move { n * 2 }
    /// });
    ///
    /// let version = act.version();
    /// act.dispatch(3);
    /// assert_eq!(version.get(), 0);
    ///
    /// # tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    /// // after it resolves
    /// assert_eq!(version.get(), 1);
    /// # });
    /// ```
    #[track_caller]
    pub fn version(&self) -> RwSignal<usize> {
        let inner = self
            .inner
            .try_with_value(|inner| inner.version())
            .unwrap_or_else(unwrap_signal!(self));
        inner.into()
    }

    /// Whether the action has been dispatched and is currently waiting to resolve.
    ///
    /// ```rust
    /// # use reactive_graph::actions::*;
    /// # use reactive_graph::prelude::*;
    /// # tokio_test::block_on(async move {
    /// # any_spawner::Executor::init_tokio();
    /// # let _guard = reactive_graph::diagnostics::SpecialNonReactiveZone::enter();
    /// let act = Action::new(|n: &u8| {
    ///     let n = n.to_owned();
    ///     async move { n * 2 }
    /// });
    ///
    /// let pending = act.pending();
    /// assert_eq!(pending.get(), false);
    /// act.dispatch(3);
    /// assert_eq!(pending.get(), true);
    ///
    /// # tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    /// // after it resolves
    /// assert_eq!(pending.get(), false);
    /// # });
    /// ```
    #[track_caller]
    pub fn pending(&self) -> Memo<bool> {
        let inner = self
            .inner
            .try_with_value(|inner| inner.pending())
            .unwrap_or_else(unwrap_signal!(self));
        inner.into()
    }
}

impl<I, O, S> Action<I, O, S>
where
    I: 'static,
    S: Storage<ArcAction<I, O>> + Storage<ArcRwSignal<Option<I>>>,
{
    /// The current argument that was dispatched to the async function. This value will
    /// be `Some` while we are waiting for it to resolve, and `None` after it has resolved.
    ///
    /// ```rust
    /// # use reactive_graph::actions::*;
    /// # use reactive_graph::prelude::*;
    /// # tokio_test::block_on(async move {
    /// # any_spawner::Executor::init_tokio();
    /// # let _guard = reactive_graph::diagnostics::SpecialNonReactiveZone::enter();
    /// let act = ArcAction::new(|n: &u8| {
    ///     let n = n.to_owned();
    ///     async move { n * 2 }
    /// });
    ///
    /// let input = act.input();
    /// assert_eq!(input.get(), None);
    /// act.dispatch(3);
    /// assert_eq!(input.get(), Some(3));
    ///
    /// # tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    /// // after it resolves
    /// assert_eq!(input.get(), None);
    /// # });
    /// ```
    #[track_caller]
    pub fn input(&self) -> RwSignal<Option<I>, S> {
        let inner = self
            .inner
            .try_with_value(|inner| inner.input())
            .unwrap_or_else(unwrap_signal!(self));
        inner.into()
    }
}

impl<I, O, S> Action<I, O, S>
where
    O: 'static,
    S: Storage<ArcAction<I, O>> + Storage<ArcRwSignal<Option<O>>>,
{
    /// The most recent return value of the `async` function. This will be `None` before
    /// the action has ever run successfully, and subsequently will always be `Some(_)`,
    /// holding the old value until a new value has been received.
    ///
    /// ```rust
    /// # use reactive_graph::actions::*;
    /// # use reactive_graph::prelude::*;
    /// # tokio_test::block_on(async move {
    /// # any_spawner::Executor::init_tokio();
    /// # let _guard = reactive_graph::diagnostics::SpecialNonReactiveZone::enter();
    /// let act = Action::new(|n: &u8| {
    ///     let n = n.to_owned();
    ///     async move { n * 2 }
    /// });
    ///
    /// let value = act.value();
    /// assert_eq!(value.get(), None);
    /// act.dispatch(3);
    /// assert_eq!(value.get(), None);
    ///
    /// # tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    /// // after it resolves
    /// assert_eq!(value.get(), Some(6));
    /// // dispatch another value, and it still holds the old value
    /// act.dispatch(3);
    /// assert_eq!(value.get(), Some(6));
    /// # });
    /// ```
    #[track_caller]
    pub fn value(&self) -> RwSignal<Option<O>, S> {
        let inner = self
            .inner
            .try_with_value(|inner| inner.value())
            .unwrap_or_else(unwrap_signal!(self));
        inner.into()
    }
}

impl<I, O, S> Action<I, O, S>
where
    I: Send + Sync + 'static,
    O: Send + Sync + 'static,
    S: Storage<ArcAction<I, O>>,
{
    /// Calls the `async` function with a reference to the input type as its argument.
    #[track_caller]
    pub fn dispatch(&self, input: I) {
        self.inner.with_value(|inner| inner.dispatch(input));
    }
}

impl<I, O, S> Action<I, O, S>
where
    I: 'static,
    O: 'static,
    S: Storage<ArcAction<I, O>>,
{
    /// Calls the `async` function with a reference to the input type as its argument.
    #[track_caller]
    pub fn dispatch_local(&self, input: I) {
        self.inner.with_value(|inner| inner.dispatch_local(input));
    }
}

impl<I, O, S> Action<I, O, S>
where
    I: Send + Sync + 'static,
    O: Send + Sync + 'static,
    S: Storage<ArcAction<I, O>>,
{
    /// Creates a new action, which does not require the action itself to be `Send`, but will run
    /// it on the same thread it was created on.
    ///
    /// In all other ways, this is identical to [`Action::new`].
    #[track_caller]
    pub fn new_unsync<F, Fu>(action_fn: F) -> Self
    where
        F: Fn(&I) -> Fu + 'static,
        Fu: Future<Output = O> + 'static,
    {
        Self {
            inner: StoredValue::new_with_storage(ArcAction::new_unsync(
                action_fn,
            )),
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
        }
    }

    /// Creates a new action, which does not require the action itself to be `Send`, but will run
    /// it on the same thread it was created on, and gives an initial value.
    ///
    /// In all other ways, this is identical to [`Action::new`].
    #[track_caller]
    pub fn new_unsync_with_value<F, Fu>(value: Option<O>, action_fn: F) -> Self
    where
        F: Fn(&I) -> Fu + 'static,
        Fu: Future<Output = O> + 'static,
    {
        Self {
            inner: StoredValue::new_with_storage(
                ArcAction::new_unsync_with_value(value, action_fn),
            ),
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
        }
    }
}

impl<I, O, S> DefinedAt for Action<I, O, S> {
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

impl<I, O, S> Clone for Action<I, O, S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<I, O, S> Copy for Action<I, O, S> {}

/// Creates a new action. This is lazy: it does not run the action function until some value
/// is dispatched.
///
/// The constructor takes a function which will create a new `Future` from some input data.
/// When the action is dispatched, this `action_fn` will run, and the `Future` it returns will
/// be spawned.
///
/// The `action_fn` must be `Send + Sync` so that the `ArcAction` is `Send + Sync`. The
/// `Future` must be `Send` so that it can be moved across threads by the async executor as
/// needed. In order to be stored in the `Copy` arena, the input and output types should also
/// be `Send + Sync`.
///
/// ```rust
/// # use reactive_graph::actions::*;
/// # use reactive_graph::prelude::*;
/// # tokio_test::block_on(async move {
/// # any_spawner::Executor::init_tokio();
/// # let _guard = reactive_graph::diagnostics::SpecialNonReactiveZone::enter();
/// let act = create_action(|n: &u8| {
///     let n = n.to_owned();
///     async move { n * 2 }
/// });
///
/// act.dispatch(3);
/// assert_eq!(act.input().get(), Some(3));
///
/// // Remember that async functions already return a future if they are
/// // not `await`ed. You can save keystrokes by leaving out the `async move`
///
/// let act2 = Action::new(|n: &String| yell(n.to_owned()));
/// act2.dispatch(String::from("i'm in a doctest"));
/// # tokio::time::sleep(std::time::Duration::from_millis(10)).await;
///
/// // after it resolves
/// assert_eq!(act2.value().get(), Some("I'M IN A DOCTEST".to_string()));
///
/// async fn yell(n: String) -> String {
///     n.to_uppercase()
/// }
/// # });
/// ```
#[inline(always)]
#[track_caller]
#[deprecated = "This function is being removed to conform to Rust idioms. \
                Please use `Action::new()` instead."]
pub fn create_action<I, O, F, Fu>(action_fn: F) -> Action<I, O>
where
    I: Send + Sync + 'static,
    O: Send + Sync + 'static,
    F: Fn(&I) -> Fu + Send + Sync + 'static,
    Fu: Future<Output = O> + Send + 'static,
{
    Action::new(action_fn)
}
