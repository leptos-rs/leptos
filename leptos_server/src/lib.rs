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
//!   form data using [`serde_qs`](https://docs.rs/serde_qs/latest/serde_qs/) or as `application/cbor`
//!   using [`cbor`](https://docs.rs/cbor/latest/cbor/). **Note**: You should explicitly include `serde` with the
//!   `derive` feature enabled in your `Cargo.toml`. You can do this by running `cargo add serde --features=derive`.
//! - **The [Scope](leptos_reactive::Scope) comes from the server.** Optionally, the first argument of a server function
//!   can be a Leptos [Scope](leptos_reactive::Scope). This scope can be used to inject dependencies like the HTTP request
//!   or response or other server-only dependencies, but it does *not* have access to reactive state that exists in the client.
//!
//! ## Server Function Encodings
//!
//! By default, the server function call is a `POST` request that serializes the arguments as URL-encoded form data in the body
//! of the request. But there are a few other methods supported. Optionally, we can provide another argument to the `#[server]`
//! macro to specify an alternate encoding:
//!
//! ```rust,ignore
//! #[server(AddTodo, "/api", "Url")]
//! #[server(AddTodo, "/api", "GetJson")]
//! #[server(AddTodo, "/api", "Cbor")]
//! #[server(AddTodo, "/api", "GetCbor")]
//! ```
//!
//! The four options use different combinations of HTTP verbs and encoding methods:
//!
//! | Name              | Method | Request     | Response |
//! | ----------------- | ------ | ----------- | -------- |
//! | **Url** (default) | POST   | URL encoded | JSON     |
//! | **GetJson**       | GET    | URL encoded | JSON     |
//! | **Cbor**          | POST   | CBOR        | CBOR     |
//! | **GetCbor**       | GET    | URL encoded | CBOR     |
//!
//! In other words, you have two choices:
//!
//! - `GET` or `POST`? This has implications for things like browser or CDN caching; while `POST` requests should not be cached,
//! `GET` requests can be.
//! - Plain text (arguments sent with URL/form encoding, results sent as JSON) or a binary format (CBOR, encoded as a base64
//! string)?
//!
//! ## Why not `PUT` or `DELETE`? Why URL/form encoding, and not JSON?**
//!
//! These are reasonable questions. Much of the web is built on REST API patterns that encourage the use of semantic HTTP
//! methods like `DELETE` to delete an item from a database, and many devs are accustomed to sending data to APIs in the
//! JSON format.
//!
//! The reason we use `POST` or `GET` with URL-encoded data by default is the `<form>` support. For better or for worse,
//! HTML forms don’t support `PUT` or `DELETE`, and they don’t support sending JSON. This means that if you use anything
//! but a `GET` or `POST` request with URL-encoded data, it can only work once WASM has loaded.
//!
//! The CBOR encoding is suported for historical reasons; an earlier version of server functions used a URL encoding that
//! didn’t support nested objects like structs or vectors as server function arguments, which CBOR did. But note that the
//! CBOR forms encounter the same issue as `PUT`, `DELETE`, or JSON: they do not degrade gracefully if the WASM version of
//! your app is not available.

use leptos_reactive::*;
pub use server_fn::{
    error::ServerFnErrorErr, Encoding, Payload, ServerFnError,
};

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
/// A concrete type for a server function.
#[derive(Clone)]
pub struct ServerFnTraitObj(pub server_fn::ServerFnTraitObj<Scope>);

#[cfg(any(feature = "ssr", doc))]
impl std::ops::Deref for ServerFnTraitObj {
    type Target = server_fn::ServerFnTraitObj<Scope>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(any(feature = "ssr", doc))]
impl std::ops::DerefMut for ServerFnTraitObj {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[cfg(any(feature = "ssr", doc))]
impl ServerFnTraitObj {
    /// Create a new `ServerFnTraitObj` from a `server_fn::ServerFnTraitObj`.
    pub const fn from_generic_server_fn(
        server_fn: server_fn::ServerFnTraitObj<Scope>,
    ) -> Self {
        Self(server_fn)
    }
}

#[cfg(feature = "ssr")]
inventory::collect!(ServerFnTraitObj);

#[allow(unused)]
type ServerFunction = server_fn::ServerFnTraitObj<Scope>;

#[cfg(any(feature = "ssr", doc))]
lazy_static::lazy_static! {
    static ref REGISTERED_SERVER_FUNCTIONS: Arc<RwLock<HashMap<&'static str, ServerFnTraitObj>>> = {
        let mut map = HashMap::new();
        for server_fn in inventory::iter::<ServerFnTraitObj> {
            map.insert(server_fn.0.url(), server_fn.clone());
        }
        Arc::new(RwLock::new(map))
    };
}

