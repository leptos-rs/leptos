#![forbid(unsafe_code)]

use std::sync::Arc;

use bytes::Bytes;
use futures::{
    stream::{self, once},
    StreamExt,
};
use http::{
    header::{ACCEPT, LOCATION, REFERER},
    request::Parts,
    HeaderValue, StatusCode, Uri,
};
use hydration_context::SsrSharedContext;
use leptos::{
    prelude::{provide_context, Owner, ScopedFuture},
    server_fn::{
        codec::Encoding, http_export::Request,
        response::generic::Body as ServerFnBody, ServerFn, ServerFnTraitObj,
    },
    IntoView,
};
use leptos_integration_utils::{ExtendResponse, PinnedStream};
use leptos_meta::ServerMetaContext;
use leptos_router::{
    components::provide_server_redirect, location::RequestUrl, PathSegment,
    RouteList, RouteListing, SsrMode,
};
use mime_guess::MimeGuess;
use routefinder::Router;
use server_fn::middleware::Service;
use thiserror::Error;

use wasi::http::types::{
    IncomingRequest, OutgoingBody, OutgoingResponse, ResponseOutparam,
};

use crate::{
    response::{Body, Response, ResponseOptions},
    utils::redirect,
    CHUNK_BYTE_SIZE,
};

/// Handle routing, static file serving and response tx using the low-level
/// `wasi:http` APIs.
///
/// ## Usage
///
/// Please, note that the handler expect to be run with a local Executor initiated.
///
/// ```
/// use leptos_wasi::prelude::Handler;
///
/// let conf = get_configuration(None).unwrap();
/// let leptos_options = conf.leptos_options;
///
/// Handler::build(request, response_out)
///    .expect("could not create handler")
///    // Those two functions should be called first because they can
///    // *shortcut* the handler, see "Performance Considerations".
///
///    // Any HTTP request prefixed with `/pkg` will call the passed
///    // `serve_static_files` function to deliver static files.
///    .static_files_handler("/pkg", serve_static_files)
///    .with_server_fn::<YourServerFn>()
///    // Fetch all available routes from your App.
///    .generate_routes(App)
///    // Actually process the request and write the response.
///    .handle_with_context(move || shell(leptos_options.clone()), || {}).await.expect("could not handle the request");
/// ```
///
/// ## Performance Considerations
///
/// This handler is optimised for the special case of WASI Components being spawned
/// on a per-request basis. That is, the lifetime of the component is bound to the
/// one of the request, so we don't do any fancy pre-setup: it means
/// **your Server-Side will always be cold-started**.
///
/// While it could have a bad impact on the performance of your app, please, know
/// that there is a *shotcut* mechanism implemented that allows the [`Handler`]
/// to shortcut the whole HTTP Rendering and Reactivity logic to directly jump to
/// writting the response in those case:
///
/// * The user request a static-file, then, calling [`Handler::static_files_handler`]
///   will *shortcut* the handler and all future calls are ignored to reach
///   [`Handler::handle_with_context`] *almost* instantly.
/// * The user reach a server function, then, calling [`Handler::with_server_fn`]
///   will check if the request's path matches the one from the passed server functions,
///   if so, *shortcut* the handler.
///
/// This implementation ensures that, even though your component is cold-started
/// on each request, the performance are good. Please, note that this approach is
/// directly enabled by the fact WASI Components have under-millisecond start-up
/// times! It wouldn't be practical to do that with traditional container-based solutions.
///
/// ## Limitations
///
/// [`SsrMode::Static`] is not implemented yet, having one in your `<Router>`
/// will cause [`Handler::handle_with_context`] to panic!
pub struct Handler {
    req: Request<Bytes>,
    res_out: ResponseOutparam,

    // *shortcut* if any is set
    server_fn:
        Option<ServerFnTraitObj<Request<Bytes>, http::Response<ServerFnBody>>>,
    preset_res: Option<Response>,
    should_404: bool,

    // built using the user-defined app_fn
    ssr_router: Router<RouteListing>,
}

