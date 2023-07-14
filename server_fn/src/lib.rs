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
//! **Important**: Before calling a server function on a non-web platform, you must set the server URL by calling [`set_server_url`].
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
//!   log::debug!("posts = {posts:#?}");
//! }
//! # }
//!
//! // make sure you've registered it somewhere in main
//! fn main() {
//!   // for non-web apps, you must set the server URL manually
//!   server_fn::set_server_url("http://localhost:3000");
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
//! - **Return types must implement [Serialize](serde::Serialize).**
//!   This should be fairly obvious: we have to serialize arguments to send them to the server, and we
//!   need to deserialize the result to return it to the client.
//! - **Arguments must be implement [serde::Serialize].** They are serialized as an `application/x-www-form-urlencoded`
//!   form data using [`serde_qs`](https://docs.rs/serde_qs/latest/serde_qs/) or as `application/cbor`
//!   using [`cbor`](https://docs.rs/cbor/latest/cbor/).

// used by the macro
#[doc(hidden)]
pub use const_format;
// used by the macro
#[cfg(feature = "ssr")]
#[doc(hidden)]
pub use inventory;
#[cfg(any(feature = "ssr", doc))]
use proc_macro2::TokenStream;
#[cfg(any(feature = "ssr", doc))]
use quote::TokenStreamExt;
// used by the macro
#[doc(hidden)]
pub use serde;
use serde::{de::DeserializeOwned, Serialize};
pub use server_fn_macro_default::server;
use std::{future::Future, pin::Pin, str::FromStr};
#[cfg(any(feature = "ssr", doc))]
use syn::parse_quote;
// used by the macro
#[doc(hidden)]
pub use xxhash_rust;
/// Error types used in server functions.
pub mod error;
pub use error::ServerFnError;

/// Default server function registry
pub mod default;

/// Something that can register a server function.
pub trait ServerFunctionRegistry<T> {
    /// An error that can occur when registering a server function.
    type Error: std::error::Error;

    /// Registers a server function at the given URL.
    #[deprecated = "Explicit server function registration is no longer \
                    required on most platforms (including Linux, macOS, iOS, \
                    FreeBSD, Android, and Windows). If you are on another \
                    platform and need to explicitly register server functions, \
                    call ServerFn::register_explicit() instead."]
    fn register(
        url: &'static str,
        server_function: SerializedFnTraitObj<T>,
        encoding: Encoding,
    ) -> Result<(), Self::Error>;

    /// Server functions are automatically registered on most platforms, (including Linux, macOS,
    /// iOS, FreeBSD, Android, and Windows). If you are on another platform, like a WASM server runtime,
    /// this will explicitly register server functions.
    fn register_explicit(
        prefix: &'static str,
        url: &'static str,
        server_function: SerializedFnTraitObj<T>,
        encoding: Encoding,
    ) -> Result<(), Self::Error>;

    /// Returns the server function registered at the given URL, or `None` if no function is registered at that URL.
    fn get(url: &str) -> Option<ServerFnTraitObj<T>>;

    /// Returns the server function registered at the given URL, or `None` if no function is registered at that URL.
    fn get_trait_obj(url: &str) -> Option<ServerFnTraitObj<T>>;
    /// Returns the encoding of the server FN at the given URL, or `None` if no function is
    /// registered at that URL
    fn get_encoding(url: &str) -> Option<Encoding>;
    /// Returns a list of all registered server functions.
    fn paths_registered() -> Vec<&'static str>;
}

/// A server function that can be called from the client.
pub type SerializedFnTraitObj<T> =
    fn(
        T,
        &[u8],
    ) -> Pin<Box<dyn Future<Output = Result<Payload, ServerFnError>>>>;

/// A server function that can be called from the client.
#[derive(Clone)]
pub struct ServerFnTraitObj<T> {
    pub(crate) prefix: &'static str,
    pub(crate) url: &'static str,
    pub(crate) encoding: Encoding,
    pub(crate) run: SerializedFnTraitObj<T>,
}

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
///     if let Some(server_fn) = server_fn_by_path::<MyRegistry>(path.as_str()) {
///         let body: &[u8] = &body;
///         match (server_fn.trait_obj)(&body).await {
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
pub fn server_fn_by_path<T: 'static, R: ServerFunctionRegistry<T>>(
    path: &str,
) -> Option<ServerFnTraitObj<T>> {
    R::get(path)
}

/// Returns a trait obj of the server fn for calling purposes
#[cfg(any(feature = "ssr", doc))]
pub fn server_fn_trait_obj_by_path<T: 'static, R: ServerFunctionRegistry<T>>(
    path: &str,
) -> Option<ServerFnTraitObj<T>> {
    R::get_trait_obj(path)
}

