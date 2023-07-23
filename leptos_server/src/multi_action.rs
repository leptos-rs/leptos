use crate::{ServerFn, ServerFnError};
use leptos_reactive::{
    create_rw_signal, signal_prelude::*, spawn_local, store_value, ReadSignal,
    RwSignal, Scope, StoredValue,
};
use std::{future::Future, pin::Pin, rc::Rc};

/// An action that synchronizes multiple imperative `async` calls to the reactive system,
/// tracking the progress of each one.
///
/// Where an [Action](crate::Action) fires a single call, a `MultiAction` allows you to
/// keep track of multiple in-flight actions.
///
/// If you’re trying to load data by running an `async` function reactively, you probably
/// want to use a [Resource](leptos_reactive::Resource) instead. If you’re trying to occasionally
/// run an `async` function in response to something like a user adding a task to a todo list,
/// you’re in the right place.
///
/// ```rust
/// # use leptos::*;
/// # run_scope(create_runtime(), |cx| {
/// async fn send_new_todo_to_api(task: String) -> usize {
///   // do something...
///   // return a task id
///   42
/// }
/// let add_todo = create_multi_action(cx, |task: &String| {
///   // `task` is given as `&String` because its value is available in `input`
///   send_new_todo_to_api(task.clone())
/// });
///
/// # if false {
/// add_todo.dispatch("Buy milk".to_string());
/// add_todo.dispatch("???".to_string());
/// add_todo.dispatch("Profit!!!".to_string());
/// # }
///
/// # });
/// ```
///
/// The input to the `async` function should always be a single value,
/// but it can be of any type. The argument is always passed by reference to the
/// function, because it is stored in [Submission::input] as well.
///
/// ```rust
/// # use leptos::*;
/// # run_scope(create_runtime(), |cx| {
/// // if there's a single argument, just use that
/// let action1 = create_multi_action(cx, |input: &String| {
///     let input = input.clone();
///     async move { todo!() }
/// });
///
/// // if there are no arguments, use the unit type `()`
/// let action2 = create_multi_action(cx, |input: &()| async { todo!() });
///
/// // if there are multiple arguments, use a tuple
/// let action3 =
///     create_multi_action(cx, |input: &(usize, String)| async { todo!() });
/// # });
/// ```
pub struct MultiAction<I, O>(StoredValue<MultiActionState<I, O>>)
where
    I: 'static,
    O: 'static;

impl<I, O> MultiAction<I, O>
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

impl<I, O> Copy for MultiAction<I, O>
where
    I: 'static,
    O: 'static,
{
}

impl<I, O> MultiAction<I, O>
where
    I: 'static,
    O: 'static,
{
    /// Calls the `async` function with a reference to the input type as its argument.
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        tracing::instrument(level = "trace", skip_all,)
    )]
    pub fn dispatch(&self, input: I) {
        self.0.with_value(|a| a.dispatch(input))
    }

    /// The set of all submissions to this multi-action.
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        tracing::instrument(level = "trace", skip_all,)
    )]
    pub fn submissions(&self) -> ReadSignal<Vec<Submission<I, O>>> {
        self.0.with_value(|a| a.submissions())
    }

    /// The URL associated with the action (typically as part of a server function.)
    /// This enables integration with the `MultiActionForm` component in `leptos_router`.
    pub fn url(&self) -> Option<String> {
        self.0.with_value(|a| a.url.as_ref().cloned())
    }

    /// How many times an action has successfully resolved.
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        tracing::instrument(level = "trace", skip_all,)
    )]
    pub fn version(&self) -> RwSignal<usize> {
        self.0.with_value(|a| a.version)
    }

    /// Associates the URL of the given server function with this action.
    /// This enables integration with the `MultiActionForm` component in `leptos_router`.
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        tracing::instrument(level = "trace", skip_all,)
    )]
    pub fn using_server_fn<T: ServerFn>(self) -> Self {
        let prefix = T::prefix();
        self.0.update_value(|a| {
            a.url = if prefix.is_empty() {
                Some(T::url().to_string())
            } else {
                Some(prefix.to_string() + "/" + T::url())
            };
        });

        self
    }
}

struct MultiActionState<I, O>
where
    I: 'static,
    O: 'static,
{
    cx: Scope,
    /// How many times an action has successfully resolved.
    pub version: RwSignal<usize>,
    submissions: RwSignal<Vec<Submission<I, O>>>,
    url: Option<String>,
    #[allow(clippy::complexity)]
    action_fn: Rc<dyn Fn(&I) -> Pin<Box<dyn Future<Output = O>>>>,
}

/// An action that has been submitted by dispatching it to a [MultiAction](crate::MultiAction).
pub struct Submission<I, O>
where
    I: 'static,
    O: 'static,
{
    /// The current argument that was dispatched to the `async` function.
    /// `Some` while we are waiting for it to resolve, `None` if it has resolved.
    pub input: RwSignal<Option<I>>,
    /// The most recent return value of the `async` function.
    pub value: RwSignal<Option<O>>,
    pub(crate) pending: RwSignal<bool>,
    /// Controls this submission has been canceled.
    pub canceled: RwSignal<bool>,
}

impl<I, O> Clone for Submission<I, O> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<I, O> Copy for Submission<I, O> {}

