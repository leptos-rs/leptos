use crate::{
    error::{FromServerFnError, IntoAppError, ServerFnErrorErr},
    request::Req,
};
use axum::{
    body::{Body, Bytes},
    extract::{
        ws::{CloseFrame, Message, Utf8Bytes},
        FromRequest,
    },
    response::Response,
};
use futures::{FutureExt, SinkExt, Stream, StreamExt};
use http::{
    header::{ACCEPT, CONTENT_TYPE, REFERER},
    Request,
};
use http_body_util::BodyExt;
use std::borrow::Cow;

impl<E> Req<E> for Request<Body>
where
    E: FromServerFnError + Send,
{
    type WebsocketResponse = Response;

    fn as_query(&self) -> Option<&str> {
        self.uri().query()
    }

    fn to_content_type(&self) -> Option<Cow<'_, str>> {
        self.headers()
            .get(CONTENT_TYPE)
            .map(|h| String::from_utf8_lossy(h.as_bytes()))
    }

    fn accepts(&self) -> Option<Cow<'_, str>> {
        self.headers()
            .get(ACCEPT)
            .map(|h| String::from_utf8_lossy(h.as_bytes()))
    }

    fn referer(&self) -> Option<Cow<'_, str>> {
        self.headers()
            .get(REFERER)
            .map(|h| String::from_utf8_lossy(h.as_bytes()))
    }

    async fn try_into_bytes(self) -> Result<Bytes, E> {
        let (_parts, body) = self.into_parts();

        body.collect().await.map(|c| c.to_bytes()).map_err(|e| {
            ServerFnErrorErr::Deserialization(e.to_string()).into_app_error()
        })
    }

    async fn try_into_string(self) -> Result<String, E> {
        let bytes = self.try_into_bytes().await?;
        String::from_utf8(bytes.to_vec()).map_err(|e| {
            ServerFnErrorErr::Deserialization(e.to_string()).into_app_error()
        })
    }

    fn try_into_stream(
        self,
    ) -> Result<impl Stream<Item = Result<Bytes, E>> + Send + 'static, E> {
        Ok(self.into_body().into_data_stream().map(|chunk| {
            chunk.map_err(|e| {
                ServerFnErrorErr::Deserialization(e.to_string())
                    .into_app_error()
            })
        }))
    }

    fn try_into_websocket(
        self,
    ) -> Result<
        (
            impl Stream<Item = Result<Bytes, E>> + Send + 'static,
            impl futures::Sink<Result<Bytes, E>> + Send + 'static,
            Self::WebsocketResponse,
        ),
        E,
    > {
        let upgrade =
            axum::extract::ws::WebSocketUpgrade::from_request(self, &())
                .now_or_never()
                .unwrap()
                .unwrap();
        let (mut outgoing_tx, outgoing_rx) =
            futures::channel::mpsc::channel(2048);
        let (incoming_tx, mut incoming_rx) =
            futures::channel::mpsc::channel::<Result<Bytes, E>>(2048);
        let response = upgrade
            .on_failed_upgrade({
                let mut outgoing_tx = outgoing_tx.clone();
                move |err: axum::Error| {
                    _ = outgoing_tx.start_send(Err(E::from_server_fn_error(ServerFnErrorErr::Response(err.to_string()))));
                }
            })
            .on_upgrade(|mut msg| async move {
                loop {
                    futures::select! {
                        incoming = incoming_rx.next() => {
                            let Some(incoming) = incoming else {
                                return;
                            };
                            match incoming {
                                Ok(message) => {
                                    if let Err(err) = msg.send(Message::Binary(message)).await {
                                        _ = outgoing_tx.start_send(Err(E::from_server_fn_error(ServerFnErrorErr::Request(err.to_string()))));
                                    }
                                }
                                Err(err) => {
                                    if let Err(err) = msg.send(Message::Close(Some(CloseFrame {
                                        code: axum::extract::ws::close_code::ERROR,
                                        reason: Utf8Bytes::from(err.ser()),
                                    }))).await {
                                        _ = outgoing_tx.start_send(Err(E::from_server_fn_error(ServerFnErrorErr::Request(err.to_string()))));
                                    }
                                    if let Err(err) =  msg.close().await {
                                        _ = outgoing_tx.start_send(Err(E::from_server_fn_error(ServerFnErrorErr::Request(err.to_string()))));
                                    }
                                    return;
                                }
                            }
                        },
                        outgoing = msg.recv().fuse() => {
                            let Some(outgoing) = outgoing else {
                                return;
                            };
                            match outgoing {
                                Ok(Message::Binary(bytes)) => {
                                    _ = outgoing_tx
                                        .start_send(
                                            Ok(Bytes::from(bytes)),
                                        );
                                }
                                Ok(Message::Text(text)) => {
                                    _ = outgoing_tx.start_send(Ok(Bytes::from(text)));
                                }
                                Ok(_) => {}
                                Err(e) => {
                                    _ = outgoing_tx.start_send(Err(E::from_server_fn_error(ServerFnErrorErr::Response(e.to_string()))));
                                }
                            }
                        }
                    }
                }
            });

        Ok((outgoing_rx, incoming_tx, response))
    }
}
