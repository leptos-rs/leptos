pub mod client;
pub mod codec;
#[macro_use]
pub mod error;
pub mod middleware;
pub mod redirect;
pub mod request;
pub mod response;

#[cfg(feature = "actix")]
#[doc(hidden)]
pub use ::actix_web as actix_export;
#[cfg(feature = "axum")]
#[doc(hidden)]
pub use ::axum as axum_export;
use client::Client;
use codec::{Encoding, FromReq, FromRes, IntoReq, IntoRes};
#[doc(hidden)]
pub use const_format;
use dashmap::DashMap;
pub use error::ServerFnError;
use error::ServerFnErrorSerde;
use http::Method;
use middleware::{Layer, Service};
use once_cell::sync::Lazy;
use request::Req;
use response::{ClientRes, Res};
#[doc(hidden)]
pub use serde;
use std::{fmt::Display, future::Future, pin::Pin, str::FromStr, sync::Arc};
#[doc(hidden)]
pub use xxhash_rust;

pub trait ServerFn
where
    Self: Send
        + FromReq<Self::Error, Self::ServerRequest, Self::InputEncoding>
        + IntoReq<
            Self::Error,
            <Self::Client as Client<Self::Error>>::Request,
            Self::InputEncoding,
        >,
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
        + FromRes<
            Self::Error,
            <Self::Client as Client<Self::Error>>::Response,
            Self::OutputEncoding,
        > + Send;

    /// The [`Encoding`] used in the request for arguments into the server function.
    type InputEncoding: Encoding;

    /// The [`Encoding`] used in the response for the result of the server function.
    type OutputEncoding: Encoding;

    /// The type of the custom error on [`ServerFnError`], if any. (If there is no
    /// custom error type, this can be `NoCustomError` by default.)
    type Error: FromStr + Display;

    /// Middleware that should be applied to this server function.
    fn middlewares(
    ) -> Vec<Arc<dyn Layer<Self::ServerRequest, Self::ServerResponse>>> {
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
    ) -> impl Future<Output = Result<Self::Output, ServerFnError<Self::Error>>> + Send
    {
        async move {
            // create and send request on client
            let req =
                self.into_req(Self::PATH, Self::OutputEncoding::CONTENT_TYPE)?;
            Self::run_on_client_with_req(req).await
        }
    }

    fn run_on_client_with_req(
        req: <Self::Client as Client<Self::Error>>::Request,
    ) -> impl Future<Output = Result<Self::Output, ServerFnError<Self::Error>>> + Send
    {
        async move {
            let res = Self::Client::send(req).await?;

            let status = res.status();
            let location = res.location();

            // if it returns an error status, deserialize the error using FromStr
            let res = if (400..=599).contains(&status) {
                let text = res.try_into_string().await?;
                Err(ServerFnError::<Self::Error>::de(&text))
            } else {
                // otherwise, deserialize the body as is
                Ok(Self::Output::from_res(res).await)
            }?;

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

    fn url() -> &'static str {
        Self::PATH
    }
}

#[cfg(feature = "ssr")]
#[doc(hidden)]
pub use inventory;

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

pub type MiddlewareSet<Req, Res> = Vec<Arc<dyn Layer<Req, Res>>>;

pub struct ServerFnTraitObj<Req, Res> {
    path: &'static str,
    method: Method,
    handler: fn(Req) -> Pin<Box<dyn Future<Output = Res> + Send>>,
    middleware: fn() -> MiddlewareSet<Req, Res>,
}

impl<Req, Res> ServerFnTraitObj<Req, Res> {
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

    pub fn path(&self) -> &'static str {
        self.path
    }

    pub fn method(&self) -> Method {
        self.method.clone()
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

// Axum integration
#[cfg(feature = "axum")]
pub mod axum {
    use crate::{
        middleware::{BoxedService, Service},
        Encoding, LazyServerFnMap, ServerFn, ServerFnTraitObj,
    };
    use axum::body::Body;
    use http::{Method, Request, Response, StatusCode};

    inventory::collect!(ServerFnTraitObj<Request<Body>, Response<Body>>);

    static REGISTERED_SERVER_FUNCTIONS: LazyServerFnMap<
        Request<Body>,
        Response<Body>,
    > = initialize_server_fn_map!(Request<Body>, Response<Body>);

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

    pub fn server_fn_paths() -> impl Iterator<Item = (&'static str, Method)> {
        REGISTERED_SERVER_FUNCTIONS
            .iter()
            .map(|item| (item.path(), item.method()))
    }

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

// Actix integration
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

    inventory::collect!(ServerFnTraitObj<ActixRequest, ActixResponse>);

    static REGISTERED_SERVER_FUNCTIONS: LazyServerFnMap<
        ActixRequest,
        ActixResponse,
    > = initialize_server_fn_map!(ActixRequest, ActixResponse);

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

    pub fn server_fn_paths() -> impl Iterator<Item = (&'static str, Method)> {
        REGISTERED_SERVER_FUNCTIONS
            .iter()
            .map(|item| (item.path(), item.method()))
    }

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
