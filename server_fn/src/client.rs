use crate::{error::ServerFnError, request::ClientReq, response::ClientRes};
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
pub trait Client<CustErr> {
    /// The type of a request sent by this client.
    type Request: ClientReq<CustErr> + Send;
    /// The type of a response received by this client.
    type Response: ClientRes<CustErr> + Send;

    /// Sends the request and receives a response.
    fn send(
        req: Self::Request,
    ) -> impl Future<Output = Result<Self::Response, ServerFnError<CustErr>>> + Send;
}

#[cfg(feature = "browser")]
/// Implements [`Client`] for a `fetch` request in the browser.
pub mod browser {
    use super::Client;
    use crate::{
        error::ServerFnError,
        request::browser::{AbortOnDrop, BrowserRequest, RequestInner},
        response::browser::BrowserResponse,
    };
    use gloo_net::{http::Response, Error};
    use send_wrapper::SendWrapper;
    use std::{
        future::Future,
        marker::PhantomData,
        pin::Pin,
        task::{Context, Poll},
    };
    use web_sys::AbortController;

    /// Implements [`Client`] for a `fetch` request in the browser.    
    pub struct BrowserClient;

    impl<CustErr> Client<CustErr> for BrowserClient {
        type Request = BrowserRequest;
        type Response = BrowserResponse;

        fn send(
            req: Self::Request,
        ) -> impl Future<Output = Result<Self::Response, ServerFnError<CustErr>>>
               + Send {
            SendWrapper::new(async move {
                let mut req = req.0.take();
                let RequestInner {
                    request,
                    abort_ctrl,
                } = req;
                /*BrowserRequestFuture {
                    request_fut: request.send(),
                    abort_ctrl,
                    cust_err: PhantomData,
                }*/
                request
                    .send()
                    .await
                    .map(|res| BrowserResponse(SendWrapper::new(res)))
                    .map_err(|e| ServerFnError::Request(e.to_string()))
            })
        }
    }

    /*pin_project_lite::pin_project! {
        struct BrowserRequestFuture<Fut, CustErr>
        where
            Fut: Future<Output = Result<Response, Error>>,
        {
            #[pin]
            request_fut: Fut,
            abort_ctrl: Option<AbortOnDrop>,
            cust_err: PhantomData<CustErr>
        }
    }

    impl<Fut, CustErr> Future for BrowserRequestFuture<Fut, CustErr>
    where
        Fut: Future<Output = Result<Response, Error>>,
    {
        type Output = Result<BrowserResponse, ServerFnError<CustErr>>;

        fn poll(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
        ) -> Poll<Self::Output> {
            let this = self.project();
            match this.request_fut.poll(cx) {
                Poll::Ready(value) => todo!(),
                Poll::Pending => Poll::Pending,
            }
        }
    }*/
}

#[cfg(feature = "reqwest")]
/// Implements [`Client`] for a request made by [`reqwest`].
pub mod reqwest {
    use super::Client;
    use crate::{error::ServerFnError, request::reqwest::CLIENT};
    use futures::TryFutureExt;
    use reqwest::{Request, Response};
    use std::future::Future;

    /// Implements [`Client`] for a request made by [`reqwest`].
    pub struct ReqwestClient;

    impl<CustErr> Client<CustErr> for ReqwestClient {
        type Request = Request;
        type Response = Response;

        fn send(
            req: Self::Request,
        ) -> impl Future<Output = Result<Self::Response, ServerFnError<CustErr>>>
               + Send {
            CLIENT
                .execute(req)
                .map_err(|e| ServerFnError::Request(e.to_string()))
        }
    }
}
