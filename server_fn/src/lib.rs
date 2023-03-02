#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! # Server Functions
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
//! The [`#[server]`](https://docs.rs/server_fn/latest/server_fn/attr.server.html) macro allows you to annotate a function to
//! indicate that it should only run on the server (i.e., when you have an `ssr` feature in your
//! crate that is enabled).
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
//! # #[tokio::main]
//! # async fn main() {
//! async {
//!   let posts = read_posts(3, "my search".to_string()).await;
//!   log::debug!("posts = {posts{:#?}");
//! }
//! # }
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
//! - **Return types must implement [Serialize](serde::Serialize).**
//!   This should be fairly obvious: we have to serialize arguments to send them to the server, and we
//!   need to deserialize the result to return it to the client.
//! - **Arguments must be implement [serde::Serialize].** They are serialized as an `application/x-www-form-urlencoded`
//!   form data using [`serde_urlencoded`](https://docs.rs/serde_urlencoded/latest/serde_urlencoded/) or as `application/cbor`
//!   using [`cbor`](https://docs.rs/cbor/latest/cbor/).

// used by the macro
#[doc(hidden)]
pub use const_format;
#[cfg(any(feature = "ssr", doc))]
// used by the macro
#[doc(hidden)]
pub use inventory;
#[cfg(any(feature = "ssr", doc))]
use proc_macro2::TokenStream;
#[cfg(any(feature = "ssr", doc))]
use quote::TokenStreamExt;
// used by the macro
#[doc(hidden)]
pub use serde;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{future::Future, pin::Pin, str::FromStr};
use thiserror::Error;
// used by the macro
#[doc(hidden)]
pub use xxhash_rust;

/// The default wrapper for the server function that accepts no context from the server
pub mod default;

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

/// Holds the current options for encoding types.
/// More could be added, but they need to be serde
#[derive(Clone, Copy, Debug, PartialEq)]
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

#[cfg(any(feature = "ssr", doc))]
impl quote::ToTokens for Encoding {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let option: syn::Ident = match *self {
            Encoding::Cbor => syn::parse_quote!(Cbor),
            Encoding::Url => syn::parse_quote!(Url),
        };
        let expansion: syn::Ident = syn::parse_quote! {
            Encoding::#option
        };
        tokens.append(expansion);
    }
}

/// Defines a "server function." A server function can be called from the server or the client,
/// but the body of its code will only be run on the server, i.e., if a crate feature `ssr` (server-side-rendering) is enabled.
///
/// Server functions are created using the `server` macro.
///
/// The set of server functions can be queried on the server for routing purposes by calling [server_fn_by_path].
///
/// Technically, the trait is implemented on a type that describes the server function's arguments.
pub trait ServerFn<T: 'static>
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
        cx: T,
    ) -> Pin<Box<dyn Future<Output = Result<Self::Output, ServerFnError>>>>;

    /// Runs the function on the client by sending an HTTP request to the server.
    #[cfg(any(not(feature = "ssr"), doc))]
    fn call_fn_client(
        self,
        cx: T,
    ) -> Pin<Box<dyn Future<Output = Result<Self::Output, ServerFnError>>>>;

    /// Returns a trait object that can be used to call the server function.
    #[cfg(any(feature = "ssr", doc))]
    fn call_from_bytes(
        cx: T,
        bytes: &[u8],
    ) -> Pin<Box<dyn Future<Output = Result<Payload, ServerFnError>>>> {
        let value = Self::encoding().deserialize_from_bytes(bytes);
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
                        .map_err(|e| {
                            ServerFnError::Serialization(e.to_string())
                        }) {
                        Ok(_) => Payload::Binary(buffer),
                        Err(e) => return Err(e),
                    }
                }
            };

            Ok(result)
        })
            as Pin<Box<dyn Future<Output = Result<Payload, ServerFnError>>>>
    }
}

/// Type for errors that can occur when using server functions.
#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum ServerFnError {
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
pub async fn call_server_fn<T, C: 'static>(
    url: &str,
    args: impl ServerFn<C>,
    enc: Encoding,
) -> Result<T, ServerFnError>
where
    T: serde::Serialize + serde::de::DeserializeOwned + Sized,
{
    use ciborium::ser::into_writer;
    use js_sys::Uint8Array;
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
        T::deserialize(&mut deserializer)
            .map_err(|e| ServerFnError::Deserialization(e.to_string()))
    }
}

#[cfg(any(feature = "ssr", doc))]
/// A server function that can be called from the client.
pub type SerializedFnTraitObj<T> =
    fn(
        T,
        &[u8],
    ) -> Pin<Box<dyn Future<Output = Result<Payload, ServerFnError>>>>;

#[cfg(any(feature = "ssr", doc))]
/// A concrete type for a registered server function
pub struct ServerFnTraitObj<T> {
    prefix: &'static str,
    url: &'static str,
    encoding: Encoding,
    run: SerializedFnTraitObj<T>,
}

#[cfg(any(feature = "ssr", doc))]
impl<T> ServerFnTraitObj<T> {
    /// Creates a new server function with the given prefix, URL, encoding, and function.
    pub const fn new(
        prefix: &'static str,
        url: &'static str,
        encoding: Encoding,
        run: SerializedFnTraitObj<T>,
    ) -> Self {
        Self {
            prefix,
            url,
            encoding,
            run,
        }
    }

    /// Runs the server function with the given server agruments and serialized buffer from the client.
    pub fn call(
        &self,
        args: T,
        buffer: &[u8],
    ) -> Pin<Box<dyn Future<Output = Result<Payload, ServerFnError>>>> {
        (self.run)(args, buffer)
    }

    /// Returns the prefix of the server function.
    pub fn prefix(&self) -> &str {
        self.prefix
    }

    /// Returns the URL of the server function.
    pub fn url(&self) -> &str {
        self.url
    }

    /// Returns the encoding of the server function.
    pub fn encoding(&self) -> Encoding {
        self.encoding
    }
}

impl Encoding {
    /// Deserializes the given bytes into the given type.
    pub fn deserialize_from_bytes<T: serde::de::DeserializeOwned>(
        &self,
        data: &[u8],
    ) -> Result<T, ServerFnError> {
        match self {
            Self::Url => serde_urlencoded::from_bytes(data)
                .map_err(|e| ServerFnError::Deserialization(e.to_string())),
            Self::Cbor => ciborium::de::from_reader(data)
                .map_err(|e| ServerFnError::Deserialization(e.to_string())),
        }
    }

    /// Serializes the given value into bytes.
    pub fn serialize_into_bytes<T: serde::Serialize>(
        &self,
        value: &T,
    ) -> Result<Payload, ServerFnError> {
        Ok(match self {
            Self::Url => match serde_json::to_string(&value)
                .map_err(|e| ServerFnError::Serialization(e.to_string()))
            {
                Ok(r) => Payload::Url(r),
                Err(e) => return Err(e),
            },
            Self::Cbor => {
                let mut buffer: Vec<u8> = Vec::new();
                match ciborium::ser::into_writer(&value, &mut buffer)
                    .map_err(|e| ServerFnError::Serialization(e.to_string()))
                {
                    Ok(_) => Payload::Binary(buffer),
                    Err(e) => return Err(e),
                }
            }
        })
    }
}
