//#![deny(missing_docs)]
#![forbid(unsafe_code)]

mod action;
pub use action::*;
use std::borrow::Borrow;
mod local_resource;
pub use local_resource::*;
mod multi_action;
pub use multi_action::*;
mod once_resource;
pub use once_resource::*;
mod resource;
pub use resource::*;
mod shared;
////! # Leptos Server Functions
////!
////! This package is based on a simple idea: sometimes it’s useful to write functions
////! that will only run on the server, and call them from the client.
////!
////! If you’re creating anything beyond a toy app, you’ll need to do this all the time:
////! reading from or writing to a database that only runs on the server, running expensive
////! computations using libraries you don’t want to ship down to the client, accessing
////! APIs that need to be called from the server rather than the client for CORS reasons
////! or because you need a secret API key that’s stored on the server and definitely
////! shouldn’t be shipped down to a user’s browser.
////!
////! Traditionally, this is done by separating your server and client code, and by setting
////! up something like a REST API or GraphQL API to allow your client to fetch and mutate
////! data on the server. This is fine, but it requires you to write and maintain your code
////! in multiple separate places (client-side code for fetching, server-side functions to run),
////! as well as creating a third thing to manage, which is the API contract between the two.
////!
////! This package provides two simple primitives that allow you instead to write co-located,
////! isomorphic server functions. (*Co-located* means you can write them in your app code so
////! that they are “located alongside” the client code that calls them, rather than separating
////! the client and server sides. *Isomorphic* means you can call them from the client as if
////! you were simply calling a function; the function call has the “same shape” on the client
////! as it does on the server.)
////!
////! ### `#[server]`
////!
////! The [`#[server]`](https://docs.rs/leptos/latest/leptos/attr.server.html) macro allows you to annotate a function to
////! indicate that it should only run on the server (i.e., when you have an `ssr` feature in your
////! crate that is enabled).
////!
////! ```rust,ignore
////! use leptos::prelude::*;
////! #[server(ReadFromDB)]
////! async fn read_posts(how_many: usize, query: String) -> Result<Vec<Posts>, ServerFnError> {
////!   // do some server-only work here to access the database
////!   let posts = todo!();;
////!   Ok(posts)
////! }
////!
////! // call the function
////! spawn_local(async {
////!   let posts = read_posts(3, "my search".to_string()).await;
////!   log::debug!("posts = {posts:#?}");
////! });
////! ```
////!
////! If you call this function from the client, it will serialize the function arguments and `POST`
////! them to the server as if they were the inputs in `<form method="POST">`.
////!
////! Here’s what you need to remember:
////! - **Server functions must be `async`.** Even if the work being done inside the function body
////!   can run synchronously on the server, from the client’s perspective it involves an asynchronous
////!   function call.
////! - **Server functions must return `Result<T, ServerFnError>`.** Even if the work being done
////!   inside the function body can’t fail, the processes of serialization/deserialization and the
////!   network call are fallible.
////! - **Return types must be [Serializable](leptos_reactive::Serializable).**
////!   This should be fairly obvious: we have to serialize arguments to send them to the server, and we
////!   need to deserialize the result to return it to the client.
////! - **Arguments must be implement [serde::Serialize].** They are serialized as an `application/x-www-form-urlencoded`
////!   form data using [`serde_qs`](https://docs.rs/serde_qs/latest/serde_qs/) or as `application/cbor`
////!   using [`cbor`](https://docs.rs/cbor/latest/cbor/). **Note**: You should explicitly include `serde` with the
////!   `derive` feature enabled in your `Cargo.toml`. You can do this by running `cargo add serde --features=derive`.
////! - Context comes from the server. [`use_context`](leptos_reactive::use_context) can be used to access specific
////!   server-related data, as documented in the server integrations. This allows accessing things like HTTP request
////!   headers as needed. However, server functions *not* have access to reactive state that exists in the client.
////!
////! ## Server Function Encodings
////!
////! By default, the server function call is a `POST` request that serializes the arguments as URL-encoded form data in the body
////! of the request. But there are a few other methods supported. Optionally, we can provide another argument to the `#[server]`
////! macro to specify an alternate encoding:
////!
////! ```rust,ignore
////! #[server(AddTodo, "/api", "Url")]
////! #[server(AddTodo, "/api", "GetJson")]
////! #[server(AddTodo, "/api", "Cbor")]
////! #[server(AddTodo, "/api", "GetCbor")]
////! ```
////!
////! The four options use different combinations of HTTP verbs and encoding methods:
////!
////! | Name              | Method | Request     | Response |
////! | ----------------- | ------ | ----------- | -------- |
////! | **Url** (default) | POST   | URL encoded | JSON     |
////! | **GetJson**       | GET    | URL encoded | JSON     |
////! | **Cbor**          | POST   | CBOR        | CBOR     |
////! | **GetCbor**       | GET    | URL encoded | CBOR     |
////!
////! In other words, you have two choices:
////!
////! - `GET` or `POST`? This has implications for things like browser or CDN caching; while `POST` requests should not be cached,
////! `GET` requests can be.
////! - Plain text (arguments sent with URL/form encoding, results sent as JSON) or a binary format (CBOR, encoded as a base64
////! string)?
////!
////! ## Why not `PUT` or `DELETE`? Why URL/form encoding, and not JSON?**
////!
////! These are reasonable questions. Much of the web is built on REST API patterns that encourage the use of semantic HTTP
////! methods like `DELETE` to delete an item from a database, and many devs are accustomed to sending data to APIs in the
////! JSON format.
////!
////! The reason we use `POST` or `GET` with URL-encoded data by default is the `<form>` support. For better or for worse,
////! HTML forms don’t support `PUT` or `DELETE`, and they don’t support sending JSON. This means that if you use anything
////! but a `GET` or `POST` request with URL-encoded data, it can only work once WASM has loaded.
////!
////! The CBOR encoding is supported for historical reasons; an earlier version of server functions used a URL encoding that
////! didn’t support nested objects like structs or vectors as server function arguments, which CBOR did. But note that the
////! CBOR forms encounter the same issue as `PUT`, `DELETE`, or JSON: they do not degrade gracefully if the WASM version of
////! your app is not available.

