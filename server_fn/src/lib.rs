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
//! The [`#[server]`](../leptos/attr.server.html) macro allows you to annotate a function to
//! indicate that it should only run on the server (i.e., when you have an `ssr` feature in your
//! crate that is enabled).
//!
//! **Important**: Before calling a server function on a non-web platform, you must set the server URL by calling
//! [`set_server_url`](crate::client::set_server_url).
//!
//! ```rust,ignore
//! #[server]
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
//! ```
//!
//! If you call this function from the client, it will serialize the function arguments and `POST`
//! them to the server as if they were the URL-encoded inputs in `<form method="post">`.
//!
//! Here’s what you need to remember:
//! - **Server functions must be `async`.** Even if the work being done inside the function body
//!   can run synchronously on the server, from the client’s perspective it involves an asynchronous
//!   function call.
//! - **Server functions must return `Result<T, ServerFnError>`.** Even if the work being done
//!   inside the function body can’t fail, the processes of serialization/deserialization and the
//!   network call are fallible. [`ServerFnError`] can receive generic errors.
//! - **Server functions are part of the public API of your application.** A server function is an
//!   ad hoc HTTP API endpoint, not a magic formula. Any server function can be accessed by any HTTP
//!   client. You should take care to sanitize any data being returned from the function to ensure it
//!   does not leak data that should exist only on the server.
//! - **Server functions can’t be generic.** Because each server function creates a separate API endpoint,
//!   it is difficult to monomorphize. As a result, server functions cannot be generic (for now?) If you need to use
//!   a generic function, you can define a generic inner function called by multiple concrete server functions.
//! - **Arguments and return types must be serializable.** We support a variety of different encodings,
//!   but one way or another arguments need to be serialized to be sent to the server and deserialized
//!   on the server, and the return type must be serialized on the server and deserialized on the client.
//!   This means that the set of valid server function argument and return types is a subset of all
//!   possible Rust argument and return types. (i.e., server functions are strictly more limited than
//!   ordinary functions.)
//!
//! ## Server Function Encodings
//!
//! Server functions are designed to allow a flexible combination of input and output encodings, the set
//! of which can be found in the [`codec`] module.
//!
//! The serialization/deserialization process for server functions consists of a series of steps,
//! each of which is represented by a different trait:
//! 1. [`IntoReq`]: The client serializes the [`ServerFn`] argument type into an HTTP request.
//! 2. The [`Client`] sends the request to the server.
//! 3. [`FromReq`]: The server deserializes the HTTP request back into the [`ServerFn`] type.
//! 4. The server calls calls [`ServerFn::run_body`] on the data.
//! 5. [`IntoRes`]: The server serializes the [`ServerFn::Output`] type into an HTTP response.
//! 6. The server integration applies any middleware from [`ServerFn::middlewares`] and responds to the request.
//! 7. [`FromRes`]: The client deserializes the response back into the [`ServerFn::Output`] type.
//!
//! [server]: ../leptos/attr.server.html
//! [`serde_qs`]: <https://docs.rs/serde_qs/latest/serde_qs/>
//! [`cbor`]: <https://docs.rs/cbor/latest/cbor/>

/// Implementations of the client side of the server function call.
pub mod client;

/// Encodings for arguments and results.
pub mod codec;

#[macro_use]
/// Error types and utilities.
pub mod error;
/// Types to add server middleware to a server function.
pub mod middleware;
/// Utilities to allow client-side redirects.
pub mod redirect;
/// Types and traits for  for HTTP requests.
pub mod request;
/// Types and traits for HTTP responses.
pub mod response;