/// Returns the Encoding of the server fn  at a particular path
#[cfg(any(feature = "ssr", doc))]
pub fn server_fn_encoding_by_path<T: 'static, R: ServerFunctionRegistry<T>>(
    path: &str,
) -> Option<Encoding> {
    R::get_encoding(path)
}

/// Returns the set of currently-registered server function paths, for debugging purposes.
#[cfg(any(feature = "ssr", doc))]
pub fn server_fns_by_path<T: 'static, R: ServerFunctionRegistry<T>>(
) -> Vec<&'static str> {
    R::paths_registered()
}

/// Holds the current options for encoding types.
/// More could be added, but they need to be serde
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum Encoding {
    /// A Binary Encoding Scheme Called Cbor
    Cbor,
    /// The Default URL-encoded encoding method
    #[default]
    Url,
    /// Pass arguments to server fns as part of the query string. Cacheable. Returns JSON
    GetJSON,
    /// Pass arguments to server fns as part of the query string. Cacheable. Returns CBOR
    GetCBOR,
}

impl FromStr for Encoding {
    type Err = ();

    fn from_str(input: &str) -> Result<Encoding, Self::Err> {
        match input {
            "URL" => Ok(Encoding::Url),
            "Cbor" => Ok(Encoding::Cbor),
            "GetCbor" => Ok(Encoding::GetCBOR),
            "GetJson" => Ok(Encoding::GetJSON),
            _ => Err(()),
        }
    }
}

