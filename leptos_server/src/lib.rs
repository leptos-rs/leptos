#![deny(missing_docs)]

//! # Leptos Server Functions
//!
//! This package is based on a simple idea: sometimes it’s useful to write functions
//! that will only run on the server, and call them from the client.
//!
//! If you’re creating anything beyond a toy app, you’ll need to do this all the time:
//! reading from or writing to a database that only runs on the server, running expensive
//! computations using libraries you don’t want to ship down to the client, accessing
//! APIs that need to be called from the server rather than the client for CORS reasons
//! or because you need a secret API key that’s stored on the server and definitely
//! shouldn’t be shipped down to a user’s browser.
//!
//! Traditionally, this is done by separating your server and client code, and by setting
//! up something like a REST API or GraphQL API to allow your client to fetch and mutate
//! data on the server. This is fine, but it requires you to write and maintain your code
//! in multiple separate places (client-side code for fetching, server-side functions to run),
//! as well as creating a third thing to manage, which is the API contract between the two.
//!
//! This package provides two simple primitives that allow you instead to write co-located,
//! isomorphic server functions. (*Co-located* means you can write them in your app code so
//! that they are “located alongside” the client code that calls them, rather than separating
//! the client and server sides. *Isomorphic* means you can call them from the client as if
//! you were simply calling a function; the function call has the “same shape” on the client
//! as it does on the server.)
//!
//! ### `#[server]`
//!
//! The `#[server]` macro allows you to annotate a function to indicate that it should only run
//! on the server (i.e., when you have an `ssr` feature in your crate that is enabled).
//!
//! ```rust,ignore
//! #[server(ReadFromDB)]
//! async fn read_posts(how_many: usize, query: String) -> Result<Vec<Posts>, ServerFnError> {
//!   // do some server-only work here to access the database
//!   let posts = ...;
//!   Ok(posts)
//! }
//!
//! // call the function
//! spawn_local(async {
//!   let posts = read_posts(3, "my search".to_string()).await;
//!   log::debug!("posts = {posts{:#?}");
//! })
//! ```
//!
//! If you call this function from the client, it will serialize the function arguments and `POST`
//! them to the server as if they were the inputs in `<form method="POST">`.
//!
//! Here’s what you need to remember:
//! - **Server functions must be `async`.** Even if the work being done inside the function body
//!   can run synchronously on the server, from the client’s perspective it involves an asynchronous
//!   function call.
//! - **Server functions must return `Result<T, ServerFnError>`.** Even if the work being done
//!   inside the function body can’t fail, the processes of serialization/deserialization and the
//!   network call are fallible.
//! - **Server function arguments and return types must be [Serializable](leptos_reactive::Serializable).**
//!   This should be fairly obvious: we have to serialize arguments to send them to the server, and we
//!   need to deserialize the result to return it to the client.

pub use form_urlencoded;
use leptos_reactive::*;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{future::Future, pin::Pin, rc::Rc};
use thiserror::Error;

#[cfg(any(feature = "ssr", doc))]
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

#[cfg(any(feature = "ssr", doc))]
type ServerFnTraitObj =
    dyn Fn(&[u8]) -> Pin<Box<dyn Future<Output = Result<String, ServerFnError>>>> + Send + Sync;

#[cfg(any(feature = "ssr", doc))]
lazy_static::lazy_static! {
    static ref REGISTERED_SERVER_FUNCTIONS: Arc<RwLock<HashMap<&'static str, Arc<ServerFnTraitObj>>>> = Default::default();
}