#[cfg(feature = "actix")]
#[doc(hidden)]
pub use ::actix_web as actix_export;
#[cfg(feature = "axum-no-default")]
#[doc(hidden)]
pub use ::axum as axum_export;
use client::Client;
use codec::{Encoding, FromReq, FromRes, IntoReq, IntoRes};
#[doc(hidden)]
pub use const_format;
use dashmap::DashMap;
pub use error::ServerFnError;
use error::ServerFnErrorSerde;
#[cfg(feature = "form-redirects")]
use error::ServerFnUrlError;
use http::Method;
use middleware::{Layer, Service};
use once_cell::sync::Lazy;
use redirect::RedirectHook;
use request::Req;
use response::{ClientRes, Res};
#[cfg(feature = "rkyv")]
pub use rkyv;
#[doc(hidden)]
pub use serde;
#[doc(hidden)]
#[cfg(feature = "serde-lite")]
pub use serde_lite;
use std::{fmt::Display, future::Future, pin::Pin, str::FromStr, sync::Arc};
#[doc(hidden)]
pub use xxhash_rust;

/// Defines a function that runs only on the server, but can be called from the server or the client.
///
/// The type for which `ServerFn` is implemented is actually the type of the arguments to the function,
/// while the function body itself is implemented in [`run_body`](ServerFn::run_body).
///
/// This means that `Self` here is usually a struct, in which each field is an argument to the function.
/// In other words,
/// ```rust,ignore
/// #[server]
/// pub async fn my_function(foo: String, bar: usize) -> Result<usize, ServerFnError> {
///     Ok(foo.len() + bar)
/// }
/// ```
/// should expand to
/// ```rust,ignore
/// #[derive(Serialize, Deserialize)]
/// pub struct MyFunction {
///     foo: String,
///     bar: usize
/// }
///
/// impl ServerFn for MyFunction {
///     async fn run_body() -> Result<usize, ServerFnError> {
///         Ok(foo.len() + bar)
///     }
///
///     // etc.
/// }
/// ```
pub trait ServerFn
where
    Self: Send
        + FromReq<Self::InputEncoding, Self::ServerRequest, Self::Error>
        + IntoReq<
            Self::InputEncoding,
            <Self::Client as Client<Self::Error>>::Request,
            Self::Error,
        >,
{
    /// A unique path for the server function’s API endpoint, relative to the host, including its prefix.
    const PATH: &'static str;

    /// The type of the HTTP client that will send the request from the client side.
    ///
    /// For example, this might be `gloo-net` in the browser, or `reqwest` for a desktop app.
    type Client: Client<Self::Error>;

    /// The type of the HTTP request when received by the server function on the server side.
    type ServerRequest: Req<Self::Error> + Send;

    /// The type of the HTTP response returned by the server function on the server side.
    type ServerResponse: Res<Self::Error> + Send;

    /// The return type of the server function.
    ///
    /// This needs to be converted into `ServerResponse` on the server side, and converted
    /// *from* `ClientResponse` when received by the client.
    type Output: IntoRes<Self::OutputEncoding, Self::ServerResponse, Self::Error>
        + FromRes<
            Self::OutputEncoding,
            <Self::Client as Client<Self::Error>>::Response,
            Self::Error,
        > + Send;

    /// The [`Encoding`] used in the request for arguments into the server function.
    type InputEncoding: Encoding;

    /// The [`Encoding`] used in the response for the result of the server function.
    type OutputEncoding: Encoding;

    /// The type of the custom error on [`ServerFnError`], if any. (If there is no
    /// custom error type, this can be `NoCustomError` by default.)
    type Error: FromStr + Display;

    /// Returns [`Self::PATH`].
    fn url() -> &'static str {
        Self::PATH
    }

    /// Middleware that should be applied to this server function.
    fn middlewares(
    ) -> Vec<Arc<dyn Layer<Self::ServerRequest, Self::ServerResponse>>> {
        Vec::new()
    }

    /// The body of the server function. This will only run on the server.
    fn run_body(
        self,
    ) -> impl Future<Output = Result<Self::Output, ServerFnError<Self::Error>>> + Send;

    #[doc(hidden)]
    fn run_on_server(
        req: Self::ServerRequest,
    ) -> impl Future<Output = Self::ServerResponse> + Send {
        // Server functions can either be called by a real Client,
        // or directly by an HTML <form>. If they're accessed by a <form>, default to
        // redirecting back to the Referer.
        #[cfg(feature = "form-redirects")]
        let accepts_html = req
            .accepts()
            .map(|n| n.contains("text/html"))
            .unwrap_or(false);
        #[cfg(feature = "form-redirects")]
        let mut referer = req.referer().as_deref().map(ToOwned::to_owned);

        async move {
            #[allow(unused_variables, unused_mut)]
            // used in form redirects feature
            let (mut res, err) = Self::execute_on_server(req)
                .await
                .map(|res| (res, None))
                .unwrap_or_else(|e| {
                    (
                        Self::ServerResponse::error_response(Self::PATH, &e),
                        Some(e),
                    )
                });

            // if it accepts HTML, we'll redirect to the Referer
            #[cfg(feature = "form-redirects")]
            if accepts_html {
                // if it had an error, encode that error in the URL
                if let Some(err) = err {
                    if let Ok(url) = ServerFnUrlError::new(Self::PATH, err)
                        .to_url(referer.as_deref().unwrap_or("/"))
                    {
                        referer = Some(url.to_string());
                    }
                }
                // otherwise, strip error info from referer URL, as that means it's from a previous
                // call
                else if let Some(referer) = referer.as_mut() {
                    ServerFnUrlError::<Self::Error>::strip_error_info(referer)
                }

                // set the status code and Location header
                res.redirect(referer.as_deref().unwrap_or("/"));
            }

            res
        }
    }

    #[doc(hidden)]
    fn run_on_client(
        self,
    ) -> impl Future<Output = Result<Self::Output, ServerFnError<Self::Error>>> + Send
    {
        async move {
            // create and send request on client
            let req =
                self.into_req(Self::PATH, Self::OutputEncoding::CONTENT_TYPE)?;
            Self::run_on_client_with_req(req, redirect::REDIRECT_HOOK.get())
                .await
        }
    }

    #[doc(hidden)]
    fn run_on_client_with_req(
        req: <Self::Client as Client<Self::Error>>::Request,
        redirect_hook: Option<&RedirectHook>,
    ) -> impl Future<Output = Result<Self::Output, ServerFnError<Self::Error>>> + Send
    {
        async move {
            let res = Self::Client::send(req).await?;

            let status = res.status();
            let location = res.location();
            let has_redirect_header = res.has_redirect();

            // if it returns an error status, deserialize the error using FromStr
            let res = if (400..=599).contains(&status) {
                let text = res.try_into_string().await?;
                Err(ServerFnError::<Self::Error>::de(&text))
            } else {
                // otherwise, deserialize the body as is
                Ok(Self::Output::from_res(res).await)
            }?;

            // if redirected, call the redirect hook (if that's been set)
            if let Some(redirect_hook) = redirect_hook {
                if (300..=399).contains(&status) || has_redirect_header {
                    redirect_hook(&location);
                }
            }
            res
        }
    }

    /// Runs the server function (on the server), bubbling up an `Err(_)` after any stage.
    #[doc(hidden)]
    fn execute_on_server(
        req: Self::ServerRequest,
    ) -> impl Future<
        Output = Result<Self::ServerResponse, ServerFnError<Self::Error>>,
    > + Send {
        async {
            let this = Self::from_req(req).await?;
            let output = this.run_body().await?;
            let res = output.into_res().await?;
            Ok(res)
        }
    }
}