impl<I, O> Submission<I, O>
where
    I: 'static,
    O: 'static,
{
    /// Whether this submission is currently waiting to resolve.
    pub fn pending(&self) -> ReadSignal<bool> {
        self.pending.read_only()
    }

    /// Cancels the submission, preventing it from resolving.
    pub fn cancel(&self) {
        self.canceled.set(true);
    }
}

impl<I, O> MultiActionState<I, O>
where
    I: 'static,
    O: 'static,
{
    /// Calls the `async` function with a reference to the input type as its argument.
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        tracing::instrument(level = "trace", skip_all,)
    )]
    pub fn dispatch(&self, input: I) {
        let cx = self.cx;
        let fut = (self.action_fn)(&input);

        let submission = Submission {
            input: create_rw_signal(cx, Some(input)),
            value: create_rw_signal(cx, None),
            pending: create_rw_signal(cx, true),
            canceled: create_rw_signal(cx, false),
        };

        self.submissions.update(|subs| subs.push(submission));

        let canceled = submission.canceled;
        let input = submission.input;
        let pending = submission.pending;
        let value = submission.value;
        let version = self.version;

        spawn_local(async move {
            let new_value = fut.await;
            let canceled = cx.untrack(move || canceled.get());
            if !canceled {
                value.set(Some(new_value));
            }
            input.set(None);
            pending.set(false);
            version.update(|n| *n += 1);
        })
    }

    /// The set of all submissions to this multi-action.
    pub fn submissions(&self) -> ReadSignal<Vec<Submission<I, O>>> {
        self.submissions.read_only()
    }
}

/// Creates an [MultiAction] to synchronize an imperative `async` call to the synchronous reactive system.
///
/// If you’re trying to load data by running an `async` function reactively, you probably
/// want to use a [create_resource](leptos_reactive::create_resource) instead. If you’re trying
/// to occasionally run an `async` function in response to something like a user clicking a button,
/// you're in the right place.
///
/// ```rust
/// # use leptos::*;
/// # run_scope(create_runtime(), |cx| {
/// async fn send_new_todo_to_api(task: String) -> usize {
///   // do something...
///   // return a task id
///   42
/// }
/// let add_todo = create_multi_action(cx, |task: &String| {
///   // `task` is given as `&String` because its value is available in `input`
///   send_new_todo_to_api(task.clone())
/// });
/// # if false {
///
/// add_todo.dispatch("Buy milk".to_string());
/// add_todo.dispatch("???".to_string());
/// add_todo.dispatch("Profit!!!".to_string());
///
/// assert_eq!(add_todo.submissions().get().len(), 3);
/// # }
/// # });
/// ```
///
/// The input to the `async` function should always be a single value,
/// but it can be of any type. The argument is always passed by reference to the
/// function, because it is stored in [Submission::input] as well.
///
/// ```rust
/// # use leptos::*;
/// # run_scope(create_runtime(), |cx| {
/// // if there's a single argument, just use that
/// let action1 = create_multi_action(cx, |input: &String| {
///     let input = input.clone();
///     async move { todo!() }
/// });
///
/// // if there are no arguments, use the unit type `()`
/// let action2 = create_multi_action(cx, |input: &()| async { todo!() });
///
/// // if there are multiple arguments, use a tuple
/// let action3 =
///     create_multi_action(cx, |input: &(usize, String)| async { todo!() });
/// # });
/// ```
#[cfg_attr(
    any(debug_assertions, feature = "ssr"),
    tracing::instrument(level = "trace", skip_all,)
)]
pub fn create_multi_action<I, O, F, Fu>(
    cx: Scope,
    action_fn: F,
) -> MultiAction<I, O>
where
    I: 'static,
    O: 'static,
    F: Fn(&I) -> Fu + 'static,
    Fu: Future<Output = O> + 'static,
{
    let version = create_rw_signal(cx, 0);
    let submissions = create_rw_signal(cx, Vec::new());
    let action_fn = Rc::new(move |input: &I| {
        let fut = action_fn(input);
        Box::pin(fut) as Pin<Box<dyn Future<Output = O>>>
    });

    MultiAction(store_value(
        cx,
        MultiActionState {
            cx,
            version,
            submissions,
            url: None,
            action_fn,
        },
    ))
}

/// Creates an [MultiAction] that can be used to call a server function.
///
/// ```rust
/// # use leptos::*;
///
/// #[server(MyServerFn)]
/// async fn my_server_fn() -> Result<(), ServerFnError> {
///     todo!()
/// }
///
/// # run_scope(create_runtime(), |cx| {
/// let my_server_multi_action = create_server_multi_action::<MyServerFn>(cx);
/// # });
/// ```
#[cfg_attr(
    any(debug_assertions, feature = "ssr"),
    tracing::instrument(level = "trace", skip_all,)
)]
pub fn create_server_multi_action<S>(
    cx: Scope,
) -> MultiAction<S, Result<S::Output, ServerFnError>>
where
    S: Clone + ServerFn,
{
    #[cfg(feature = "ssr")]
    let c = move |args: &S| S::call_fn(args.clone(), cx);
    #[cfg(not(feature = "ssr"))]
    let c = move |args: &S| S::call_fn_client(args.clone(), cx);
    create_multi_action(cx, c).using_server_fn::<S>()
}
