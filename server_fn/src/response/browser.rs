use super::ClientRes;
use crate::{
    error::{FromServerFnError, IntoAppError, ServerFnErrorErr},
    redirect::REDIRECT_HEADER,
    request::browser::AbortOnDrop,
};
use bytes::Bytes;
use futures::{Stream, StreamExt};
pub use gloo_net::http::Response;
use http::{HeaderMap, HeaderName, HeaderValue};
use js_sys::Uint8Array;
use send_wrapper::SendWrapper;
use std::{future::Future, str::FromStr};
use wasm_bindgen::JsCast;
use wasm_streams::ReadableStream;

/// The response to a `fetch` request made in the browser.
pub struct BrowserResponse(
    pub(crate) SendWrapper<Response>,
    pub(crate) SendWrapper<Option<AbortOnDrop>>,
);

impl BrowserResponse {
    fn prevent_cancellation(&mut self) {
        if let Some(ctrl) = self.1.as_mut() {
            ctrl.prevent_cancellation();
        }
    }

    /// Generate the headers from the internal [`Response`] object.
    /// This is a workaround for the fact that the `Response` object does not
    /// have a [`HeaderMap`] directly. This function will iterate over the
    /// headers and convert them to a [`HeaderMap`].
    pub fn generate_headers(&self) -> HeaderMap {
        self.0
            .headers()
            .entries()
            .filter_map(|(key, value)| {
                let key = HeaderName::from_str(&key).ok()?;
                let value = HeaderValue::from_str(&value).ok()?;
                Some((key, value))
            })
            .collect()
    }
}

impl<E: FromServerFnError> ClientRes<E> for BrowserResponse {
    fn try_into_string(
        mut self,
    ) -> impl Future<Output = Result<String, E>> + Send {
        // the browser won't send this async work between threads (because it's single-threaded)
        // so we can safely wrap this
        SendWrapper::new(async move {
            let result = self.0.text().await;
            self.prevent_cancellation();
            result.map_err(|e| {
                ServerFnErrorErr::Deserialization(e.to_string())
                    .into_app_error()
            })
        })
    }

    fn try_into_bytes(
        mut self,
    ) -> impl Future<Output = Result<Bytes, E>> + Send {
        // the browser won't send this async work between threads (because it's single-threaded)
        // so we can safely wrap this
        SendWrapper::new(async move {
            let result = self.0.binary().await;
            self.prevent_cancellation();
            result.map(Bytes::from).map_err(|e| {
                ServerFnErrorErr::Deserialization(e.to_string())
                    .into_app_error()
            })
        })
    }

    fn try_into_stream(
        self,
    ) -> Result<impl Stream<Item = Result<Bytes, Bytes>> + Send + 'static, E>
    {
        let body = self.0.body().ok_or_else(|| {
            E::from_server_fn_error(ServerFnErrorErr::Response(
                "response has no body".into(),
            ))
        })?;
        let mut stream = ReadableStream::from_raw(body).into_stream().map(
            |data| match data {
                Err(e) => {
                    web_sys::console::error_1(&e);
                    Err(E::from_server_fn_error(ServerFnErrorErr::Request(
                        format!("{e:?}"),
                    ))
                    .ser()
                    .body)
                }
                Ok(data) => {
                    let data = data.unchecked_into::<Uint8Array>();
                    let mut buf = Vec::new();
                    let length = data.length();
                    buf.resize(length as usize, 0);
                    data.copy_to(&mut buf);
                    Ok(Bytes::from(buf))
                }
            },
        );
        let mut abort_ctrl = self.1.take();
        Ok(SendWrapper::new(futures::stream::poll_fn(move |cx| {
            let next = stream.poll_next_unpin(cx);
            if matches!(next, std::task::Poll::Ready(None))
                && let Some(ctrl) = abort_ctrl.as_mut()
            {
                ctrl.prevent_cancellation();
            }
            next
        })))
    }

    fn status(&self) -> u16 {
        self.0.status()
    }

    fn status_text(&self) -> String {
        self.0.status_text()
    }

    fn location(&self) -> String {
        self.0
            .headers()
            .get("Location")
            .unwrap_or_else(|| self.0.url())
    }

    fn has_redirect(&self) -> bool {
        self.0.headers().get(REDIRECT_HEADER).is_some()
    }
}

#[cfg(all(test, target_family = "wasm"))]
mod tests {
    use super::*;
    use crate::error::ServerFnError;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    fn browser_response(body: &str) -> BrowserResponse {
        let raw = js_sys::eval(&format!("new Response({body})"))
            .expect("create fetch Response")
            .dyn_into::<web_sys::Response>()
            .expect("cast fetch Response");
        BrowserResponse(
            SendWrapper::new(Response::from(raw)),
            SendWrapper::new(None),
        )
    }