#[cfg(feature = "ssr")]
#[doc(hidden)]
pub use inventory;

/// Uses the `inventory` crate to initialize a map between paths and server functions.
#[macro_export]
macro_rules! initialize_server_fn_map {
    ($req:ty, $res:ty) => {
        once_cell::sync::Lazy::new(|| {
            $crate::inventory::iter::<ServerFnTraitObj<$req, $res>>
                .into_iter()
                .map(|obj| (obj.path(), obj.clone()))
                .collect()
        })
    };
}

/// A list of middlewares that can be applied to a server function.
pub type MiddlewareSet<Req, Res> = Vec<Arc<dyn Layer<Req, Res>>>;

/// A trait object that allows multiple server functions that take the same
/// request type and return the same response type to be gathered into a single
/// collection.
pub struct ServerFnTraitObj<Req, Res> {
    path: &'static str,
    method: Method,
    handler: fn(Req) -> Pin<Box<dyn Future<Output = Res> + Send>>,
    middleware: fn() -> MiddlewareSet<Req, Res>,
}

impl<Req, Res> ServerFnTraitObj<Req, Res> {
    /// Converts the relevant parts of a server function into a trait object.
    pub const fn new(
        path: &'static str,
        method: Method,
        handler: fn(Req) -> Pin<Box<dyn Future<Output = Res> + Send>>,
        middleware: fn() -> MiddlewareSet<Req, Res>,
    ) -> Self {
        Self {
            path,
            method,
            handler,
            middleware,
        }
    }