impl Handler {
    /// Wraps the WASI resources to handle the request.
    /// Could fail if the [`IncomingRequest`] cannot be converted to
    /// a [`http:Request`].
    pub fn build(
        req: IncomingRequest,
        res_out: ResponseOutparam,
    ) -> Result<Self, HandlerError> {
        Ok(Self {
            req: crate::request::Request(req).try_into()?,
            res_out,
            server_fn: None,
            preset_res: None,
            ssr_router: Router::new(),
            should_404: false,
        })
    }

    // Test whether we are ready to send a response to shortcut some
    // code and provide a fast-path.
    #[inline]
    const fn shortcut(&self) -> bool {
        self.server_fn.is_some() || self.preset_res.is_some() || self.should_404
    }

    /// Tests if the request path matches the bound server function
    /// and *shortcut* the [`Handler`] to quickly reach
    /// the call to [`Handler::handle_with_context`].
    pub fn with_server_fn<T>(mut self) -> Self
    where
        T: ServerFn<
                ServerRequest = Request<Bytes>,
                ServerResponse = http::Response<ServerFnBody>,
            > + 'static,
    {
        if self.shortcut() {
            return self;
        }

        if self.req.method() == T::InputEncoding::METHOD
            && self.req.uri().path() == T::PATH
        {
            self.server_fn = Some(ServerFnTraitObj::new(
                T::PATH,
                T::InputEncoding::METHOD,
                |request| Box::pin(T::run_on_server(request)),
                T::middlewares,
            ));
        }

        self
    }

    /// If the request is prefixed with `prefix` [`Uri`], then
    /// the handler will call the passed `handler` with the Uri trimmed of
    /// the prefix. If the closure returns
    /// None, the response will be 404, otherwise, the returned [`Body`]
    /// will be served as-if.
    ///
    /// This function, when matching, *shortcut* the [`Handler`] to quickly reach
    /// the call to [`Handler::handle_with_context`].
    pub fn static_files_handler<T>(
        mut self,
        prefix: T,
        handler: impl Fn(String) -> Option<Body> + 'static + Send + Clone,
    ) -> Self
    where
        T: TryInto<Uri>,
        <T as TryInto<Uri>>::Error: std::error::Error,
    {
        if self.shortcut() {
            return self;
        }

        if let Some(trimmed_url) = self.req.uri().path().strip_prefix(
            prefix.try_into().expect("you passed an invalid Uri").path(),
        ) {
            match handler(trimmed_url.to_string()) {
                None => self.should_404 = true,
                Some(body) => {
                    let mut res = http::Response::new(body);
                    let mime = MimeGuess::from_path(trimmed_url);

                    res.headers_mut().insert(
                        http::header::CONTENT_TYPE,
                        HeaderValue::from_str(
                            mime.first_or_octet_stream().as_ref(),
                        )
                        .expect("internal error: could not parse MIME type"),
                    );

                    self.preset_res = Some(Response(res));
                }
            }
        }

        self
    }

    /// This mocks a request to the `app_fn` component to extract your
    /// `<Router>`'s `<Routes>`.
    pub fn generate_routes<IV>(
        self,
        app_fn: impl Fn() -> IV + 'static + Send + Clone,
    ) -> Self
    where
        IV: IntoView + 'static,
    {
        self.generate_routes_with_exclusions_and_context(app_fn, None, || {})
    }

    /// This mocks a request to the `app_fn` component to extract your
    /// `<Router>`'s `<Routes>`.
    ///
    /// You can pass an `additional_context` to [`provide_context`] to the
    /// application.
    pub fn generate_routes_with_context<IV>(
        self,
        app_fn: impl Fn() -> IV + 'static + Send + Clone,
        additional_context: impl Fn() + 'static + Send + Clone,
    ) -> Self
    where
        IV: IntoView + 'static,
    {
        self.generate_routes_with_exclusions_and_context(
            app_fn,
            None,
            additional_context,
        )
    }

