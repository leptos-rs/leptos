use crate::{
    error::{FromServerFnError, IntoAppError, ServerFnErrorErr},
    request::Req,
    response::actix::ActixResponse,
};
use actix_web::{web::Payload, HttpRequest};
use actix_ws::Message;
use bytes::Bytes;
use futures::{FutureExt, Stream, StreamExt};
use send_wrapper::SendWrapper;
use std::{borrow::Cow, future::Future};

/// A wrapped Actix request.
///
/// This uses a [`SendWrapper`] that allows the Actix `HttpRequest` type to be `Send`, but panics
/// if it it is ever sent to another thread. Actix pins request handling to a single thread, so this
/// is necessary to be compatible with traits that require `Send` but should never panic in actual use.
pub struct ActixRequest(pub(crate) SendWrapper<(HttpRequest, Payload)>);

impl ActixRequest {
    /// Returns the raw Actix request, and its body.
    pub fn take(self) -> (HttpRequest, Payload) {
        self.0.take()
    }

    fn header(&self, name: &str) -> Option<Cow<'_, str>> {
        self.0
             .0
            .headers()
            .get(name)
            .map(|h| String::from_utf8_lossy(h.as_bytes()))
    }
}

impl From<(HttpRequest, Payload)> for ActixRequest {
    fn from(value: (HttpRequest, Payload)) -> Self {
        ActixRequest(SendWrapper::new(value))
    }
}

impl<E> Req<E> for ActixRequest
where
    E: FromServerFnError + Send,
{
    type WebsocketResponse = ActixResponse;

    fn as_query(&self) -> Option<&str> {
        self.0 .0.uri().query()
    }

    fn to_content_type(&self) -> Option<Cow<'_, str>> {
        self.header("Content-Type")
    }

    fn accepts(&self) -> Option<Cow<'_, str>> {
        self.header("Accept")
    }

    fn referer(&self) -> Option<Cow<'_, str>> {
        self.header("Referer")
    }

    fn try_into_bytes(self) -> impl Future<Output = Result<Bytes, E>> + Send {
        // Actix is going to keep this on a single thread anyway so it's fine to wrap it
        // with SendWrapper, which makes it `Send` but will panic if it moves to another thread
        SendWrapper::new(async move {
            let payload = self.0.take().1;
            payload.to_bytes().await.map_err(|e| {
                ServerFnErrorErr::Deserialization(e.to_string())
                    .into_app_error()
            })
        })
    }

    fn try_into_string(self) -> impl Future<Output = Result<String, E>> + Send {
        // Actix is going to keep this on a single thread anyway so it's fine to wrap it
        // with SendWrapper, which makes it `Send` but will panic if it moves to another thread
        SendWrapper::new(async move {
            let payload = self.0.take().1;
            let bytes = payload.to_bytes().await.map_err(|e| {
                E::from_server_fn_error(ServerFnErrorErr::Deserialization(
                    e.to_string(),
                ))
            })?;
            String::from_utf8(bytes.into()).map_err(|e| {
                E::from_server_fn_error(ServerFnErrorErr::Deserialization(
                    e.to_string(),
                ))
            })
        })
    }

    fn try_into_stream(
        self,
    ) -> Result<impl Stream<Item = Result<Bytes, E>> + Send, E> {
        let payload = self.0.take().1;
        let stream = payload.map(|res| {
            res.map_err(|e| {
                ServerFnErrorErr::Deserialization(e.to_string())
                    .into_app_error()
            })
        });
        Ok(SendWrapper::new(stream))
    }

    async fn try_into_websocket(
        self,
    ) -> Result<
        (
            impl Stream<Item = Result<Bytes, E>> + Send + 'static,
            impl futures::Sink<Result<Bytes, E>> + Send + 'static,
            Self::WebsocketResponse,
        ),
        E,
    > {
        let (request, payload) = self.0.take();
        let (response, mut session, mut msg_stream) =
            actix_ws::handle(&request, payload).map_err(|e| {
                E::from_server_fn_error(ServerFnErrorErr::Request(
                    e.to_string(),
                ))
            })?;

        let (mut response_stream_tx, response_stream_rx) =
            futures::channel::mpsc::channel(2048);
        let (response_sink_tx, mut response_sink_rx) =
            futures::channel::mpsc::channel(2048);

        actix_web::rt::spawn(async move {
            loop {
                futures::select! {
                    incoming = response_sink_rx.next() => {
                        let Some(incoming) = incoming else {
                            break;
                        };
                        match incoming {
                            Ok(message) => {
                                if let Err(err) = session.binary(message).await {
                                    _ = response_stream_tx.start_send(Err(E::from_server_fn_error(ServerFnErrorErr::Request(err.to_string()))));
                                }
                            }
                            Err(err) => {
                                _ = response_stream_tx.start_send(Err(err));
                            }
                        }
                    },
                    outgoing = msg_stream.next().fuse() => {
                        let Some(outgoing) = outgoing else {
                            break;
                        };
                        match outgoing {
                            Ok(Message::Ping(bytes)) => {
                                if session.pong(&bytes).await.is_err() {
                                    break;
                                }
                            }
                            Ok(Message::Binary(bytes)) => {
                                _ = response_stream_tx
                                    .start_send(
                                        Ok(bytes),
                                    );
                            }
                            Ok(Message::Text(text)) => {
                                _ = response_stream_tx.start_send(Ok(text.into_bytes()));
                            }
                            Ok(_other) => {
                            }
                            Err(e) => {
                                _ = response_stream_tx.start_send(Err(E::from_server_fn_error(ServerFnErrorErr::Response(e.to_string()))));
                            }
                        }
                    }
                }
            }
            let _ = session.close(None).await;
        });

        Ok((
            response_stream_rx,
            response_sink_tx,
            ActixResponse::from(response),
        ))
    }
}
