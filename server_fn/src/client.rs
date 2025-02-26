use crate::{request::ClientReq, response::ClientRes};
use bytes::Bytes;
use futures::{Sink, Stream};
use std::{future::Future, sync::OnceLock};

static ROOT_URL: OnceLock<&'static str> = OnceLock::new();

/// Set the root server URL that all server function paths are relative to for the client.
///
/// If this is not set, it defaults to the origin.
pub fn set_server_url(url: &'static str) {
    ROOT_URL.set(url).unwrap();
}

/// Returns the root server URL for all server functions.
pub fn get_server_url() -> &'static str {
    ROOT_URL.get().copied().unwrap_or("")
}

/// A client defines a pair of request/response types and the logic to send
/// and receive them.
///
/// This trait is implemented for things like a browser `fetch` request or for
/// the `reqwest` trait. It should almost never be necessary to implement it
/// yourself, unless you’re trying to use an alternative HTTP crate on the client side.
pub trait Client<E> {
    /// The type of a request sent by this client.
    type Request: ClientReq<E> + Send + 'static;
    /// The type of a response received by this client.
    type Response: ClientRes<E> + Send + 'static;

    /// Sends the request and receives a response.
    fn send(
        req: Self::Request,
    ) -> impl Future<Output = Result<Self::Response, E>> + Send;

    /// Opens a websocket connection to the server.
    fn open_websocket(
        path: &str,
    ) -> impl Future<
        Output = Result<
            (
                impl Stream<Item = Result<Bytes, E>> + Send + 'static,
                impl Sink<Result<Bytes, E>> + Send + 'static,
            ),
            E,
        >,
    > + Send;
}

#[cfg(feature = "browser")]
/// Implements [`Client`] for a `fetch` request in the browser.
pub mod browser {
    use super::Client;
    use crate::{
        error::{FromServerFnError, IntoAppError, ServerFnErrorErr},
        request::browser::{BrowserRequest, RequestInner},
        response::browser::BrowserResponse,
    };
    use bytes::Bytes;
    use futures::{Sink, SinkExt, StreamExt, TryStreamExt};
    use gloo_net::websocket::{events::CloseEvent, Message, WebSocketError};
    use send_wrapper::SendWrapper;
    use std::{future::Future, pin::Pin};

    /// Implements [`Client`] for a `fetch` request in the browser.
    pub struct BrowserClient;

    impl<E: FromServerFnError> Client<E> for BrowserClient {
        type Request = BrowserRequest;
        type Response = BrowserResponse;

        fn send(
            req: Self::Request,
        ) -> impl Future<Output = Result<Self::Response, E>> + Send {
            SendWrapper::new(async move {
                let req = req.0.take();
                let RequestInner {
                    request,
                    mut abort_ctrl,
                } = req;
                let res = request
                    .send()
                    .await
                    .map(|res| BrowserResponse(SendWrapper::new(res)))
                    .map_err(|e| {
                        ServerFnErrorErr::Request(e.to_string())
                            .into_app_error()
                    });

                // at this point, the future has successfully resolved without being dropped, so we
                // can prevent the `AbortController` from firing
                if let Some(ctrl) = abort_ctrl.as_mut() {
                    ctrl.prevent_cancellation();
                }
                res
            })
        }

        fn open_websocket(
            url: &str,
        ) -> impl Future<
            Output = Result<
                (
                    impl futures::Stream<Item = Result<Bytes, E>> + Send + 'static,
                    impl futures::Sink<Result<Bytes, E>> + Send + 'static,
                ),
                E,
            >,
        > + Send {
            SendWrapper::new(async move {
                let websocket =
                    gloo_net::websocket::futures::WebSocket::open(url)
                        .map_err(|err| {
                            E::from_server_fn_error(ServerFnErrorErr::Request(
                                err.to_string(),
                            ))
                        })?;
                let (sink, stream) = websocket.split();

                let stream = stream
                    .map_err(|err| {
                        E::from_server_fn_error(ServerFnErrorErr::Request(
                            err.to_string(),
                        ))
                    })
                    .map_ok(move |msg| match msg {
                        Message::Text(text) => Bytes::from(text),
                        Message::Bytes(bytes) => Bytes::from(bytes),
                    });
                let stream = SendWrapper::new(stream);

                struct SendWrapperSink<S> {
                    // NOTE: We can't use pin project here because the `SendWrapper` doesn't export
                    // that invariant. It could change in a minor version.
                    sink: Pin<Box<SendWrapper<Pin<Box<S>>>>>,
                }

                impl<S> SendWrapperSink<S> {
                    fn new(sink: S) -> Self {
                        Self {
                            sink: Box::pin(SendWrapper::new(Box::pin(sink))),
                        }
                    }
                }

                impl<S, Item> Sink<Item> for SendWrapperSink<S>
                where
                    S: Sink<Item>,
                {
                    type Error = S::Error;

                    fn poll_ready(
                        self: std::pin::Pin<&mut Self>,
                        cx: &mut std::task::Context<'_>,
                    ) -> std::task::Poll<Result<(), Self::Error>>
                    {
                        self.get_mut().sink.poll_ready_unpin(cx)
                    }

                    fn start_send(
                        self: std::pin::Pin<&mut Self>,
                        item: Item,
                    ) -> Result<(), Self::Error> {
                        self.get_mut().sink.start_send_unpin(item)
                    }

                    fn poll_flush(
                        self: std::pin::Pin<&mut Self>,
                        cx: &mut std::task::Context<'_>,
                    ) -> std::task::Poll<Result<(), Self::Error>>
                    {
                        self.get_mut().sink.poll_flush_unpin(cx)
                    }

                    fn poll_close(
                        self: std::pin::Pin<&mut Self>,
                        cx: &mut std::task::Context<'_>,
                    ) -> std::task::Poll<Result<(), Self::Error>>
                    {
                        self.get_mut().sink.poll_close_unpin(cx)
                    }
                }

                let sink = sink.with(|message: Result<Bytes, E>| async move {
                    match message {
                        Ok(message) => Ok(Message::Bytes(message.into())),
                        Err(err) => {
                            const CLOSE_CODE_ERROR: u16 = 1011;
                            Err(WebSocketError::ConnectionClose(CloseEvent {
                                code: CLOSE_CODE_ERROR,
                                reason: err.ser(),
                                was_clean: true,
                            }))
                        }
                    }
                });
                let sink = SendWrapperSink::new(sink);

                Ok((stream, sink))
            })
        }
    }
}

#[cfg(feature = "reqwest")]
/// Implements [`Client`] for a request made by [`reqwest`].
pub mod reqwest {
    use super::Client;
    use crate::{
        error::{FromServerFnError, IntoAppError, ServerFnErrorErr},
        request::reqwest::CLIENT,
    };
    use futures::TryFutureExt;
    use reqwest::{Request, Response};
    use std::future::Future;

    /// Implements [`Client`] for a request made by [`reqwest`].
    pub struct ReqwestClient;

    impl<E: FromServerFnError> Client<E> for ReqwestClient {
        type Request = Request;
        type Response = Response;

        fn send(
            req: Self::Request,
        ) -> impl Future<Output = Result<Self::Response, E>> + Send {
            CLIENT.execute(req).map_err(|e| {
                ServerFnErrorErr::Request(e.to_string()).into_app_error()
            })
        }
    }
}
