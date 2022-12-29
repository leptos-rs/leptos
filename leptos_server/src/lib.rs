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
//! The [`#[server]` macro](leptos::leptos_macro::server) allows you to annotate a function to
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
//!   log::debug!("posts = {posts{:#?}");
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
//!   form data using [`serde_urlencoded`](https://docs.rs/serde_urlencoded/latest/serde_urlencoded/) or as `application/cbor`
//!   using [`cbor`](https://docs.rs/cbor/latest/cbor/).
//! - **The [Scope](leptos_reactive::Scope) comes from the server.** Optionally, the first argument of a server function
//!   can be a Leptos [Scope](leptos_reactive::Scope). This scope can be used to inject dependencies like the HTTP request
//!   or response or other server-only dependencies, but it does *not* have access to reactive state that exists in the client.

pub use form_urlencoded;
use leptos_reactive::*;

use proc_macro2::{Literal, TokenStream};
use quote::TokenStreamExt;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{future::Future, pin::Pin, str::FromStr};
use syn::{
    parse::{Parse, ParseStream},
    parse_quote,
};
use thiserror::Error;

mod action;
mod multi_action;
pub use action::*;
pub use multi_action::*;

#[cfg(any(feature = "ssr", doc))]
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

#[cfg(any(feature = "ssr", doc))]
type ServerFnTraitObj = dyn Fn(Scope, &[u8]) -> Pin<Box<dyn Future<Output = Result<Payload, ServerFnError>>>>
    + Send
    + Sync;

#[cfg(any(feature = "ssr", doc))]
lazy_static::lazy_static! {
    static ref REGISTERED_SERVER_FUNCTIONS: Arc<RwLock<HashMap<&'static str, Arc<ServerFnTraitObj>>>> = Default::default();
}

/// A dual type to hold the possible Response datatypes
#[derive(Debug)]
pub enum Payload {
    ///Encodes Data using CBOR
    Binary(Vec<u8>),
    ///Encodes data in the URL
    Url(String),
    ///Encodes Data using Json
    Json(String),
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

/// Holds the current options for encoding types.
/// More could be added, but they need to be serde
#[derive(Debug, PartialEq)]
pub enum Encoding {
    /// A Binary Encoding Scheme Called Cbor
    Cbor,
    /// The Default URL-encoded encoding method
    Url,
}

impl FromStr for Encoding {
    type Err = ();

    fn from_str(input: &str) -> Result<Encoding, Self::Err> {
        match input {
            "URL" => Ok(Encoding::Url),
            "Cbor" => Ok(Encoding::Cbor),
            _ => Err(()),
        }
    }
}

impl quote::ToTokens for Encoding {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let option: syn::Ident = match *self {
            Encoding::Cbor => parse_quote!(Cbor),
            Encoding::Url => parse_quote!(Url),
        };
        let expansion: syn::Ident = syn::parse_quote! {
          Encoding::#option
        };
        tokens.append(expansion);
    }
}

impl Parse for Encoding {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let variant_name: String = input.parse::<Literal>()?.to_string();

