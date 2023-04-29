#![deny(missing_docs)]
#![forbid(unsafe_code)]

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
//! The [`#[server]`](https://docs.rs/leptos/latest/leptos/attr.server.html) macro allows you to annotate a function to
//! indicate that it should only run on the server (i.e., when you have an `ssr` feature in your
//! crate that is enabled).
//!
//! **Important**: All server functions must be registered by calling [ServerFn::register]
//! somewhere within your `main` function.
//!
//! ```rust,ignore
//! # use leptos::*;
//! #[server(ReadFromDB)]
//! async fn read_posts(cx: Scope, how_many: usize, query: String) -> Result<Vec<Posts>, ServerFnError> {
//!   // do some server-only work here to access the database
//!   let posts = ...;
//!   Ok(posts)
//! }
//!
//! // call the function
//! # run_scope(create_runtime(), |cx| {
//! spawn_local(async {
//!   let posts = read_posts(3, "my search".to_string()).await;
//!   log::debug!("posts = {posts:#?}");
//! })
//! # });
//!
//! // make sure you've registered it somewhere in main
//! fn main() {
//!   _ = ReadFromDB::register();
//! }
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
//! - **Return types must be [Serializable](leptos_reactive::Serializable).**
//!   This should be fairly obvious: we have to serialize arguments to send them to the server, and we
//!   need to deserialize the result to return it to the client.
//! - **Arguments must be implement [serde::Serialize].** They are serialized as an `application/x-www-form-urlencoded`
//!   form data using [`serde_html_form`](https://docs.rs/serde_html_form/latest/serde_html_form/) or as `application/cbor`
//!   using [`cbor`](https://docs.rs/cbor/latest/cbor/). **Note**: You should explicitly include `serde` with the
//!   `derive` feature enabled in your `Cargo.toml`. You can do this by running `cargo add serde --features=derive`.
//! - **The [Scope](leptos_reactive::Scope) comes from the server.** Optionally, the first argument of a server function
//!   can be a Leptos [Scope](leptos_reactive::Scope). This scope can be used to inject dependencies like the HTTP request
//!   or response or other server-only dependencies, but it does *not* have access to reactive state that exists in the client.

use leptos_reactive::*;
pub use server_fn::{Encoding, Payload, ServerFnError};

mod action;
mod multi_action;
pub use action::*;
pub use multi_action::*;
extern crate tracing;

#[cfg(any(feature = "ssr", doc))]
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

#[cfg(any(feature = "ssr", doc))]
type ServerFnTraitObj = server_fn::ServerFnTraitObj<Scope>;

#[allow(unused)]
type ServerFunction = server_fn::ServerFunction<Scope>;

#[cfg(any(feature = "ssr", doc))]
lazy_static::lazy_static! {
    static ref REGISTERED_SERVER_FUNCTIONS: Arc<RwLock<HashMap<&'static str, ServerFunction>>> = Default::default();
}

#[cfg(any(feature = "ssr", doc))]
/// The registry of all Leptos server functions.
pub struct LeptosServerFnRegistry;

#[cfg(any(feature = "ssr", doc))]
impl server_fn::ServerFunctionRegistry<Scope> for LeptosServerFnRegistry {
    type Error = ServerRegistrationFnError;

    fn register(
        url: &'static str,
        trait_obj: Arc<ServerFnTraitObj>,
        encoding: Encoding,
    ) -> Result<(), Self::Error> {
        // store it in the hashmap
        let mut func_write = REGISTERED_SERVER_FUNCTIONS
            .write()
            .map_err(|e| ServerRegistrationFnError::Poisoned(e.to_string()))?;
        let prev = func_write.insert(
            url,
            ServerFunction {
                trait_obj,
                encoding,
            },
        );

        // if there was already a server function with this key,
        // return Err
        match prev {
            Some(_) => {
                Err(ServerRegistrationFnError::AlreadyRegistered(format!(
                    "There was already a server function registered at {:?}. \
                     This can happen if you use the same server function name \
                     in two different modules
                on `stable` or in `release` mode.",
                    url
                )))
            }
            None => Ok(()),
        }
    }

    /// Returns the server function registered at the given URL, or `None` if no function is registered at that URL.
    fn get(url: &str) -> Option<ServerFunction> {
        REGISTERED_SERVER_FUNCTIONS
            .read()
            .ok()
            .and_then(|fns| fns.get(url).cloned())
    }

    /// Returns the server function trait obj registered at the given URL, or `None` if no function is registered at that URL.
    fn get_trait_obj(url: &str) -> Option<Arc<ServerFnTraitObj>> {
        REGISTERED_SERVER_FUNCTIONS
            .read()
            .ok()
            .and_then(|fns| fns.get(url).map(|sf| sf.trait_obj.clone()))
    }
    /// Return the
    fn get_encoding(url: &str) -> Option<Encoding> {
        REGISTERED_SERVER_FUNCTIONS
            .read()
            .ok()
            .and_then(|fns| fns.get(url).map(|sf| sf.encoding.clone()))
    }

    /// Returns a list of all registered server functions.
    fn paths_registered() -> Vec<&'static str> {
        REGISTERED_SERVER_FUNCTIONS
            .read()
            .ok()
            .map(|fns| fns.keys().cloned().collect())
            .unwrap_or_default()
    }
}

#[cfg(any(feature = "ssr", doc))]
/// Errors that can occur when registering a server function.
#[derive(
    thiserror::Error, Debug, Clone, serde::Serialize, serde::Deserialize,
)]
pub enum ServerRegistrationFnError {
    /// The server function is already registered.
    #[error("The server function {0} is already registered")]
    AlreadyRegistered(String),
    /// The server function registry is poisoned.
    #[error("The server function registry is poisoned: {0}")]
    Poisoned(String),
}

/// Get a ServerFunction struct containing info about the server fn
#[cfg(any(feature = "ssr", doc))]
pub fn server_fn_by_path(path: &str) -> Option<ServerFunction> {
    server_fn::server_fn_by_path::<Scope, LeptosServerFnRegistry>(path)
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
///     if let Some(server_fn) = server_fn_by_path(path.as_str()) {
///         let query = req.query_string().as_bytes();
///         let data = match &server_fn.encoding {
///             Encoding::Url | Encoding::Cbor => &body,
///             Encoding::GetJSON | Encoding::GetCBOR => query,
///         };
///         match (server_fn.trait_obj)(data).await {
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
pub fn server_fn_trait_obj_by_path(
    path: &str,
) -> Option<Arc<ServerFnTraitObj>> {
    server_fn::server_fn_trait_obj_by_path::<Scope, LeptosServerFnRegistry>(
        path,
    )
}

/// Get the Encoding of a server fn if one is registered at that path. Otherwise, return None
#[cfg(any(feature = "ssr", doc))]
pub fn server_fn_encoding_by_path(path: &str) -> Option<Encoding> {
    server_fn::server_fn_encoding_by_path::<Scope, LeptosServerFnRegistry>(path)
}

/// Returns the set of currently-registered server function paths, for debugging purposes.
#[cfg(any(feature = "ssr", doc))]
pub fn server_fns_by_path() -> Vec<&'static str> {
    server_fn::server_fns_by_path::<Scope, LeptosServerFnRegistry>()
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
pub trait ServerFn: server_fn::ServerFn<Scope> {
    /// Registers the server function, allowing the server to query it by URL.
    #[cfg(any(feature = "ssr", doc))]
    fn register() -> Result<(), ServerFnError> {
        Self::register_in::<LeptosServerFnRegistry>()
    }
}

impl<T> ServerFn for T where T: server_fn::ServerFn<Scope> {}
