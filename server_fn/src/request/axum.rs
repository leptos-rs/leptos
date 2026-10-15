use crate::{
    error::{FromServerFnError, IntoAppError, ServerFnErrorErr},
    request::Req,
};
use axum::{
    RequestExt,
    body::{Body, Bytes},
    response::Response,
};
#[cfg(feature = "axum")]
use futures::SinkExt;
use futures::{Sink, Stream, StreamExt};
use http::{
    Request, Uri,
    header::{ACCEPT, CONTENT_TYPE, REFERER},
};
use http_body_util::{BodyExt, LengthLimitError};
use std::borrow::Cow;

const BODY_LIMIT_ERROR: &str =
    "request body exceeds the size limit set by axum's `DefaultBodyLimit` (2 \
     MiB by default; raise it with `DefaultBodyLimit::max(..)`) or by another \
     body limit layer, such as tower-http's `RequestBodyLimitLayer`";

fn body_error<E: FromServerFnError>(err: axum::Error, uri: &Uri) -> E {
    // The `LengthLimitError` of a `Limited` body only says "length limit
    // exceeded", which does not tell users which limit to raise. Each limit
    // layer (axum's default limit, tower-http's `RequestBodyLimitLayer`)
    // wraps the error again, so search the whole chain.
    let is_limit_error = std::iter::successors(
        Some(&err as &(dyn std::error::Error + 'static)),
        |err| err.source(),
    )
    .any(|err| err.is::<LengthLimitError>());
    let message = if is_limit_error {
        tracing::warn!(
            target: "server_fn::body_limit",
            path = %uri.path(),
            "{BODY_LIMIT_ERROR}"
        );
        BODY_LIMIT_ERROR.to_string()
    } else {
        err.to_string()
    };
    ServerFnErrorErr::Deserialization(message).into_app_error()
}

impl<Error, InputStreamError, OutputStreamError>
    Req<Error, InputStreamError, OutputStreamError> for Request<Body>
where
    Error: FromServerFnError + Send,
    InputStreamError: FromServerFnError + Send,
    OutputStreamError: FromServerFnError + Send,
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

    async fn try_into_bytes(self) -> Result<Bytes, Error> {
        let uri = self.uri().clone();
        let body = self.into_limited_body();
        body.collect()
            .await
            .map(|c| c.to_bytes())
            .map_err(|e| body_error(e, &uri))
    }

    async fn try_into_string(self) -> Result<String, Error> {
        let bytes = Req::<Error>::try_into_bytes(self).await?;
        String::from_utf8(bytes.into()).map_err(|e| {
            ServerFnErrorErr::Deserialization(e.to_string()).into_app_error()
        })
    }

    fn try_into_stream(
        self,
    ) -> Result<impl Stream<Item = Result<Bytes, Bytes>> + Send + 'static, Error>
    {
        let uri = self.uri().clone();
        let body = self.into_limited_body();
        // End the stream after the first error: `Limited` fails every later
        // frame too, so a consumer that skips errors would read the whole
        // upload.
        Ok(body.into_data_stream().scan(false, move |failed, chunk| {
            let item = (!*failed).then(|| {
                chunk.map_err(|e| {
                    *failed = true;
                    body_error::<Error>(e, &uri).ser().body
                })
            });
            futures::future::ready(item)
        }))
    }

    async fn try_into_websocket(
        self,
    ) -> Result<
        (
            impl Stream<Item = Result<Bytes, Bytes>> + Send + 'static,
            impl Sink<Bytes> + Send + 'static,
            impl std::future::Future<Output = ()> + Send + 'static,
            Self::WebsocketResponse,
        ),
        Error,
    > {
        #[cfg(not(feature = "axum"))]
        {
            Err::<
                (
                    futures::stream::Once<
                        std::future::Ready<Result<Bytes, Bytes>>,
                    >,
                    futures::sink::Drain<Bytes>,
                    std::future::Pending<()>,
                    Self::WebsocketResponse,
                ),
                Error,
            >(Error::from_server_fn_error(
                crate::ServerFnErrorErr::Response(
                    "Websocket connections not supported for Axum when the \
                     `axum` feature is not enabled on the `server_fn` crate."
                        .to_string(),
                ),
            ))
        }
        #[cfg(feature = "axum")]
        {
            use axum::extract::{FromRequest, ws::Message};
            use futures::FutureExt;

            let upgrade =
                axum::extract::ws::WebSocketUpgrade::from_request(self, &())
                    .await
                    .map_err(|err| {
                        Error::from_server_fn_error(ServerFnErrorErr::Request(
                            err.to_string(),
                        ))
                    })?;
            let (mut outgoing_tx, outgoing_rx) =
                futures::channel::mpsc::channel::<Result<Bytes, Bytes>>(2048);
            let (incoming_tx, mut incoming_rx) =
                futures::channel::mpsc::channel::<Bytes>(2048);
            let (closed_tx, closed_rx) =
                futures::channel::oneshot::channel::<()>();
            let response = upgrade
        .on_failed_upgrade({
            let mut outgoing_tx = outgoing_tx.clone();
            move |err: axum::Error| {
                _ = outgoing_tx.start_send(Err(InputStreamError::from_server_fn_error(ServerFnErrorErr::Response(err.to_string())).ser().body));
            }
        })
        .on_upgrade(move |mut session| async move {
            loop {
                futures::select! {
                    incoming = incoming_rx.next() => {
                        let Some(incoming) = incoming else {
                            break;
                        };
                        if let Err(err) = session.send(Message::Binary(incoming)).await
                            && outgoing_tx.send(Err(InputStreamError::from_server_fn_error(ServerFnErrorErr::Request(err.to_string())).ser().body)).await.is_err() {
                                break;
                            }
                    },
                        outgoing = session.recv().fuse() => {
                        let Some(outgoing) = outgoing else {
                            break;
                        };
                        match outgoing {
                            Ok(Message::Binary(bytes)) => {
                                if outgoing_tx.send(Ok(bytes)).await.is_err() {
                                    break;
                                }
                            }
                            Ok(Message::Text(text)) => {
                                if outgoing_tx.send(Ok(Bytes::from(text))).await.is_err() {
                                    break;
                                }
                            }
                            Ok(Message::Ping(bytes)) => {
                                if session.send(Message::Pong(bytes)).await.is_err() {
                                    break;
                                }
                            }
                            Ok(_other) => {}
                            Err(e) => {
                                if outgoing_tx.send(Err(InputStreamError::from_server_fn_error(ServerFnErrorErr::Response(e.to_string())).ser().body)).await.is_err() {
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            _ = session.send(Message::Close(None)).await;
            drop(closed_tx);
        });

            Ok((outgoing_rx, incoming_tx, closed_rx.map(|_| ()), response))
        }
    }
}

#[cfg(all(test, feature = "axum"))]
mod tests {
    use super::*;
    use crate::ServerFnError;
    use axum::extract::DefaultBodyLimit;

    async fn drain(req: Request<Body>) -> (usize, Option<ServerFnError>) {
        let stream =
            <Request<Body> as Req<ServerFnError>>::try_into_stream(req)
                .expect("request should become a stream");
        futures::pin_mut!(stream);
        let mut total = 0;
        let mut error = None;
        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(bytes) => total += bytes.len(),
                Err(bytes) => error = Some(ServerFnError::de(bytes)),
            }
        }
        (total, error)
    }

    fn assert_body_limit_error(error: Option<ServerFnError>) {
        assert!(
            matches!(
                &error,
                Some(ServerFnError::Deserialization(msg))
                    if msg.contains("DefaultBodyLimit")
            ),
            "expected a body limit error, got {error:?}"
        );
    }

    #[tokio::test]
    async fn stream_enforces_configured_body_limit() {
        let mut req = Request::new(Body::from(vec![b'x'; 100_000]));
        DefaultBodyLimit::max(1024).apply(&mut req);
        let (total, error) = drain(req).await;
        assert_body_limit_error(error);
        assert!(total <= 1024, "received {total} bytes");
    }

    #[tokio::test]
    async fn stream_enforces_default_body_limit() {
        let req = Request::new(Body::from(vec![b'x'; 2 * 1024 * 1024 + 1]));
        let (_, error) = drain(req).await;
        assert_body_limit_error(error);
    }

    #[tokio::test]
    async fn stream_detects_nested_body_limit() {
        // A limit layer such as tower-http's `RequestBodyLimitLayer` wraps the
        // body in `Limited` before axum's default limit wraps it again.
        let body = Body::new(http_body_util::Limited::new(
            Body::from(vec![b'x'; 100_000]),
            1024,
        ));
        let (_, error) = drain(Request::new(body)).await;
        assert_body_limit_error(error);
    }

    #[tokio::test]
    async fn stream_ends_after_first_error() {
        let chunks = (0..100).map(|_| {
            Ok::<_, std::convert::Infallible>(Bytes::from(vec![b'x'; 1000]))
        });
        let mut req =
            Request::new(Body::from_stream(futures::stream::iter(chunks)));
        DefaultBodyLimit::max(1024).apply(&mut req);
        let items: Vec<_> =
            <Request<Body> as Req<ServerFnError>>::try_into_stream(req)
                .expect("request should become a stream")
                .collect()
                .await;
        // One chunk fits in the limit; the next one fails and ends the stream.
        assert_eq!(items.len(), 2, "{items:?}");
        assert!(items[0].is_ok() && items[1].is_err(), "{items:?}");
    }

    #[tokio::test]
    async fn stream_respects_disabled_body_limit() {
        let mut req = Request::new(Body::from(vec![b'x'; 100_000]));
        DefaultBodyLimit::disable().apply(&mut req);
        let (total, error) = drain(req).await;
        assert!(error.is_none(), "disabled limit returned {error:?}");
        assert_eq!(total, 100_000);
    }

    #[tokio::test]
    async fn stream_keeps_other_body_errors() {
        let body = Body::from_stream(futures::stream::iter([Err::<Bytes, _>(
            std::io::Error::other("connection reset"),
        )]));
        let (_, error) = drain(Request::new(body)).await;
        assert_eq!(
            error,
            Some(ServerFnError::Deserialization("connection reset".into()))
        );
    }

    #[tokio::test]
    async fn bytes_report_configured_body_limit() {
        let mut req = Request::new(Body::from(vec![b'x'; 100_000]));
        DefaultBodyLimit::max(1024).apply(&mut req);
        let result =
            <Request<Body> as Req<ServerFnError>>::try_into_bytes(req).await;
        assert_body_limit_error(result.err());
    }

    #[tokio::test]
    async fn bytes_report_default_body_limit() {
        let req = Request::new(Body::from(vec![b'x'; 2 * 1024 * 1024 + 1]));
        let result =
            <Request<Body> as Req<ServerFnError>>::try_into_bytes(req).await;
        assert_body_limit_error(result.err());
    }
}
