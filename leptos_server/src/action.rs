//use crate::{ServerFn, ServerFnError};
#[cfg(debug_assertions)]
use leptos_reactive::console_warn;
use leptos_reactive::{
    is_suppressing_resource_load, signal_prelude::*, spawn_local, store_value,
    try_batch, use_context, StoredValue,
};
use server_fn::{error::ServerFnUrlError, ServerFn, ServerFnError};
use std::{cell::Cell, future::Future, pin::Pin, rc::Rc};

/// An action synchronizes an imperative `async` call to the synchronous reactive system.
///
/// If you’re trying to load data by running an `async` function reactively, you probably
/// want to use a [Resource](leptos_reactive::Resource) instead. If you’re trying to occasionally
/// run an `async` function in response to something like a user clicking a button, you're in the right place.
///
/// ```rust
/// # use leptos::*;
/// # let runtime = create_runtime();
/// async fn send_new_todo_to_api(task: String) -> usize {
///     // do something...
///     // return a task id
///     42
/// }
/// let save_data = create_action(|task: &String| {
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
/// # if false {
/// // dispatch the action
/// save_data.dispatch("My todo".to_string());
///
/// // when we're making the call
/// // assert_eq!(input.get(), Some("My todo".to_string()));
/// // assert_eq!(pending.get(), true); // is pending
/// // assert_eq!(result_of_call.get(), None); // has not yet gotten a response
///
/// // after call has resolved
/// assert_eq!(input.get(), None); // input clears out after resolved
/// assert_eq!(pending.get(), false); // no longer pending
/// assert_eq!(result_of_call.get(), Some(42));
/// assert_eq!(version.get(), 1);
/// # };
/// # runtime.dispose();
/// ```
///
/// The input to the `async` function should always be a single value,
/// but it can be of any type. The argument is always passed by reference to the
/// function, because it is stored in [Action::input] as well.
///
/// ```rust
/// # use leptos::*;
/// # let runtime = create_runtime();
/// // if there's a single argument, just use that
/// let action1 = create_action(|input: &String| {
///     let input = input.clone();
///     async move { todo!() }
/// });
///
/// // if there are no arguments, use the unit type `()`
/// let action2 = create_action(|input: &()| async { todo!() });
///
/// // if there are multiple arguments, use a tuple
/// let action3 = create_action(|input: &(usize, String)| async { todo!() });
/// # runtime.dispose();
/// ```
pub struct Action<I, O>(StoredValue<ActionState<I, O>>)
where
    I: 'static,
    O: 'static;

impl<I, O> Action<I, O>
where
    I: 'static,
    O: 'static,
{
    /// Calls the `async` function with a reference to the input type as its argument.
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        tracing::instrument(level = "trace", skip_all,)
    )]
    #[track_caller]
    pub fn dispatch(&self, input: I) {
        #[cfg(debug_assertions)]
        let loc = std::panic::Location::caller();

        self.0.with_value(|a| {
            a.dispatch(
                input,
                #[cfg(debug_assertions)]
                loc,
            )
        })
    }

    /// Create an [Action].
    ///
    /// [Action] is a type of [Signal] which represent imperative calls to
    /// an asynchronous function. Where a [Resource](leptos_reactive::Resource) is driven as a function
    /// of a [Signal], [Action]s are [Action::dispatch]ed by events or handlers.
    ///
    /// ```rust
    /// # use leptos::*;
    /// # let runtime = create_runtime();
    ///
    /// let act = Action::new(|n: &u8| {
    ///     let n = n.to_owned();
    ///     async move { n * 2 }
    /// });
    /// # if false {
    /// act.dispatch(3);
    /// assert_eq!(act.value().get(), Some(6));
    ///
    /// // Remember that async functions already return a future if they are
    /// // not `await`ed. You can save keystrokes by leaving out the `async move`
    ///
    /// let act2 = Action::new(|n: &String| yell(n.to_owned()));
    /// act2.dispatch(String::from("i'm in a doctest"));
    /// assert_eq!(act2.value().get(), Some("I'M IN A DOCTEST".to_string()));
    /// # }
    ///
    /// async fn yell(n: String) -> String {
    ///     n.to_uppercase()
    /// }
    ///
    /// # runtime.dispose();
    /// ```
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        tracing::instrument(level = "trace", skip_all,)
    )]
    pub fn new<F, Fu>(action_fn: F) -> Self
    where
        F: Fn(&I) -> Fu + 'static,
        Fu: Future<Output = O> + 'static,
    {
        let version = create_rw_signal(0);
        let input = create_rw_signal(None);
        let value = create_rw_signal(None);
        let pending = create_rw_signal(false);
        let pending_dispatches = Rc::new(Cell::new(0));
        let action_fn = Rc::new(move |input: &I| {
            let fut = action_fn(input);
            Box::pin(fut) as Pin<Box<dyn Future<Output = O>>>
        });

        Action(store_value(ActionState {
            version,
            url: None,
            input,
            value,
            pending,
            pending_dispatches,
            action_fn,
        }))
    }

    /// Whether the action has been dispatched and is currently waiting for its future to be resolved.
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        tracing::instrument(level = "trace", skip_all,)
    )]
    pub fn pending(&self) -> ReadSignal<bool> {
        self.0.with_value(|a| a.pending.read_only())
    }

    /// Updates whether the action is currently pending. If the action has been dispatched
    /// multiple times, and some of them are still pending, it will *not* update the `pending`
    /// signal.
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        tracing::instrument(level = "trace", skip_all,)
    )]
    pub fn set_pending(&self, pending: bool) {
        self.0.try_with_value(|a| {
            let pending_dispatches = &a.pending_dispatches;
            let still_pending = {
                pending_dispatches.set(if pending {
                    pending_dispatches.get().wrapping_add(1)
                } else {
                    pending_dispatches.get().saturating_sub(1)
                });
                pending_dispatches.get()
            };
            if still_pending == 0 {
                a.pending.set(false);
            } else {
                a.pending.set(true);
            }
        });
    }

    /// The URL associated with the action (typically as part of a server function.)
    /// This enables integration with the `ActionForm` component in `leptos_router`.
    pub fn url(&self) -> Option<String> {
        self.0.with_value(|a| a.url.as_ref().cloned())
    }

    /// How many times the action has successfully resolved.
    pub fn version(&self) -> RwSignal<usize> {
        self.0.with_value(|a| a.version)
    }

    /// The current argument that was dispatched to the `async` function.
    /// `Some` while we are waiting for it to resolve, `None` if it has resolved.
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        tracing::instrument(level = "trace", skip_all,)
    )]
    pub fn input(&self) -> RwSignal<Option<I>> {
        self.0.with_value(|a| a.input)
    }

    /// The most recent return value of the `async` function.
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        tracing::instrument(level = "trace", skip_all,)
    )]
    pub fn value(&self) -> RwSignal<Option<O>> {
        self.0.with_value(|a| a.value)
    }
}

