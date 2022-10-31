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
//!
//! ### `create_action`
//!
//! The easiest way to call server functions from the client is the `create_action` primitive.
//! This returns an [Action](crate::Action), with a [dispatch](crate::Action::dispatch) method
//! that can run any `async` function, including one that contains one or more calls to server functions.
//!
//! Dispatching an action increments its [version](crate::Action::version) field, which is a
//! signal. This is very useful, as it can be used to invalidate a [Resource](leptos_reactive::Resource)
//! that reads from the same data.

pub use async_trait::async_trait;
pub use form_urlencoded;
use leptos_reactive::*;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{future::Future, pin::Pin, rc::Rc};
use thiserror::Error;

#[cfg(feature = "ssr")]
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

#[cfg(feature = "ssr")]
type ServerFnTraitObj =
    dyn Fn(&[u8]) -> Pin<Box<dyn Future<Output = Result<String, ServerFnError>>>> + Send + Sync;

#[cfg(feature = "ssr")]
lazy_static::lazy_static! {
    pub static ref REGISTERED_SERVER_FUNCTIONS: Arc<RwLock<HashMap<&'static str, Arc<ServerFnTraitObj>>>> = Default::default();
}

#[cfg(feature = "ssr")]
pub fn server_fn_by_path(path: &str) -> Option<Arc<ServerFnTraitObj>> {
    REGISTERED_SERVER_FUNCTIONS
        .read()
        .ok()
        .and_then(|fns| fns.get(path).cloned())
}

#[async_trait]
pub trait ServerFn
where
    Self: Sized + 'static,
{
    type Output: Serializable;

    fn url() -> &'static str;

    fn as_form_data(&self) -> Vec<(&'static str, String)>;

    fn from_form_data(data: &[u8]) -> Result<Self, ServerFnError>;

    #[cfg(feature = "ssr")]
    async fn call_fn(self) -> Result<Self::Output, ServerFnError>;

    #[cfg(feature = "ssr")]
    fn register() -> Result<(), ServerFnError> {
        // create the handler for this server function
        // takes a String -> returns its async value
        let run_server_fn = Arc::new(|data: &[u8]| {
            // decode the args
            let value = Self::from_form_data(data);
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

#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum ServerFnError {
    #[error("error while trying to register the server function: {0}")]
    Registration(String),
    #[error("error reaching server to call server function: {0}")]
    Request(String),
    #[error("error running server function: {0}")]
    ServerError(String),
    #[error("error deserializing server function results {0}")]
    Deserialization(String),
    #[error("error serializing server function results {0}")]
    Serialization(String),
    #[error("error deserializing server function arguments {0}")]
    Args(String),
    #[error("missing argument {0}")]
    MissingArg(String),
}

#[cfg(any(feature = "csr", feature = "hydrate"))]
pub async fn call_server_fn<T>(url: &str, args: impl ServerFn) -> Result<T, ServerFnError>
where
    T: Serializable + Sized,
{
    use leptos_dom::*;

    let args_form_data = web_sys::FormData::new().expect_throw("could not create FormData");
    for (field_name, value) in args.as_form_data().into_iter() {
        args_form_data
            .append_with_str(field_name, &value)
            .expect_throw("could not append form field");
    }
    let args_form_data = web_sys::UrlSearchParams::new_with_str_sequence_sequence(&args_form_data)
        .expect_throw("could not URL encode FormData");
    let args_form_data = args_form_data.to_string().as_string().unwrap_or_default();

    let resp = gloo_net::http::Request::post(url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Accept", "application/json")
        .body(args_form_data.to_string())
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

#[derive(Clone)]
pub struct Action<I, O>
where
    I: 'static,
    O: 'static,
{
    pub version: RwSignal<usize>,
    input: RwSignal<Option<I>>,
    value: RwSignal<Option<O>>,
    pending: RwSignal<bool>,
    url: Option<&'static str>,
    #[allow(clippy::complexity)]
    action_fn: Rc<dyn Fn(&I) -> Pin<Box<dyn Future<Output = O>>>>,
}

impl<I, O> Action<I, O>
where
    I: 'static,
    O: 'static,
{
    pub fn using_server_fn<T: ServerFn>(mut self) -> Self {
        self.url = Some(T::url());
        self
    }

    pub fn pending(&self) -> impl Fn() -> bool {
        let value = self.value;
        move || value.with(|val| val.is_some())
    }

    pub fn input(&self) -> ReadSignal<Option<I>> {
        self.input.read_only()
    }

    pub fn value(&self) -> ReadSignal<Option<O>> {
        self.value.read_only()
    }

    pub fn url(&self) -> Option<&str> {
        self.url
    }

    pub fn dispatch(&self, input: I) {
        let fut = (self.action_fn)(&input);
        self.input.set(Some(input));
        let version = self.version;
        let pending = self.pending;
        let value = self.value;
        pending.set(true);
        spawn_local(async move {
            let new_value = fut.await;
            value.set(Some(new_value));
            pending.set(false);
            version.update(|n| *n += 1);
        })
    }
}

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
