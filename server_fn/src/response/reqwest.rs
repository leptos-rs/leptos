use super::ClientRes;
use crate::error::{FromServerFnError, IntoAppError, ServerFnErrorErr};
use bytes::Bytes;
use futures::{Stream, TryStreamExt};
pub use reqwest::Response;
#[cfg(target_arch = "wasm32")]
use send_wrapper::SendWrapper;

#[cfg(target_arch = "wasm32")]
/// A wrapper for `reqwest::Response` that implements `Send` on WASM targets.
pub struct WasmResponse(SendWrapper<Response>);

#[cfg(target_arch = "wasm32")]
impl From<Response> for WasmResponse {
    fn from(response: Response) -> Self {
        Self(SendWrapper::new(response))
    }
}

#[cfg(target_arch = "wasm32")]
impl<E: FromServerFnError> ClientRes<E> for WasmResponse {
    fn try_into_string(
        self,
    ) -> impl std::future::Future<Output = Result<String, E>> + Send {
        SendWrapper::new(async move {
            let response = self.0.take();
            response.text().await.map_err(|e| {
                ServerFnErrorErr::Deserialization(e.to_string())
                    .into_app_error()
            })
        })
    }

    fn try_into_bytes(
        self,
    ) -> impl std::future::Future<Output = Result<Bytes, E>> + Send {
        SendWrapper::new(async move {
            let response = self.0.take();
            response.bytes().await.map_err(|e| {
                ServerFnErrorErr::Deserialization(e.to_string())
                    .into_app_error()
            })
        })
    }

    fn try_into_stream(
        self,
    ) -> Result<impl Stream<Item = Result<Bytes, Bytes>> + Send + 'static, E>
    {
        let response = self.0.take();
        Ok(SendWrapper::new(response.bytes_stream().map_err(|e| {
            E::from_server_fn_error(ServerFnErrorErr::Response(e.to_string()))
                .ser()
        })))
    }

    fn status(&self) -> u16 {
        self.0.status().as_u16()
    }

    fn status_text(&self) -> String {
        self.0.status().to_string()
    }

    fn location(&self) -> String {
        self.0
            .headers()
            .get("Location")
            .map(|value| String::from_utf8_lossy(value.as_bytes()).to_string())
            .unwrap_or_else(|| self.0.url().to_string())
    }

    fn has_redirect(&self) -> bool {
        self.0.headers().get("Location").is_some()
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl<E: FromServerFnError> ClientRes<E> for Response {
    async fn try_into_string(self) -> Result<String, E> {
        self.text().await.map_err(|e| {
            ServerFnErrorErr::Deserialization(e.to_string()).into_app_error()
        })
    }

    async fn try_into_bytes(self) -> Result<Bytes, E> {
        self.bytes().await.map_err(|e| {
            ServerFnErrorErr::Deserialization(e.to_string()).into_app_error()
        })
    }

    fn try_into_stream(
        self,
    ) -> Result<impl Stream<Item = Result<Bytes, Bytes>> + Send + 'static, E>
    {
        Ok(self.bytes_stream().map_err(|e| {
            E::from_server_fn_error(ServerFnErrorErr::Response(e.to_string()))
                .ser()
        }))
    }

    fn status(&self) -> u16 {
        self.status().as_u16()
    }

    fn status_text(&self) -> String {
        self.status().to_string()
    }

    fn location(&self) -> String {
        self.headers()
            .get("Location")
            .map(|value| String::from_utf8_lossy(value.as_bytes()).to_string())
            .unwrap_or_else(|| self.url().to_string())
    }

    fn has_redirect(&self) -> bool {
        self.headers().get("Location").is_some()
    }
}