impl<I> Action<I, Result<I::Output, ServerFnError<I::Error>>>
where
    I: ServerFn + 'static,
{
    /// Create an [Action] to imperatively call a [server](leptos_macro::server) function.
    ///
    /// The struct representing your server function's arguments should be
    /// provided to the [Action]. Unless specified as an argument to the server
    /// macro, the generated struct is your function's name converted to CamelCase.
    ///
    /// ```rust
    /// # // Not in a localset, so this would always panic.
    /// # if false {
    /// # use leptos::*;
    /// # let rt = create_runtime();
    ///
    /// // The type argument can be on the right of the equal sign.
    /// let act = Action::<Add, _>::server();
    /// let args = Add { lhs: 5, rhs: 7 };
    /// act.dispatch(args);
    /// assert_eq!(act.value().get(), Some(Ok(12)));
    ///
    /// // Or on the left of the equal sign.
    /// let act: Action<Sub, _> = Action::server();
    /// let args = Sub { lhs: 20, rhs: 5 };
    /// act.dispatch(args);
    /// assert_eq!(act.value().get(), Some(Ok(15)));
    ///
    /// let not_dispatched = Action::<Add, _>::server();
    /// assert_eq!(not_dispatched.value().get(), None);
    ///
    /// #[server]
    /// async fn add(lhs: u8, rhs: u8) -> Result<u8, ServerFnError> {
    ///     Ok(lhs + rhs)
    /// }
    ///
    /// #[server]
    /// async fn sub(lhs: u8, rhs: u8) -> Result<u8, ServerFnError> {
    ///     Ok(lhs - rhs)
    /// }
    ///
    /// # rt.dispose();
    /// # }
    /// ```
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        tracing::instrument(level = "trace", skip_all,)
    )]
    pub fn server() -> Action<I, Result<I::Output, ServerFnError<I::Error>>>
    where
        I: ServerFn + Clone,
        I::Error: Clone + 'static,
    {
        // The server is able to call the function directly
        #[cfg(feature = "ssr")]
        let action_function = |args: &I| I::run_body(args.clone());

        // When not on the server send a fetch to request the fn call.
        #[cfg(not(feature = "ssr"))]
        let action_function = |args: &I| I::run_on_client(args.clone());

        // create the action
        Action::new(action_function).using_server_fn()
    }

    /// Associates the URL of the given server function with this action.
    /// This enables integration with the `ActionForm` component in `leptos_router`.
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        tracing::instrument(level = "trace", skip_all,)
    )]
    pub fn using_server_fn(self) -> Self
    where
        I::Error: Clone + 'static,
    {
        let url = I::url();
        let action_error = use_context::<Rc<ServerFnUrlError<I::Error>>>()
            .and_then(|err| {
                if err.path() == url {
                    Some(err.error().clone())
                } else {
                    None
                }
            });
        self.0.update_value(|state| {
            if let Some(err) = action_error {
                state.value.set_untracked(Some(Err(err)));
            }
            state.url = Some(url.to_string());
        });
        self
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

struct ActionState<I, O>
where
    I: 'static,
    O: 'static,
{
    /// How many times the action has successfully resolved.
    pub version: RwSignal<usize>,
    /// The current argument that was dispatched to the `async` function.
    /// `Some` while we are waiting for it to resolve, `None` if it has resolved.
    pub input: RwSignal<Option<I>>,
    /// The most recent return value of the `async` function.
    pub value: RwSignal<Option<O>>,
    pending: RwSignal<bool>,
    url: Option<String>,
    /// How many dispatched actions are still pending.
    pending_dispatches: Rc<Cell<usize>>,
    #[allow(clippy::complexity)]
    action_fn: Rc<dyn Fn(&I) -> Pin<Box<dyn Future<Output = O>>>>,
}

impl<I, O> ActionState<I, O>
where
    I: 'static,
    O: 'static,
{
    /// Calls the `async` function with a reference to the input type as its argument.
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        tracing::instrument(level = "trace", skip_all,)
    )]
    pub fn dispatch(
        &self,
        input: I,
        #[cfg(debug_assertions)] loc: &'static std::panic::Location<'static>,
    ) {
        if !is_suppressing_resource_load() {
            let fut = (self.action_fn)(&input);
            self.input.set(Some(input));
            let input = self.input;
            let version = self.version;
            let pending = self.pending;
            let pending_dispatches = Rc::clone(&self.pending_dispatches);
            let value = self.value;
            pending.set(true);
            pending_dispatches.set(pending_dispatches.get().wrapping_add(1));
            spawn_local(async move {
                let new_value = fut.await;
                let res = try_batch(move || {
                    value.set(Some(new_value));
                    input.set(None);
                    version.update(|n| *n += 1);
                    pending_dispatches
                        .set(pending_dispatches.get().saturating_sub(1));
                    if pending_dispatches.get() == 0 {
                        pending.set(false);
                    }
                });

                if res.is_err() {
                    #[cfg(debug_assertions)]
                    console_warn(&format!(
                        "At {loc}, you are dispatching an action in a runtime \
                         that has already been disposed. This may be because \
                         you are calling `.dispatch()` in the body of a \
                         component, during initial server-side rendering. If \
                         that's the case, you should probably be using \
                         `create_resource` instead of `create_action`."
                    ));
                }
            })
        }
    }
}