    fn abortable_response(
        body: &str,
    ) -> (BrowserResponse, web_sys::AbortController) {
        let controller = web_sys::AbortController::new().unwrap();
        let mut response = browser_response(body);
        response.1 =
            SendWrapper::new(Some(AbortOnDrop(Some(controller.clone()))));
        (response, controller)
    }

    #[wasm_bindgen_test]
    fn dropping_unread_response_aborts() {
        let (response, controller) = abortable_response("'body'");
        assert!(!controller.signal().aborted());
        drop(response);
        assert!(controller.signal().aborted());
    }

    #[wasm_bindgen_test(async)]
    async fn dropping_pending_body_reads_aborts() {
        let (response, controller) =
            abortable_response("new ReadableStream({start(){}})");
        let mut read = Box::pin(<BrowserResponse as ClientRes<
            ServerFnError,
        >>::try_into_string(response));
        assert!(futures::poll!(read.as_mut()).is_pending());
        assert!(!controller.signal().aborted());
        drop(read);
        assert!(controller.signal().aborted());

        let (response, controller) =
            abortable_response("new ReadableStream({start(){}})");
        let mut read = Box::pin(<BrowserResponse as ClientRes<
            ServerFnError,
        >>::try_into_bytes(response));
        assert!(futures::poll!(read.as_mut()).is_pending());
        assert!(!controller.signal().aborted());
        drop(read);
        assert!(controller.signal().aborted());
    }

    #[wasm_bindgen_test(async)]
    async fn completed_body_reads_do_not_abort() {
        let (response, controller) = abortable_response("'body'");
        let body =
            <BrowserResponse as ClientRes<ServerFnError>>::try_into_string(
                response,
            )
            .await
            .unwrap();
        assert_eq!(body, "body");
        assert!(!controller.signal().aborted());

        let (response, controller) = abortable_response("'body'");
        let body =
            <BrowserResponse as ClientRes<ServerFnError>>::try_into_bytes(
                response,
            )
            .await
            .unwrap();
        assert_eq!(body.as_ref(), b"body");
        assert!(!controller.signal().aborted());
    }

    #[wasm_bindgen_test(async)]
    async fn failed_body_reads_do_not_abort() {
        let body = "new ReadableStream({start(c){c.error('failed')}})";
        let (response, controller) = abortable_response(body);
        assert!(
            <BrowserResponse as ClientRes<ServerFnError>>::try_into_string(
                response
            )
            .await
            .is_err()
        );
        assert!(!controller.signal().aborted());

        let (response, controller) = abortable_response(body);
        assert!(
            <BrowserResponse as ClientRes<ServerFnError>>::try_into_bytes(
                response
            )
            .await
            .is_err()
        );
        assert!(!controller.signal().aborted());
    }

    #[wasm_bindgen_test(async)]
    async fn dropping_pending_stream_aborts() {
        let (response, controller) =
            abortable_response("new ReadableStream({start(){}})");
        let mut stream = Box::pin(
            <BrowserResponse as ClientRes<ServerFnError>>::try_into_stream(
                response,
            )
            .unwrap(),
        );
        assert!(futures::poll!(stream.next()).is_pending());
        assert!(!controller.signal().aborted());
        drop(stream);
        assert!(controller.signal().aborted());
    }

    #[wasm_bindgen_test(async)]
    async fn dropping_finished_stream_does_not_abort() {
        let (response, controller) = abortable_response("'body'");
        let mut stream = Box::pin(
            <BrowserResponse as ClientRes<ServerFnError>>::try_into_stream(
                response,
            )
            .unwrap(),
        );
        while let Some(chunk) = stream.next().await {
            chunk.unwrap();
        }
        drop(stream);
        assert!(!controller.signal().aborted());
    }

    #[wasm_bindgen_test]
    fn bodyless_response_returns_error() {
        let response = browser_response("null");

        let result =
            <BrowserResponse as ClientRes<ServerFnError>>::try_into_stream(
                response,
            );

        assert!(result.is_err());
    }

    #[wasm_bindgen_test(async)]
    async fn response_with_body_returns_stream_bytes() {
        let response = browser_response("'stream body'");
        let stream =
            <BrowserResponse as ClientRes<ServerFnError>>::try_into_stream(
                response,
            )
            .expect("response should have a body");

        let chunks = stream.collect::<Vec<_>>().await;
        let body = chunks.into_iter().map(Result::unwrap).fold(
            Vec::new(),
            |mut body, chunk| {
                body.extend_from_slice(&chunk);
                body
            },
        );

        assert_eq!(body, b"stream body");
    }
}