        // Need doubled quotes because variant_name doubles it
        match variant_name.as_ref() {
            "\"Url\"" => Ok(Self::Url),
            "\"Cbor\"" => Ok(Self::Cbor),
            _ => panic!("Encoding Not Found"),
        }
    }
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
    type Output: Serialize;

    /// URL prefix that should be prepended by the client to the generated URL.
    fn prefix() -> &'static str;

    /// The path at which the server function can be reached on the server.
    fn url() -> &'static str;

    /// The path at which the server function can be reached on the server.
    fn encoding() -> Encoding;

    /// Runs the function on the server.
    #[cfg(any(feature = "ssr", doc))]
    fn call_fn(
        self,
        cx: Scope,
    ) -> Pin<Box<dyn Future<Output = Result<Self::Output, ServerFnError>>>>;

    /// Runs the function on the client by sending an HTTP request to the server.
    #[cfg(any(not(feature = "ssr"), doc))]
    fn call_fn_client(
        self,
        cx: Scope,
    ) -> Pin<Box<dyn Future<Output = Result<Self::Output, ServerFnError>>>>;

    /// Registers the server function, allowing the server to query it by URL.
    #[cfg(any(feature = "ssr", doc))]
    fn register() -> Result<(), ServerFnError> {
        // create the handler for this server function
        // takes a String -> returns its async value

        let run_server_fn = Arc::new(|cx: Scope, data: &[u8]| {
            // decode the args
            let value = match Self::encoding() {
                Encoding::Url => serde_urlencoded::from_bytes(data)
                    .map_err(|e| ServerFnError::Deserialization(e.to_string())),
                Encoding::Cbor => ciborium::de::from_reader(data)
                    .map_err(|e| ServerFnError::Deserialization(e.to_string())),
            };
            Box::pin(async move {
                let value: Self = match value {
                    Ok(v) => v,
                    Err(e) => return Err(e),
                };

                // call the function
                let result = match value.call_fn(cx).await {
                    Ok(r) => r,
                    Err(e) => return Err(e),
                };

                // serialize the output
                let result = match Self::encoding() {
                    Encoding::Url => match serde_json::to_string(&result)
                        .map_err(|e| ServerFnError::Serialization(e.to_string()))
                    {
                        Ok(r) => Payload::Url(r),
                        Err(e) => return Err(e),
                    },
                    Encoding::Cbor => {
                        let mut buffer: Vec<u8> = Vec::new();
                        match ciborium::ser::into_writer(&result, &mut buffer)
                            .map_err(|e| ServerFnError::Serialization(e.to_string()))
                        {
                            Ok(_) => Payload::Binary(buffer),
                            Err(e) => return Err(e),
                        }
                    }
                };

                Ok(result)
            }) as Pin<Box<dyn Future<Output = Result<Payload, ServerFnError>>>>
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
pub async fn call_server_fn<T>(
    url: &str,
    args: impl ServerFn,
    enc: Encoding,
) -> Result<T, ServerFnError>
where
    T: serde::Serialize + serde::de::DeserializeOwned + Sized,
{
    use ciborium::ser::into_writer;
    use leptos_dom::js_sys::Uint8Array;
    use serde_json::Deserializer as JSONDeserializer;

    #[derive(Debug)]
    enum Payload {
        Binary(Vec<u8>),
        Url(String),
    }
    let args_encoded = match &enc {
        Encoding::Url => Payload::Url(
            serde_urlencoded::to_string(&args)
                .map_err(|e| ServerFnError::Serialization(e.to_string()))?,
        ),
        Encoding::Cbor => {
            let mut buffer: Vec<u8> = Vec::new();
            into_writer(&args, &mut buffer)
                .map_err(|e| ServerFnError::Serialization(e.to_string()))?;
            Payload::Binary(buffer)
        }
    };

    let content_type_header = match &enc {
        Encoding::Url => "application/x-www-form-urlencoded",
        Encoding::Cbor => "application/cbor",
    };

    let accept_header = match &enc {
        Encoding::Url => "application/x-www-form-urlencoded",
        Encoding::Cbor => "application/cbor",
    };

    let resp = match args_encoded {
        Payload::Binary(b) => {
            let slice_ref: &[u8] = &b;
            let js_array = Uint8Array::from(slice_ref).buffer();
            gloo_net::http::Request::post(url)
                .header("Content-Type", content_type_header)
                .header("Accept", accept_header)
                .body(js_array)
                .send()
                .await
                .map_err(|e| ServerFnError::Request(e.to_string()))?
        }
        Payload::Url(s) => gloo_net::http::Request::post(url)
            .header("Content-Type", content_type_header)
            .header("Accept", accept_header)
            .body(s)
            .send()
            .await
            .map_err(|e| ServerFnError::Request(e.to_string()))?,
    };

    // check for error status
    let status = resp.status();
    if (500..=599).contains(&status) {
        return Err(ServerFnError::ServerError(resp.status_text()));
    }

    if enc == Encoding::Cbor {
        let binary = resp
            .binary()
            .await
            .map_err(|e| ServerFnError::Deserialization(e.to_string()))?;

        ciborium::de::from_reader(binary.as_slice())
            .map_err(|e| ServerFnError::Deserialization(e.to_string()))
    } else {
        let text = resp
            .text()
            .await
            .map_err(|e| ServerFnError::Deserialization(e.to_string()))?;

        let mut deserializer = JSONDeserializer::from_str(&text);
        T::deserialize(&mut deserializer).map_err(|e| ServerFnError::Deserialization(e.to_string()))
    }
}