#[cfg(any(feature = "ssr", doc))]
impl quote::ToTokens for Encoding {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let option: syn::Ident = match *self {
            Encoding::Cbor => parse_quote!(Cbor),
            Encoding::Url => parse_quote!(Url),
            Encoding::GetJSON => parse_quote!(GetJSON),
            Encoding::GetCBOR => parse_quote!(GetCBOR),
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
    type Output: serde::Serialize;

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
        data: &[u8],
    ) -> Pin<Box<dyn Future<Output = Result<Payload, ServerFnError>>>> {
        // decode the args
        let value = match Self::encoding() {
            Encoding::Url | Encoding::GetJSON | Encoding::GetCBOR => {
                serde_qs::Config::new(5, false)
                    .deserialize_bytes(data)
                    .map_err(|e| ServerFnError::Deserialization(e.to_string()))
            }
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
                Encoding::Url | Encoding::GetJSON => {
                    match serde_json::to_string(&result).map_err(|e| {
                        ServerFnError::Serialization(e.to_string())
                    }) {
                        Ok(r) => Payload::Url(r),
                        Err(e) => return Err(e),
                    }
                }
                Encoding::Cbor | Encoding::GetCBOR => {
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

    /// Registers the server function, allowing the server to query it by URL.
    ///
    /// This function is deprecated, as server functions are now registered automatically.
    #[cfg(any(feature = "ssr", doc,))]
    #[deprecated = "Explicit server function registration is no longer \
                    required on most platforms (including Linux, macOS, iOS, \
                    FreeBSD, Android, and Windows). If you are on another \
                    platform and need to explicitly register server functions, \
                    call ServerFn::register_explicit() instead."]
    fn register_in<R: ServerFunctionRegistry<T>>() -> Result<(), ServerFnError>
    {
        Ok(())
    }

    /// Registers the server function explicitly on platforms that require it,
    /// allowing the server to query it by URL.
    #[cfg(any(feature = "ssr", doc,))]
    fn register_in_explicit<R: ServerFunctionRegistry<T>>(
    ) -> Result<(), ServerFnError> {
        // store it in the hashmap
        R::register_explicit(
            Self::prefix(),
            Self::url(),
            Self::call_from_bytes,
            Self::encoding(),
        )
        .map_err(|e| ServerFnError::Registration(e.to_string()))
    }
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
    use serde_json::Deserializer as JSONDeserializer;
    #[cfg(not(target_arch = "wasm32"))]
    let url = format!("{}{}", get_server_url(), url);

    #[derive(Debug)]
    enum Payload {
        Binary(Vec<u8>),
        Url(String),
    }
    let args_encoded = match &enc {
        Encoding::Url | Encoding::GetJSON | Encoding::GetCBOR => Payload::Url(
            serde_qs::to_string(&args)
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
        Encoding::Url | Encoding::GetJSON | Encoding::GetCBOR => {
            "application/x-www-form-urlencoded"
        }
        Encoding::Cbor => "application/cbor",
    };

    let accept_header = match &enc {
        Encoding::Url | Encoding::GetJSON => {
            "application/x-www-form-urlencoded"
        }
        Encoding::Cbor | Encoding::GetCBOR => "application/cbor",
    };

    #[cfg(target_arch = "wasm32")]
    let resp = match &enc {
        Encoding::Url | Encoding::Cbor => match args_encoded {
            Payload::Binary(b) => {
                let slice_ref: &[u8] = &b;
                let js_array = js_sys::Uint8Array::from(slice_ref).buffer();
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
        },
        Encoding::GetCBOR | Encoding::GetJSON => match args_encoded {
            Payload::Binary(_) => panic!(
                "Binary data cannot be transferred via GET request in a query \
                 string. Please try using the CBOR encoding."
            ),
            Payload::Url(s) => {
                let full_url = format!("{url}?{s}");
                gloo_net::http::Request::get(&full_url)
                    .header("Content-Type", content_type_header)
                    .header("Accept", accept_header)
                    .send()
                    .await
                    .map_err(|e| ServerFnError::Request(e.to_string()))?
            }
        },
    };
    #[cfg(not(target_arch = "wasm32"))]
    let resp = match &enc {
        Encoding::Url | Encoding::Cbor => match args_encoded {
            Payload::Binary(b) => CLIENT
                .post(url)
                .header("Content-Type", content_type_header)
                .header("Accept", accept_header)
                .body(b)
                .send()
                .await
                .map_err(|e| ServerFnError::Request(e.to_string()))?,
            Payload::Url(s) => CLIENT
                .post(url)
                .header("Content-Type", content_type_header)
                .header("Accept", accept_header)
                .body(s)
                .send()
                .await
                .map_err(|e| ServerFnError::Request(e.to_string()))?,
        },
        Encoding::GetJSON | Encoding::GetCBOR => match args_encoded {
            Payload::Binary(_) => panic!(
                "Binary data cannot be transferred via GET request in a query \
                 string. Please try using the CBOR encoding."
            ),

            Payload::Url(s) => {
                let full_url = format!("{url}?{s}");
                CLIENT
                    .get(full_url)
                    .header("Content-Type", content_type_header)
                    .header("Accept", accept_header)
                    .send()
                    .await
                    .map_err(|e| ServerFnError::Request(e.to_string()))?
            }
        },
    };

    // check for error status
    let status = resp.status();
    #[cfg(not(target_arch = "wasm32"))]
    let status = status.as_u16();
    if (500..=599).contains(&status) {
        let text = resp.text().await.unwrap_or_default();
        #[cfg(target_arch = "wasm32")]
        let status_text = resp.status_text();
        #[cfg(not(target_arch = "wasm32"))]
        let status_text = status.to_string();
        return Err(serde_json::from_str(&text)
            .unwrap_or(ServerFnError::ServerError(status_text)));
    }

    // Decoding the body of the request
    if (enc == Encoding::Cbor) || (enc == Encoding::GetCBOR) {
        #[cfg(target_arch = "wasm32")]
        let binary = resp
            .binary()
            .await
            .map_err(|e| ServerFnError::Deserialization(e.to_string()))?;
        #[cfg(target_arch = "wasm32")]
        let binary = binary.as_slice();
        #[cfg(not(target_arch = "wasm32"))]
        let binary = resp
            .bytes()
            .await
            .map_err(|e| ServerFnError::Deserialization(e.to_string()))?;
        #[cfg(not(target_arch = "wasm32"))]
        let binary = binary.as_ref();

        if status == 400 {
            return Err(ServerFnError::ServerError(
                "No server function was found at this URL.".to_string(),
            ));
        }

        ciborium::de::from_reader(binary)
            .map_err(|e| ServerFnError::Deserialization(e.to_string()))
    } else {
        let text = resp
            .text()
            .await
            .map_err(|e| ServerFnError::Deserialization(e.to_string()))?;

        if status == 400 {
            return Err(ServerFnError::ServerError(text));
        }

        let mut deserializer = JSONDeserializer::from_str(&text);
        T::deserialize(&mut deserializer)
            .map_err(|e| ServerFnError::Deserialization(e.to_string()))
    }
}

// Lazily initialize the client to be reused for all server function calls.
#[cfg(any(all(not(feature = "ssr"), not(target_arch = "wasm32")), doc))]
static CLIENT: once_cell::sync::Lazy<reqwest::Client> =
    once_cell::sync::Lazy::new(reqwest::Client::new);

#[cfg(any(all(not(feature = "ssr"), not(target_arch = "wasm32")), doc))]
static ROOT_URL: once_cell::sync::OnceCell<&'static str> =
    once_cell::sync::OnceCell::new();

#[cfg(any(all(not(feature = "ssr"), not(target_arch = "wasm32")), doc))]
/// Set the root server url that all server function paths are relative to for the client. On WASM this will default to the origin.
pub fn set_server_url(url: &'static str) {
    ROOT_URL.set(url).unwrap();
}

#[cfg(all(not(feature = "ssr"), not(target_arch = "wasm32")))]
fn get_server_url() -> &'static str {
    ROOT_URL
        .get()
        .expect("Call set_root_url before calling a server function.")
}