/// Attempts to find a server function registered at the given path.
///
/// This can be used by a server to handle the requests, as in the following example (using `actix-web`)
///
/// ```rust, ignore
/// #[post("{tail:.*}")]
/// async fn handle_server_fns(
///     req: HttpRequest,
///     params: web::Path<String>,
///     body: web::Bytes,
/// ) -> impl Responder {
///     let path = params.into_inner();
///     let accept_header = req
///         .headers()
///         .get("Accept")
///         .and_then(|value| value.to_str().ok());
///
///     if let Some(server_fn) = server_fn_by_path(path.as_str()) {
///         let body: &[u8] = &body;
///         match server_fn(&body).await {
///             Ok(serialized) => {
///                 // if this is Accept: application/json then send a serialized JSON response
///                 if let Some("application/json") = accept_header {
///                     HttpResponse::Ok().body(serialized)
///                 }
///                 // otherwise, it's probably a <form> submit or something: redirect back to the referrer
///                 else {
///                     HttpResponse::SeeOther()
///                         .insert_header(("Location", "/"))
///                         .content_type("application/json")
///                         .body(serialized)
///                 }
///             }
///             Err(e) => {
///                 eprintln!("server function error: {e:#?}");
///                 HttpResponse::InternalServerError().body(e.to_string())
///             }
///         }
///     } else {
///         HttpResponse::BadRequest().body(format!("Could not find a server function at that route."))
///     }
/// }
/// ```
#[cfg(any(feature = "ssr", doc))]
pub fn server_fn_by_path(path: &str) -> Option<Arc<ServerFnTraitObj>> {
    REGISTERED_SERVER_FUNCTIONS
        .read()
        .ok()
        .and_then(|fns| fns.get(path).cloned())
}

/// Defines a "server function." A server function can be called from the server or the client,
/// but the body of its code will only be run on the server, i.e., if a crate feature `ssr` is enabled.
///
/// (This follows the same convention as the Leptos framework's distinction between `ssr` for server-side rendering,
/// and `csr` and `hydrate` for client-side rendering and hydration, respectively.)
///
/// Server functions are created using the `server` macro.
///
/// The function should be registered by calling `ServerFn::register()`. The set of server functions
/// can be queried on the server for routing purposes by calling [server_fn_by_path].
///
/// Technically, the trait is implemented on a type that describes the server function's arguments.
pub trait ServerFn
where
    Self: Serialize + DeserializeOwned + Sized + 'static,
{
    /// The return type of the function.
    type Output: Serializable;

    /// URL prefix that should be prepended by the client to the generated URL.
    fn prefix() -> &'static str;

    /// The path at which the server function can be reached on the server.
    fn url() -> &'static str;

    /// Runs the function on the server.
    #[cfg(any(feature = "ssr", doc))]
    fn call_fn(self) -> Pin<Box<dyn Future<Output = Result<Self::Output, ServerFnError>> + Send>>;

    /// Runs the function on the client by sending an HTTP request to the server.
    #[cfg(any(not(feature = "ssr"), doc))]
    fn call_fn_client(self) -> Pin<Box<dyn Future<Output = Result<Self::Output, ServerFnError>>>>;

    /// Registers the server function, allowing the server to query it by URL.
    #[cfg(any(feature = "ssr", doc))]
    fn register() -> Result<(), ServerFnError> {
        // create the handler for this server function
        // takes a String -> returns its async value
        let run_server_fn = Arc::new(|data: &[u8]| {
            // decode the args
            let value = serde_urlencoded::from_bytes::<Self>(data)
                .map_err(|e| ServerFnError::Deserialization(e.to_string()));
            Box::pin(async move {
                let value = match value {
                    Ok(v) => v,
                    Err(e) => return Err(e),
                };

                // call the function
                let result = match value.call_fn().await {
                    Ok(r) => r,
                    Err(e) => return Err(e),
                };

                // serialize the output
                let result = match result
                    .to_json()
                    .map_err(|e| ServerFnError::Serialization(e.to_string()))
                {
                    Ok(r) => r,
                    Err(e) => return Err(e),
                };

                Ok(result)
            }) as Pin<Box<dyn Future<Output = Result<String, ServerFnError>>>>
        });

        // store it in the hashmap
        let mut write = REGISTERED_SERVER_FUNCTIONS
            .write()
            .map_err(|e| ServerFnError::Registration(e.to_string()))?;
        write.insert(Self::url(), run_server_fn);

        Ok(())
    }
}

/// Type for errors that can occur when using server functions.
#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum ServerFnError {
    /// Error while trying to register the server function (only occurs in case of poisoned RwLock).
    #[error("error while trying to register the server function: {0}")]
    Registration(String),
    /// Occurs on the client if there is a network error while trying to run function on server.
    #[error("error reaching server to call server function: {0}")]
    Request(String),
    /// Occurs when there is an error while actually running the function on the server.
    #[error("error running server function: {0}")]
    ServerError(String),
    /// Occurs on the client if there is an error deserializing the server's response.
    #[error("error deserializing server function results {0}")]
    Deserialization(String),
    /// Occurs on the client if there is an error serializing the server function arguments.
    #[error("error serializing server function results {0}")]
    Serialization(String),
    /// Occurs on the server if there is an error deserializing one of the arguments that's been sent.
    #[error("error deserializing server function arguments {0}")]
    Args(String),
    /// Occurs on the server if there's a missing argument.
    #[error("missing argument {0}")]
    MissingArg(String),
}

