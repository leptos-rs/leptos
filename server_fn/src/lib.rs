pub mod client;
pub mod codec;
#[macro_use]
pub mod error;
pub mod middleware;
pub mod redirect;
pub mod request;
pub mod response;

use client::Client;
use codec::{Encoding, FromReq, FromRes, IntoReq, IntoRes};
use dashmap::DashMap;
pub use error::ServerFnError;
use middleware::{Layer, Service};
use once_cell::sync::Lazy;
use request::Req;
use response::{ClientRes, Res};
use serde::{de::DeserializeOwned, Serialize};
use std::{future::Future, pin::Pin, sync::Arc};

// reexports for the sake of the macro
#[doc(hidden)]
pub use const_format;
#[doc(hidden)]
pub use serde;
#[doc(hidden)]
pub use xxhash_rust;

pub trait ServerFn
where
    Self: Send
        + FromReq<Self::Error, Self::ServerRequest, Self::InputEncoding>
        + IntoReq<Self::Error, <Self::Client as Client<Self::Error>>::Request, Self::InputEncoding>,
{
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
    type Output: IntoRes<Self::Error, Self::ServerResponse, Self::OutputEncoding>
        + FromRes<Self::Error, <Self::Client as Client<Self::Error>>::Response, Self::OutputEncoding>
        + Send;

    /// The [`Encoding`] used in the request for arguments into the server function.
    type InputEncoding: Encoding;

    /// The [`Encoding`] used in the response for the result of the server function.
    type OutputEncoding: Encoding;

    /// The type of the custom error on [`ServerFnError`], if any. (If there is no
    /// custom error type, this can be `NoCustomError` by default.)
    type Error: Serialize + DeserializeOwned;

    /// Middleware that should be applied to this server function.
    fn middlewares() -> Vec<Arc<dyn Layer<Self::ServerRequest, Self::ServerResponse>>> {
        Vec::new()
    }

    // The body of the server function. This will only run on the server.
    fn run_body(
        self,
    ) -> impl Future<Output = Result<Self::Output, ServerFnError<Self::Error>>> + Send;

    fn run_on_server(
        req: Self::ServerRequest,
    ) -> impl Future<Output = Self::ServerResponse> + Send {
        async {
            Self::execute_on_server(req)
                .await
                .unwrap_or_else(Self::ServerResponse::error_response)
        }
    }

    fn run_on_client(
        self,
    ) -> impl Future<Output = Result<Self::Output, ServerFnError<Self::Error>>> + Send {
        async move {
            // create and send request on client
            let req = self.into_req(Self::PATH, Self::OutputEncoding::CONTENT_TYPE)?;
            let res = Self::Client::send(req).await?;

            let status = res.status();
            let location = res.location();

            // if it returns an error status, deserialize the error
            // this is the same logic as the current implementation of server fns
            // TODO I don't love that this requires shipping `serde_json` for errors
            let res = if (400..=599).contains(&status) {
                let status_text = res.status_text();
                let text = res.try_into_string().await?;
                match serde_json::from_str(&text) {
                    Ok(e) => Err(e),
                    Err(_) => Err(ServerFnError::ServerError(if text.is_empty() {
                        format!("{} {}", status, status_text)
                    } else {
                        format!("{} {}: {}", status, status_text, text)
                    })),
                }
            } else {
                // otherwise, deserialize the body as is
                Self::Output::from_res(res).await
            };

            // if redirected, call the redirect hook (if that's been set)
            if (300..=399).contains(&status) {
                redirect::call_redirect_hook(&location);
            }

            res
        }
    }

    #[doc(hidden)]
    fn execute_on_server(
        req: Self::ServerRequest,
    ) -> impl Future<Output = Result<Self::ServerResponse, ServerFnError<Self::Error>>> + Send {
        async {
            let this = Self::from_req(req).await?;
            let output = this.run_body().await?;
            let res = output.into_res().await?;
            Ok(res)
        }
    }

    fn url() -> &'static str {
        Self::PATH
    }
}

#[doc(hidden)]
pub use inventory;

#[macro_export]
macro_rules! initialize_server_fn_map {
    ($req:ty, $res:ty) => {
        once_cell::sync::Lazy::new(|| {
            $crate::inventory::iter::<ServerFnTraitObj<$req, $res>>
                .into_iter()
                .map(|obj| (obj.path(), *obj))
                .collect()
        })
    };
}