/// Creates an [Action] to synchronize an imperative `async` call to the synchronous reactive system.
///
/// If you’re trying to load data by running an `async` function reactively, you probably
/// want to use a [create_resource](leptos_reactive::create_resource) instead. If you’re trying
/// to occasionally run an `async` function in response to something like a user clicking a button,
/// you're in the right place.
///
/// ```rust
/// # use leptos::*;
/// # let runtime = create_runtime();
/// async fn send_new_todo_to_api(task: String) -> usize {
///     // do something...
///     // return a task id
///     42
/// }
/// let save_data = create_action(|task: &String| {
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
/// # if false {
/// // dispatch the action
/// save_data.dispatch("My todo".to_string());
///
/// // when we're making the call
/// // assert_eq!(input.get(), Some("My todo".to_string()));
/// // assert_eq!(pending.get(), true); // is pending
/// // assert_eq!(result_of_call.get(), None); // has not yet gotten a response
///
/// // after call has resolved
/// assert_eq!(input.get(), None); // input clears out after resolved
/// assert_eq!(pending.get(), false); // no longer pending
/// assert_eq!(result_of_call.get(), Some(42));
/// assert_eq!(version.get(), 1);
/// # }
/// # runtime.dispose();
/// ```
///
/// The input to the `async` function should always be a single value,
/// but it can be of any type. The argument is always passed by reference to the
/// function, because it is stored in [Action::input] as well.
///
/// ```rust
/// # use leptos::*;
/// # let runtime = create_runtime();
/// // if there's a single argument, just use that
/// let action1 = create_action(|input: &String| {
///     let input = input.clone();
///     async move { todo!() }
/// });
///
/// // if there are no arguments, use the unit type `()`
/// let action2 = create_action(|input: &()| async { todo!() });
///
/// // if there are multiple arguments, use a tuple
/// let action3 = create_action(|input: &(usize, String)| async { todo!() });
/// # runtime.dispose();
/// ```
#[cfg_attr(
    any(debug_assertions, feature = "ssr"),
    tracing::instrument(level = "trace", skip_all,)
)]
pub fn create_action<I, O, F, Fu>(action_fn: F) -> Action<I, O>
where
    I: 'static,
    O: 'static,
    F: Fn(&I) -> Fu + 'static,
    Fu: Future<Output = O> + 'static,
{
    Action::new(action_fn)
}

/// Creates an [Action] that can be used to call a server function.
///
/// ```rust
/// # use leptos::*;
///
/// #[server(MyServerFn)]
/// async fn my_server_fn() -> Result<(), ServerFnError> {
///     todo!()
/// }
///
/// # let runtime = create_runtime();
/// let my_server_action = create_server_action::<MyServerFn>();
/// # runtime.dispose();
/// ```
#[cfg_attr(
    any(debug_assertions, feature = "ssr"),
    tracing::instrument(level = "trace", skip_all,)
)]
pub fn create_server_action<S>(
) -> Action<S, Result<S::Output, ServerFnError<S::Error>>>
where
    S: Clone + ServerFn,
    S::Error: Clone + 'static,
{
    Action::<S, _>::server()
}