    /// The path of the server function.
    pub fn path(&self) -> &'static str {
        self.path
    }

    /// The HTTP method the server function expects.
    pub fn method(&self) -> Method {
        self.method.clone()
    }

    /// The handler for this server function.
    pub fn handler(&self, req: Req) -> impl Future<Output = Res> + Send {
        (self.handler)(req)
    }

    /// The set of middleware that should be applied to this function.
    pub fn middleware(&self) -> MiddlewareSet<Req, Res> {
        (self.middleware)()
    }
}

impl<Req, Res> Service<Req, Res> for ServerFnTraitObj<Req, Res>
where
    Req: Send + 'static,
    Res: 'static,
{
    fn run(&mut self, req: Req) -> Pin<Box<dyn Future<Output = Res> + Send>> {
        let handler = self.handler;
        Box::pin(async move { handler(req).await })
    }
}

impl<Req, Res> Clone for ServerFnTraitObj<Req, Res> {
    fn clone(&self) -> Self {
        Self {
            path: self.path,
            method: self.method.clone(),
            handler: self.handler,
            middleware: self.middleware,
        }
    }
}

#[allow(unused)] // used by server integrations
type LazyServerFnMap<Req, Res> =
    Lazy<DashMap<&'static str, ServerFnTraitObj<Req, Res>>>;

#[cfg(feature = "ssr")]
impl<Req: 'static, Res: 'static> inventory::Collect
    for ServerFnTraitObj<Req, Res>
{
    #[inline]
    fn registry() -> &'static inventory::Registry {
        static REGISTRY: inventory::Registry = inventory::Registry::new();
        &REGISTRY
    }
}

/// Axum integration.
#[cfg(feature = "axum-no-default")]
pub mod axum {
    use crate::{
        middleware::{BoxedService, Service},
        Encoding, LazyServerFnMap, ServerFn, ServerFnTraitObj,
    };
    use axum::body::Body;
    use http::{Method, Request, Response, StatusCode};

    static REGISTERED_SERVER_FUNCTIONS: LazyServerFnMap<
        Request<Body>,
        Response<Body>,
    > = initialize_server_fn_map!(Request<Body>, Response<Body>);

    /// Explicitly register a server function. This is only necessary if you are
    /// running the server in a WASM environment (or a rare environment that the
    /// `inventory` crate won't work in.).
    pub fn register_explicit<T>()
    where
        T: ServerFn<
                ServerRequest = Request<Body>,
                ServerResponse = Response<Body>,
            > + 'static,
    {
        REGISTERED_SERVER_FUNCTIONS.insert(
            T::PATH,
            ServerFnTraitObj::new(
                T::PATH,
                T::InputEncoding::METHOD,
                |req| Box::pin(T::run_on_server(req)),
                T::middlewares,
            ),
        );
    }

    /// The set of all registered server function paths.
    pub fn server_fn_paths() -> impl Iterator<Item = (&'static str, Method)> {
        REGISTERED_SERVER_FUNCTIONS
            .iter()
            .map(|item| (item.path(), item.method()))
    }

