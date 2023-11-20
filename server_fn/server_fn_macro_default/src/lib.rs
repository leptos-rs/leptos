#![cfg_attr(feature = "nightly", feature(proc_macro_span))]
//! This crate contains the default implementation of the #[macro@crate::server] macro without a context from the server. See the [server_fn_macro] crate for more information.
#![forbid(unsafe_code)]

use proc_macro::TokenStream;
use server_fn_macro::server_macro_impl;
use syn::__private::ToTokens;

/// Declares that a function is a [server function](https://docs.rs/server_fn/).
/// This means that its body will only run on the server, i.e., when the `ssr`
/// feature is enabled.
///
/// You can specify one, two, three, or four arguments to the server function:
/// 1. **Required**: A type name that will be used to identify and register the server function
///   (e.g., `MyServerFn`).
/// 2. *Optional*: A URL prefix at which the function will be mounted when it’s registered
///   (e.g., `"/api"`). Defaults to `"/"`.
/// 3. *Optional*: The encoding for the server function (`"Url"`, `"Cbor"`, `"GetJson"`, or `"GetCbor`". See **Server Function Encodings** below.)
/// 4. *Optional*: A specific endpoint path to be used in the URL. (By default, a unique path will be generated.)
///
/// ```rust,ignore
/// // will generate a server function at `/api-prefix/hello`
/// #[server(MyServerFnType, "/api-prefix", "Url", "hello")]
/// ```
///
/// The server function itself can take any number of arguments, each of which should be serializable
/// and deserializable with `serde`.
///
/// ```ignore
/// # use server_fn::*; use serde::{Serialize, Deserialize};
/// # #[derive(Serialize, Deserialize)]
/// # pub struct Post { }
/// #[server(ReadPosts, "/api")]
/// pub async fn read_posts(how_many: u8, query: String) -> Result<Vec<Post>, ServerFnError> {
///   // do some work on the server to access the database
///   todo!()
/// }
/// ```
///
/// Note the following:
/// - **Server functions must be `async`.** Even if the work being done inside the function body
///   can run synchronously on the server, from the client’s perspective it involves an asynchronous
///   function call.
/// - **Server functions must return `Result<T, ServerFnError>`.** Even if the work being done
///   inside the function body can’t fail, the processes of serialization/deserialization and the
///   network call are fallible.
/// - **Return types must implement [Serialize](https://docs.rs/serde/latest/serde/trait.Serialize.html).**
///   This should be fairly obvious: we have to serialize arguments to send them to the server, and we
///   need to deserialize the result to return it to the client.
/// - **Arguments must be implement [`Serialize`](https://docs.rs/serde/latest/serde/trait.Serialize.html)
///   and [`DeserializeOwned`](https://docs.rs/serde/latest/serde/de/trait.DeserializeOwned.html).**
///   They are serialized as an `application/x-www-form-urlencoded`
///   form data using [`serde_qs`](https://docs.rs/serde_qs/latest/serde_qs/) or as `application/cbor`
///   using [`cbor`](https://docs.rs/cbor/latest/cbor/).
#[proc_macro_attribute]
pub fn server(args: proc_macro::TokenStream, s: TokenStream) -> TokenStream {
    match server_macro_impl(
        args.into(),
        s.into(),
        syn::parse_quote!(server_fn::default::DefaultServerFnTraitObj),
        None,
        Some(syn::parse_quote!(server_fn)),
    ) {
        Err(e) => e.to_compile_error().into(),
        Ok(s) => s.to_token_stream().into(),
    }
}
