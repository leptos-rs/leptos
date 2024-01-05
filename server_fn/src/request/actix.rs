use crate::{error::ServerFnError, request::Req};
use actix_web::{web::Payload, HttpRequest};
use bytes::Bytes;
use futures::Stream;
use send_wrapper::SendWrapper;
use std::future::Future;

pub struct ActixRequest(pub(crate) SendWrapper<(HttpRequest, Payload)>);

impl ActixRequest {
    pub fn take(self) -> (HttpRequest, Payload) {
        self.0.take()
    }
}

impl From<(HttpRequest, Payload)> for ActixRequest {
    fn from(value: (HttpRequest, Payload)) -> Self {
        ActixRequest(SendWrapper::new(value))
    }
}

impl<CustErr> Req<CustErr> for ActixRequest {
    fn as_query(&self) -> Option<&str> {
        self.0 .0.uri().query()
    }

    fn to_content_type(&self) -> Option<String> {
        self.0
             .0
            .headers()
            .get("Content-Type")
            .map(|h| String::from_utf8_lossy(h.as_bytes()).to_string())
    }

    fn try_into_bytes(
        self,
    ) -> impl Future<Output = Result<Bytes, ServerFnError<CustErr>>> + Send
    {
        // Actix is going to keep this on a single thread anyway so it's fine to wrap it
        // with SendWrapper, which makes it `Send` but will panic if it moves to another thread
        SendWrapper::new(async move {
            let payload = self.0.take().1;
            payload
                .to_bytes()
                .await
                .map_err(|e| ServerFnError::Deserialization(e.to_string()))
        })
    }

    fn try_into_string(
        self,
    ) -> impl Future<Output = Result<String, ServerFnError<CustErr>>> + Send
    {
        // Actix is going to keep this on a single thread anyway so it's fine to wrap it
        // with SendWrapper, which makes it `Send` but will panic if it moves to another thread
        SendWrapper::new(async move {
            let payload = self.0.take().1;
            let bytes = payload
                .to_bytes()
                .await
                .map_err(|e| ServerFnError::Deserialization(e.to_string()))?;
            String::from_utf8(bytes.into())
                .map_err(|e| ServerFnError::Deserialization(e.to_string()))
        })
    }

    fn try_into_stream(
        self,
    ) -> Result<
        impl Stream<Item = Result<Bytes, ServerFnError>> + Send,
        ServerFnError<CustErr>,
    > {
        Ok(futures::stream::once(async { todo!() }))
    }
}