    /// This mocks a request to the `app_fn` component to extract your
    /// `<Router>`'s `<Routes>`.
    ///
    /// You can pass an `additional_context` to [`provide_context`] to the
    /// application.
    ///
    /// You can pass a list of `excluded_routes` to avoid generating them.
    pub fn generate_routes_with_exclusions_and_context<IV>(
        mut self,
        app_fn: impl Fn() -> IV + 'static + Send + Clone,
        excluded_routes: Option<Vec<String>>,
        additional_context: impl Fn() + 'static + Send + Clone,
    ) -> Self
    where
        IV: IntoView + 'static,
    {
        // If we matched a server function, we do not need to go through
        // all of that.
        if self.shortcut() {
            return self;
        }

        if !self.ssr_router.is_empty() {
            panic!("generate_routes was called twice");
        }

        let owner = Owner::new_root(Some(Arc::new(SsrSharedContext::new())));
        let (mock_meta, _) = ServerMetaContext::new();
        let routes = owner
            .with(|| {
                // as we are generating the app to extract
                // the <Router/>, we want to mock the root path.
                provide_context(RequestUrl::new(""));
                provide_context(ResponseOptions::default());
                provide_context(http::uri::Parts::default());
                provide_context(mock_meta);
                additional_context();
                RouteList::generate(&app_fn)
            })
            .unwrap_or_default()
            .into_inner()
            .into_iter()
            .map(|rt| (rt.path().to_rf_str_representation(), rt))
            .filter(|route| {
                excluded_routes.as_ref().map_or(true, |excluded_routes| {
                    !excluded_routes.iter().any(|ex_path| *ex_path == route.0)
                })
            });

        for (path, route_listing) in routes {
            self.ssr_router
                .add(path, route_listing)
                .expect("internal error: impossible to parse a RouteListing");
        }

        self
    }

    /// Consumes the [`Handler`] to actually perform all the request handling
    /// logic.
    ///
    /// You can pass an `additional_context` to [`provide_context`] to the
    /// application.
    pub async fn handle_with_context<IV>(
        self,
        app: impl Fn() -> IV + 'static + Send + Clone,
        additional_context: impl Fn() + 'static + Clone + Send,
    ) -> Result<(), HandlerError>
    where
        IV: IntoView + 'static,
    {
        let path = self.req.uri().path().to_string();
        let best_match = self.ssr_router.best_match(&path);
        let (parts, body) = self.req.into_parts();
        let context_parts = parts.clone();
        let req = Request::from_parts(parts, body);

        let owner = Owner::new();
        let response = owner.with(|| {
            ScopedFuture::new(async move {
                let res_opts = ResponseOptions::default();
                let response: Option<Response> = if self.should_404 {
                    None
                } else if self.preset_res.is_some() {
                    self.preset_res
                } else if let Some(mut sfn) = self.server_fn {
                    provide_contexts(additional_context, context_parts, res_opts.clone());

                    // store Accepts and Referer in case we need them for redirect (below)
                    let accepts_html = req
                        .headers()
                        .get(ACCEPT)
                        .and_then(|v| v.to_str().ok())
                        .map(|v| v.contains("text/html"))
                        .unwrap_or(false);
                    let referrer = req.headers().get(REFERER).cloned();

                    let mut res = sfn.run(req).await;

                    // if it accepts text/html (i.e., is a plain form post) and doesn't already have a
                    // Location set, then redirect to to Referer
                    if accepts_html {
                        if let Some(referrer) = referrer {
                            let has_location =
                                res.headers().get(LOCATION).is_some();
                            if !has_location {
                                *res.status_mut() = StatusCode::FOUND;
                                res.headers_mut().insert(LOCATION, referrer);
                            }
                        }
                    }

                    Some(res.into())
                } else if let Some(best_match) = best_match {
                    let listing = best_match.handler();
                    let (meta_context, meta_output) = ServerMetaContext::new();

                    let add_ctx = additional_context.clone();
                    let additional_context = {
                        let res_opts = res_opts.clone();
                        let meta_ctx = meta_context.clone();
                        move || {
                            provide_contexts(add_ctx, context_parts, res_opts);
                            provide_context(meta_ctx);
                        }
                    };

                    Some(Response::from_app(
                        app,
                        meta_output,
                        additional_context,
                        res_opts.clone(),
                        match listing.mode() {
                            SsrMode::Async => |app, chunks| {
                                Box::pin(async move {
                                    let app = if cfg!(feature = "islands-router") {
                                        app.to_html_stream_in_order_branching()
                                    } else {
                                        app.to_html_stream_in_order()
                                    };
                                    let app = app.collect::<String>().await;
                                    let chunks = chunks();
                                    Box::pin(once(async move { app }).chain(chunks)) as PinnedStream<String>
                                })
                            },
                            SsrMode::InOrder => |app, chunks| {
                                Box::pin(async move {
                                    let app = if cfg!(feature = "islands-router") {
                                        app.to_html_stream_in_order_branching()
                                    } else {
                                        app.to_html_stream_in_order()
                                    };
                                    Box::pin(app.chain(chunks())) as PinnedStream<String>
                                })
                            },
                            SsrMode::PartiallyBlocked | SsrMode::OutOfOrder => |app, chunks| {
                                Box::pin(async move {
                                    let app = if cfg!(feature = "islands-router") {
                                        app.to_html_stream_out_of_order_branching()
                                    } else {
                                        app.to_html_stream_out_of_order()
                                    };
                                    Box::pin(app.chain(chunks())) as PinnedStream<String>
                                })
                            },
                            SsrMode::Static(_) => panic!("SsrMode::Static routes are not supported yet!")
                        }
                    ).await)
                } else {
                    None
                };

                response.map(|mut req| {
                    req.extend_response(&res_opts);
                    req
                })
            })
        }).await;

        let response = response.unwrap_or_else(|| {
            let body = Bytes::from("404 not found");
            let mut res = http::Response::new(Body::Sync(body));
            *res.status_mut() = http::StatusCode::NOT_FOUND;
            Response(res)
        });

        let headers = response.headers()?;
        let wasi_res = OutgoingResponse::new(headers);

        wasi_res
            .set_status_code(response.0.status().as_u16())
            .expect("invalid http status code was returned");
        let body = wasi_res.body().expect("unable to take response body");
        ResponseOutparam::set(self.res_out, Ok(wasi_res));

        let output_stream = body
            .write()
            .expect("unable to open writable stream on body");
        let mut input_stream = match response.0.into_body() {
            Body::Sync(buf) => Box::pin(stream::once(async { Ok(buf) })),
            Body::Async(stream) => stream,
        };

        while let Some(buf) = input_stream.next().await {
            let buf = buf.map_err(HandlerError::ResponseStream)?;
            let chunks = buf.chunks(CHUNK_BYTE_SIZE);
            for chunk in chunks {
                output_stream
                    .blocking_write_and_flush(chunk)
                    .map_err(HandlerError::from)?;
            }
        }

        drop(output_stream);
        OutgoingBody::finish(body, None)
            .map_err(HandlerError::WasiResponseBody)?;

        Ok(())
    }
}

