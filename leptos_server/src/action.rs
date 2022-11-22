use crate::{ServerFn, ServerFnError};
use leptos_reactive::{create_rw_signal, spawn_local, ReadSignal, RwSignal, Scope};
use std::{future::Future, pin::Pin, rc::Rc};

/// An action synchronizes an imperative `async` call to the synchronous reactive system.
///
/// If you’re trying to load data by running an `async` function reactively, you probably
/// want to use a [Resource](leptos_reactive::Resource) instead. If you’re trying to occasionally
/// run an `async` function in response to something like a user clicking a button, you're in the right place.
///
/// ```rust
/// # use leptos_reactive::*;
/// # use leptos_server::create_action;
/// # run_scope(create_runtime(), |cx| {
/// async fn send_new_todo_to_api(task: String) -> usize {
///     // do something...
///     // return a task id
///     42
/// }
/// let save_data = create_action(cx, |task: &String| {
///   // `task` is given as `&String` because its value is available in `input`
///   send_new_todo_to_api(task.clone())
/// });
///
/// // the argument currently running
/// let input = save_data.input;
/// // the most recent returned result
/// let result_of_call = save_data.value;
/// // whether the call is pending
/// let pending = save_data.pending();
/// // how many times the action has run
/// // useful for reactively updating something else in response to a `dispatch` and response
/// let version = save_data.version;
///
/// // before we do anything
/// assert_eq!(input(), None); // no argument yet
/// assert_eq!(pending(), false); // isn't pending a response
/// assert_eq!(result_of_call(), None); // there's no "last value"
/// assert_eq!(version(), 0);
/// # if !cfg!(any(feature = "csr", feature = "hydrate")) {
/// // dispatch the action
/// save_data.dispatch("My todo".to_string());
///
/// // when we're making the call
/// // assert_eq!(input(), Some("My todo".to_string()));
/// // assert_eq!(pending(), true); // is pending
/// // assert_eq!(result_of_call(), None); // has not yet gotten a response
///
/// // after call has resolved
/// assert_eq!(input(), None); // input clears out after resolved
/// assert_eq!(pending(), false); // no longer pending
/// assert_eq!(result_of_call(), Some(42));
/// assert_eq!(version(), 1);
/// # }
/// # });
/// ```
///
/// The input to the `async` function should always be a single value,
/// but it can be of any type. The argument is always passed by reference to the
/// function, because it is stored in [Action::input] as well.
///
/// ```rust
/// # use leptos_reactive::*;
/// # use leptos_server::create_action;
/// # run_scope(create_runtime(), |cx| {
/// // if there's a single argument, just use that
/// let action1 = create_action(cx, |input: &String| {
///   let input = input.clone();
///   async move { todo!() }
/// });
///
/// // if there are no arguments, use the unit type `()`
/// let action2 = create_action(cx, |input: &()| async { todo!() });
///
/// // if there are multiple arguments, use a tuple
/// let action3 = create_action(cx, |input: &(usize, String)| async { todo!() });
/// # });
/// ```
#[derive(Clone)]
pub struct Action<I, O>
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
    #[allow(clippy::complexity)]
    action_fn: Rc<dyn Fn(&I) -> Pin<Box<dyn Future<Output = O>>>>,
}