/// Executes the HTTP call to call a server function from the client, given its URL and argument type.
#[cfg(not(feature = "ssr"))]
pub async fn call_server_fn<T>(url: &str, args: impl ServerFn) -> Result<T, ServerFnError>
where
    T: Serializable + Sized,
{
    let args_form_data = serde_urlencoded::to_string(&args)
        .map_err(|e| ServerFnError::Serialization(e.to_string()))?;

    let resp = gloo_net::http::Request::post(url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Accept", "application/json")
        .body(args_form_data)
        .send()
        .await
        .map_err(|e| ServerFnError::Request(e.to_string()))?;

    // check for error status
    let status = resp.status();
    if (500..=599).contains(&status) {
        return Err(ServerFnError::ServerError(resp.status_text()));
    }

    let text = resp
        .text()
        .await
        .map_err(|e| ServerFnError::Deserialization(e.to_string()))?;

    T::from_json(&text).map_err(|e| ServerFnError::Deserialization(e.to_string()))
}

/// An action synchronizes an imperative `async` call to the synchronous reactive system.
///
/// If you’re trying to load data by running an `async` function reactively, you probably
/// want to use a [Resource](leptos_reactive::Resource) instead. If you’re trying to occasionally
/// run an `async` function in response to something like a user clicking a button, you're in the right place.
///
/// ```rust
/// # use leptos_reactive::run_scope;
/// # use leptos_server::create_action;
/// # run_scope(|cx| {
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
/// let input = save_data.input();
/// // the most recent returned result
/// let result_of_call = save_data.value();
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
/// # use leptos_reactive::run_scope;
/// # use leptos_server::create_action;
/// # run_scope(|cx| {
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
    input: RwSignal<Option<I>>,
    value: RwSignal<Option<O>>,
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
    /// Calls the server function a reference to the input type as its argument.
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

    /// The argument that was dispatched to the `async` function,
    /// only while we are waiting for it to resolve.
    pub fn input(&self) -> ReadSignal<Option<I>> {
        self.input.read_only()
    }

    /// The argument that was dispatched to the `async` function.
    ///
    /// You probably don't need to call this unless you are implementing a form
    /// or some other kind of wrapper for an action and need to set the input
    /// based on its internal logic.
    pub fn set_input(&self, value: I) {
        self.input.set(Some(value));
    }

    /// The most recent return value of the `async` function.
    pub fn value(&self) -> ReadSignal<Option<O>> {
        self.value.read_only()
    }

    /// Sets the most recent return value of the `async` function.
    ///
    /// You probably don't need to call this unless you are implementing a form
    /// or some other kind of wrapper for an action and need to set the value
    /// based on its internal logic.
    pub fn set_value(&self, value: O) {
        self.value.set(Some(value));
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
/// # use leptos_reactive::run_scope;
/// # use leptos_server::create_action;
/// # run_scope(|cx| {
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
/// let input = save_data.input();
/// // the most recent returned result
/// let result_of_call = save_data.value();
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
/// # use leptos_reactive::run_scope;
/// # use leptos_server::create_action;
/// # run_scope(|cx| {
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
/// # use leptos_reactive::run_scope;
/// # use leptos_server::{create_server_action, ServerFnError, ServerFn};
/// # use leptos_macro::server;
///
/// #[server(MyServerFn)]
/// async fn my_server_fn() -> Result<(), ServerFnError> {
///   todo!()
/// }
///
/// # run_scope(|cx| {
/// let my_server_action = create_server_action::<MyServerFn>(cx);
/// # });
/// ```
pub fn create_server_action<S>(cx: Scope) -> Action<S, Result<S::Output, ServerFnError>>
where
    S: Clone + ServerFn,
{
    #[cfg(feature = "ssr")]
    let c = |args: &S| S::call_fn(args.clone());
    #[cfg(not(feature = "ssr"))]
    let c = |args: &S| S::call_fn_client(args.clone());
    create_action(cx, c).using_server_fn::<S>()
}