#[cfg(any(feature = "ssr", doc))]
/// The registry of all Leptos server functions.
pub struct LeptosServerFnRegistry;

#[cfg(any(feature = "ssr", doc))]
impl server_fn::ServerFunctionRegistry<Scope> for LeptosServerFnRegistry {
    type Error = ServerRegistrationFnError;

    fn register(
        _url: &'static str,
        _server_function: server_fn::SerializedFnTraitObj<Scope>,
        _encoding: Encoding,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Server functions are automatically registered on most platforms, (including Linux, macOS,
    /// iOS, FreeBSD, Android, and Windows). If you are on another platform, like a WASM server runtime,
    /// you should register server functions by calling this `T::register_explicit()`.
    fn register_explicit(
        prefix: &'static str,
        url: &'static str,
        server_function: server_fn::SerializedFnTraitObj<Scope>,
        encoding: Encoding,
    ) -> Result<(), Self::Error> {
        // store it in the hashmap
        let mut func_write = REGISTERED_SERVER_FUNCTIONS
            .write()
            .map_err(|e| ServerRegistrationFnError::Poisoned(e.to_string()))?;
        let prev = func_write.insert(
            url,
            ServerFnTraitObj(server_fn::ServerFnTraitObj::new(
                prefix,
                url,
                encoding,
                server_function,
            )),
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
    fn get(url: &str) -> Option<server_fn::ServerFnTraitObj<Scope>> {
        REGISTERED_SERVER_FUNCTIONS
            .read()
            .ok()
            .and_then(|fns| fns.get(url).map(|sf| sf.0.clone()))
    }

    /// Returns the server function trait obj registered at the given URL, or `None` if no function is registered at that URL.
    fn get_trait_obj(url: &str) -> Option<server_fn::ServerFnTraitObj<Scope>> {
        REGISTERED_SERVER_FUNCTIONS
            .read()
            .ok()
            .and_then(|fns| fns.get(url).map(|sf| sf.0.clone()))
    }
    /// Return the
    fn get_encoding(url: &str) -> Option<Encoding> {
        REGISTERED_SERVER_FUNCTIONS
            .read()
            .ok()
            .and_then(|fns| fns.get(url).map(|sf| sf.encoding()))
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
pub fn server_fn_by_path(path: &str) -> Option<ServerFnTraitObj> {
    REGISTERED_SERVER_FUNCTIONS
        .read()
        .expect("Server function registry is poisoned")
        .get(path)
        .cloned()
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
pub fn server_fn_trait_obj_by_path(path: &str) -> Option<ServerFnTraitObj> {
    server_fn::server_fn_trait_obj_by_path::<Scope, LeptosServerFnRegistry>(
        path,
    )
    .map(ServerFnTraitObj::from_generic_server_fn)
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
    #[deprecated = "Explicit server function registration is no longer \
                    required on most platforms (including Linux, macOS, iOS, \
                    FreeBSD, Android, and Windows). If you are on another \
                    platform and need to explicitly register server functions, \
                    call ServerFn::register_explicit() instead."]
    fn register() -> Result<(), ServerFnError> {
        Ok(())
    }

    #[cfg(any(feature = "ssr", doc))]
    /// Explicitly registers the server function on platforms that require it,
    /// allowing the server to query it by URL.
    ///
    /// Explicit server function registration is no longer required on most platforms
    /// (including Linux, macOS, iOS, FreeBSD, Android, and Windows)
    fn register_explicit() -> Result<(), ServerFnError> {
        Self::register_in_explicit::<LeptosServerFnRegistry>()
    }
}

impl<T> ServerFn for T where T: server_fn::ServerFn<Scope> {}