pub struct ServerFnTraitObj<Req, Res> {
    path: &'static str,
    handler: fn(Req) -> Pin<Box<dyn Future<Output = Res> + Send>>,
    middleware: fn() -> Vec<Arc<dyn Layer<Req, Res>>>,
}

impl<Req, Res> ServerFnTraitObj<Req, Res> {
    pub const fn new(
        path: &'static str,
        handler: fn(Req) -> Pin<Box<dyn Future<Output = Res> + Send>>,
        middleware: fn() -> Vec<Arc<dyn Layer<Req, Res>>>,
    ) -> Self {
        Self {
            path,
            handler,
            middleware,
        }
    }

    pub fn path(&self) -> &'static str {
        self.path
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
        *self
    }
}

impl<Req, Res> Copy for ServerFnTraitObj<Req, Res> {}

type LazyServerFnMap<Req, Res> = Lazy<DashMap<&'static str, ServerFnTraitObj<Req, Res>>>;

// Axum integration
#[cfg(feature = "axum")]
pub mod axum {
    use crate::{
        middleware::{BoxedService, Layer, Service},
        LazyServerFnMap, ServerFn, ServerFnTraitObj,
    };
    use axum::body::Body;
    use http::{Request, Response, StatusCode};

    inventory::collect!(ServerFnTraitObj<Request<Body>, Response<Body>>);

    static REGISTERED_SERVER_FUNCTIONS: LazyServerFnMap<Request<Body>, Response<Body>> =
        initialize_server_fn_map!(Request<Body>, Response<Body>);

    pub fn register_explicit<T>()
    where
        T: ServerFn<ServerRequest = Request<Body>, ServerResponse = Response<Body>> + 'static,
    {
        REGISTERED_SERVER_FUNCTIONS.insert(
            T::PATH,
            ServerFnTraitObj::new(
                T::PATH,
                |req| Box::pin(T::run_on_server(req)),
                T::middlewares,
            ),
        );
    }

    pub async fn handle_server_fn(req: Request<Body>) -> Response<Body> {
        let path = req.uri().path();

        if let Some(server_fn) = REGISTERED_SERVER_FUNCTIONS.get(path) {
            let middleware = (server_fn.middleware)();
            let mut service = BoxedService::new(*server_fn);
            for middleware in middleware {
                service = middleware.layer(service);
            }
            service.run(req).await
        } else {
            Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Body::from(format!(
                    "Could not find a server function at the route {path}. \n\nIt's likely that either\n 1. The API prefix you specify in the `#[server]` macro doesn't match the prefix at which your server function handler is mounted, or \n2. You are on a platform that doesn't support automatic server function registration and you need to call ServerFn::register_explicit() on the server function type, somewhere in your `main` function.",
                )))
                .unwrap()
        }
    }
}

// Actix integration
#[cfg(feature = "actix")]
pub mod actix {
    use actix_web::{HttpRequest, HttpResponse};
    use send_wrapper::SendWrapper;

    use crate::request::actix::ActixRequest;
    use crate::response::actix::ActixResponse;
    use crate::{LazyServerFnMap, ServerFn, ServerFnTraitObj};

    inventory::collect!(ServerFnTraitObj<ActixRequest, ActixResponse>);

    static REGISTERED_SERVER_FUNCTIONS: LazyServerFnMap<ActixRequest, ActixResponse> =
        initialize_server_fn_map!(ActixRequest, ActixResponse);

    pub fn register_explicit<T>()
    where
        T: ServerFn<ServerRequest = ActixRequest, ServerResponse = ActixResponse> + 'static,
    {
        REGISTERED_SERVER_FUNCTIONS.insert(
            T::PATH,
            ServerFnTraitObj::new(T::PATH, |req| Box::pin(T::run_on_server(req))),
        );
    }

    pub async fn handle_server_fn(req: HttpRequest) -> HttpResponse {
        let path = req.uri().path();
        if let Some(server_fn) = REGISTERED_SERVER_FUNCTIONS.get(path) {
            server_fn
                .run(ActixRequest(SendWrapper::new(req)))
                .await
                .0
                .take()
        } else {
            HttpResponse::BadRequest().body(format!(
                "Could not find a server function at the route {path}. \n\nIt's likely that either\n 1. The API prefix you specify in the `#[server]` macro doesn't match the prefix at which your server function handler is mounted, or \n2. You are on a platform that doesn't support automatic server function registration and you need to call ServerFn::register_explicit() on the server function type, somewhere in your `main` function.",
            ))
        }
    }
}