    /// An Axum handler that responds to a server function request.
    pub async fn handle_server_fn(req: Request<Body>) -> Response<Body> {
        let path = req.uri().path();

        if let Some(mut service) = get_server_fn_service(path) {
            service.run(req).await
        } else {
            Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Body::from(format!(
                    "Could not find a server function at the route {path}. \
                     \n\nIt's likely that either\n 1. The API prefix you \
                     specify in the `#[server]` macro doesn't match the \
                     prefix at which your server function handler is mounted, \
                     or \n2. You are on a platform that doesn't support \
                     automatic server function registration and you need to \
                     call ServerFn::register_explicit() on the server \
                     function type, somewhere in your `main` function.",
                )))
                .unwrap()
        }
    }

    /// Returns the server function at the given path as a service that can be modified.
    pub fn get_server_fn_service(
        path: &str,
    ) -> Option<BoxedService<Request<Body>, Response<Body>>> {
        REGISTERED_SERVER_FUNCTIONS.get(path).map(|server_fn| {
            let middleware = (server_fn.middleware)();
            let mut service = BoxedService::new(server_fn.clone());
            for middleware in middleware {
                service = middleware.layer(service);
            }
            service
        })
    }
}

/// Actix integration.
#[cfg(feature = "actix")]
pub mod actix {
    use crate::{
        middleware::BoxedService, request::actix::ActixRequest,
        response::actix::ActixResponse, Encoding, LazyServerFnMap, ServerFn,
        ServerFnTraitObj,
    };
    use actix_web::{web::Payload, HttpRequest, HttpResponse};
    use http::Method;
    #[doc(hidden)]
    pub use send_wrapper::SendWrapper;

    static REGISTERED_SERVER_FUNCTIONS: LazyServerFnMap<
        ActixRequest,
        ActixResponse,
    > = initialize_server_fn_map!(ActixRequest, ActixResponse);

    /// Explicitly register a server function. This is only necessary if you are
    /// running the server in a WASM environment (or a rare environment that the
    /// `inventory` crate won't work in.).
    pub fn register_explicit<T>()
    where
        T: ServerFn<
                ServerRequest = ActixRequest,
                ServerResponse = ActixResponse,
            > + 'static,
    {
        REGISTERED_SERVER_FUNCTIONS.insert(
            T::PATH,
            ServerFnTraitObj::new(
                T::PATH,
                T::InputEncoding::METHOD,
                |req| Box::pin(T::run_on_server(req)),
                T::middlewares,
            ),
        );
    }

    /// The set of all registered server function paths.
    pub fn server_fn_paths() -> impl Iterator<Item = (&'static str, Method)> {
        REGISTERED_SERVER_FUNCTIONS
            .iter()
            .map(|item| (item.path(), item.method()))
    }

    /// An Actix handler that responds to a server function request.
    pub async fn handle_server_fn(
        req: HttpRequest,
        payload: Payload,
    ) -> HttpResponse {
        let path = req.uri().path();
        if let Some(server_fn) = REGISTERED_SERVER_FUNCTIONS.get(path) {
            let middleware = (server_fn.middleware)();
            // http::Method is the only non-Copy type here
            let mut service = BoxedService::new(server_fn.clone());
            for middleware in middleware {
                service = middleware.layer(service);
            }
            service
                .0
                .run(ActixRequest::from((req, payload)))
                .await
                .0
                .take()
        } else {
            HttpResponse::BadRequest().body(format!(
                "Could not find a server function at the route {path}. \
                 \n\nIt's likely that either\n 1. The API prefix you specify \
                 in the `#[server]` macro doesn't match the prefix at which \
                 your server function handler is mounted, or \n2. You are on \
                 a platform that doesn't support automatic server function \
                 registration and you need to call \
                 ServerFn::register_explicit() on the server function type, \
                 somewhere in your `main` function.",
            ))
        }
    }

    /// Returns the server function at the given path as a service that can be modified.
    pub fn get_server_fn_service(
        path: &str,
    ) -> Option<BoxedService<ActixRequest, ActixResponse>> {
        REGISTERED_SERVER_FUNCTIONS.get(path).map(|server_fn| {
            let middleware = (server_fn.middleware)();
            let mut service = BoxedService::new(server_fn.clone());
            for middleware in middleware {
                service = middleware.layer(service);
            }
            service
        })
    }
}