//pub use server_fn::{error::ServerFnErrorErr, ServerFnError};

//mod action;
//mod multi_action;
//pub use action::*;
//pub use multi_action::*;
//extern crate tracing;
use base64::{engine::general_purpose::STANDARD_NO_PAD, DecodeError, Engine};
pub use shared::*;
pub trait IntoEncodedString {
    fn into_encoded_string(self) -> String;
}

pub trait FromEncodedStr {
    type DecodedType<'a>: Borrow<Self>;
    type DecodingError;

    fn from_encoded_str(
        data: &str,
    ) -> Result<Self::DecodedType<'_>, Self::DecodingError>;
}

impl IntoEncodedString for String {
    fn into_encoded_string(self) -> String {
        self
    }
}

impl FromEncodedStr for str {
    type DecodedType<'a> = &'a str;
    type DecodingError = ();

    fn from_encoded_str(
        data: &str,
    ) -> Result<Self::DecodedType<'_>, Self::DecodingError> {
        Ok(data)
    }
}

impl IntoEncodedString for Vec<u8> {
    fn into_encoded_string(self) -> String {
        STANDARD_NO_PAD.encode(self)
    }
}

impl FromEncodedStr for [u8] {
    type DecodedType<'a> = Vec<u8>;
    type DecodingError = DecodeError;

    fn from_encoded_str(
        data: &str,
    ) -> Result<Self::DecodedType<'_>, Self::DecodingError> {
        STANDARD_NO_PAD.decode(data)
    }
}

#[cfg(feature = "tachys")]
mod view_implementations {
    use crate::Resource;
    use reactive_graph::traits::Read;
    use std::{future::Future, pin::Pin};
    use tachys::{
        html::attribute::Attribute,
        hydration::Cursor,
        reactive_graph::{RenderEffectState, Suspend, SuspendState},
        ssr::StreamBuilder,
        view::{
            add_attr::AddAnyAttr, Position, PositionState, Render, RenderHtml,
        },
    };

    impl<T, Ser> Render for Resource<T, Ser>
    where
        T: Render + Send + Sync + Clone,
        Ser: Send + 'static,
    {
        type State = RenderEffectState<SuspendState<T>>;

        fn build(self) -> Self::State {
            (move || Suspend::new(async move { self.await })).build()
        }

        fn rebuild(self, state: &mut Self::State) {
            (move || Suspend::new(async move { self.await })).rebuild(state)
        }
    }

    impl<T, Ser> AddAnyAttr for Resource<T, Ser>
    where
        T: RenderHtml + Send + Sync + Clone,
        Ser: Send + 'static,
    {
        type Output<SomeNewAttr: Attribute> = Box<
            dyn FnMut() -> Suspend<
                    Pin<
                        Box<
                            dyn Future<
                                    Output = <T as AddAnyAttr>::Output<
                                        <SomeNewAttr::CloneableOwned as Attribute>::CloneableOwned,
                                    >,
                                > + Send,
                        >,
                    >,
                > + Send,
        >;

        fn add_any_attr<NewAttr: Attribute>(
            self,
            attr: NewAttr,
        ) -> Self::Output<NewAttr>
        where
            Self::Output<NewAttr>: RenderHtml,
        {
            (move || Suspend::new(async move { self.await })).add_any_attr(attr)
        }
    }

    impl<T, Ser> RenderHtml for Resource<T, Ser>
    where
        T: RenderHtml + Send + Sync + Clone,
        Ser: Send + 'static,
    {
        type AsyncOutput = Option<T>;

        const MIN_LENGTH: usize = 0;

        fn dry_resolve(&mut self) {
            self.read();
        }

        fn resolve(self) -> impl Future<Output = Self::AsyncOutput> + Send {
            (move || Suspend::new(async move { self.await })).resolve()
        }

        fn to_html_with_buf(
            self,
            buf: &mut String,
            position: &mut Position,
            escape: bool,
            mark_branches: bool,
        ) {
            (move || Suspend::new(async move { self.await })).to_html_with_buf(
                buf,
                position,
                escape,
                mark_branches,
            );
        }

        fn to_html_async_with_buf<const OUT_OF_ORDER: bool>(
            self,
            buf: &mut StreamBuilder,
            position: &mut Position,
            escape: bool,
            mark_branches: bool,
        ) where
            Self: Sized,
        {
            (move || Suspend::new(async move { self.await }))
                .to_html_async_with_buf::<OUT_OF_ORDER>(
                    buf,
                    position,
                    escape,
                    mark_branches,
                );
        }

        fn hydrate<const FROM_SERVER: bool>(
            self,
            cursor: &Cursor,
            position: &PositionState,
        ) -> Self::State {
            (move || Suspend::new(async move { self.await }))
                .hydrate::<FROM_SERVER>(cursor, position)
        }
    }
}