impl<I, O> Action<I, O>
where
    I: 'static,
    O: 'static,
{
    /// Calls the `async` function with a reference to the input type as its argument.
    pub fn dispatch(&self, input: I) {
        let fut = (self.action_fn)(&input);
        self.input.set(Some(input));
        let input = self.input;
        let version = self.version;
        let pending = self.pending;
        let value = self.value;
        pending.set(true);
        spawn_local(async move {
            let new_value = fut.await;
            input.set(None);
            pending.set(false);
            value.set(Some(new_value));
            version.update(|n| *n += 1);
        })
    }

    /// Whether the action has been dispatched and is currently waiting for its future to be resolved.
    pub fn pending(&self) -> ReadSignal<bool> {
        self.pending.read_only()
    }

    /// The URL associated with the action (typically as part of a server function.)
    /// This enables integration with the `ActionForm` component in `leptos_router`.
    pub fn url(&self) -> Option<&str> {
        self.url.as_deref()
    }

    /// Associates the URL of the given server function with this action.
    /// This enables integration with the `ActionForm` component in `leptos_router`.
    pub fn using_server_fn<T: ServerFn>(mut self) -> Self {
        let prefix = T::prefix();
        self.url = if prefix.is_empty() {
            Some(T::url().to_string())
        } else {
            Some(prefix.to_string() + "/" + T::url())
        };
        self
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
/// # use leptos_reactive::*;
/// # use leptos_server::create_action;
/// # run_scope(create_runtime(), |cx| {
/// async fn send_new_todo_to_api(task: String) -> usize {
///     // do something...
///     // return a task id
///     42
/// }
/// let save_data = create_action(cx, |task: &String| {
///   // `task` is given as `&String` because its value is available in `input`
///   send_new_todo_to_api(task.clone())
/// });
///
/// // the argument currently running
/// let input = save_data.input;
/// // the most recent returned result
/// let result_of_call = save_data.value;
/// // whether the call is pending
/// let pending = save_data.pending();
/// // how many times the action has run
/// // useful for reactively updating something else in response to a `dispatch` and response
/// let version = save_data.version;
///
/// // before we do anything
/// assert_eq!(input(), None); // no argument yet
/// assert_eq!(pending(), false); // isn't pending a response
/// assert_eq!(result_of_call(), None); // there's no "last value"
/// assert_eq!(version(), 0);
/// # if !cfg!(any(feature = "csr", feature = "hydrate")) {
/// // dispatch the action
/// save_data.dispatch("My todo".to_string());
///
/// // when we're making the call
/// // assert_eq!(input(), Some("My todo".to_string()));
/// // assert_eq!(pending(), true); // is pending
/// // assert_eq!(result_of_call(), None); // has not yet gotten a response
///
/// // after call has resolved
/// assert_eq!(input(), None); // input clears out after resolved
/// assert_eq!(pending(), false); // no longer pending
/// assert_eq!(result_of_call(), Some(42));
/// assert_eq!(version(), 1);
/// # }
/// # });
/// ```
///
/// The input to the `async` function should always be a single value,
/// but it can be of any type. The argument is always passed by reference to the
/// function, because it is stored in [Action::input] as well.
///
/// ```rust
/// # use leptos_reactive::*;
/// # use leptos_server::create_action;
/// # run_scope(create_runtime(), |cx| {
/// // if there's a single argument, just use that
/// let action1 = create_action(cx, |input: &String| {
///   let input = input.clone();
///   async move { todo!() }
/// });
///
/// // if there are no arguments, use the unit type `()`
/// let action2 = create_action(cx, |input: &()| async { todo!() });
///
/// // if there are multiple arguments, use a tuple
/// let action3 = create_action(cx, |input: &(usize, String)| async { todo!() });
/// # });
/// ```
pub fn create_action<I, O, F, Fu>(cx: Scope, action_fn: F) -> Action<I, O>
where
    I: 'static,
    O: 'static,
    F: Fn(&I) -> Fu + 'static,
    Fu: Future<Output = O> + 'static,
{
    let version = create_rw_signal(cx, 0);
    let input = create_rw_signal(cx, None);
    let value = create_rw_signal(cx, None);
    let pending = create_rw_signal(cx, false);
    let action_fn = Rc::new(move |input: &I| {
        let fut = action_fn(input);
        Box::pin(async move { fut.await }) as Pin<Box<dyn Future<Output = O>>>
    });

    Action {
        version,
        url: None,
        input,
        value,
        pending,
        action_fn,
    }
}

/// Creates an [Action] that can be used to call a server function.
///
/// ```rust
/// # use leptos_reactive::*;
/// # use leptos_server::{create_server_action, ServerFnError, ServerFn};
/// # use leptos_macro::server;
///
/// #[server(MyServerFn)]
/// async fn my_server_fn() -> Result<(), ServerFnError> {
///   todo!()
/// }
///
/// # run_scope(create_runtime(), |cx| {
/// let my_server_action = create_server_action::<MyServerFn>(cx);
/// # });
/// ```
pub fn create_server_action<S>(cx: Scope) -> Action<S, Result<S::Output, ServerFnError>>
where
    S: Clone + ServerFn,
{
    #[cfg(feature = "ssr")]
    let c = move |args: &S| S::call_fn(args.clone(), cx);
    #[cfg(not(feature = "ssr"))]
    let c = move |args: &S| S::call_fn_client(args.clone(), cx);
    create_action(cx, c).using_server_fn::<S>()
}