fn provide_contexts(
    additional_context: impl Fn() + 'static + Clone + Send,
    context_parts: Parts,
    res_opts: ResponseOptions,
) {
    additional_context();
    provide_context(RequestUrl::new(context_parts.uri.path()));
    provide_context(context_parts);
    provide_context(res_opts);
    provide_server_redirect(redirect);
}

trait RouterPathRepresentation {
    fn to_rf_str_representation(&self) -> String;
}

impl RouterPathRepresentation for &[PathSegment] {
    fn to_rf_str_representation(&self) -> String {
        let mut path = String::new();
        for segment in self.iter() {
            // TODO trailing slash handling
            let raw = segment.as_raw_str();
            if !raw.is_empty() && !raw.starts_with('/') {
                path.push('/');
            }
            match segment {
                PathSegment::Static(s) => path.push_str(s),
                PathSegment::Param(s) => {
                    path.push(':');
                    path.push_str(s);
                }
                PathSegment::Splat(_) => {
                    path.push('*');
                }
                PathSegment::Unit => {}
            }
        }
        path
    }
}

#[derive(Error, Debug)]
pub enum HandlerError {
    #[error("error handling request")]
    Request(#[from] crate::request::RequestError),

    #[error("error handling response")]
    Response(#[from] crate::response::ResponseError),

    #[error("response stream emitted an error")]
    ResponseStream(throw_error::Error),

    #[error("wasi stream failure")]
    WasiStream(#[from] wasi::io::streams::StreamError),

    #[error("failed to finish response body")]
    WasiResponseBody(wasi::http::types::ErrorCode),
}
