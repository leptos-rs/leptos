use async_trait::async_trait;
use leptos::*;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::{Arc, RwLock};
use thiserror::Error;

#[cfg(feature = "ssr")]
lazy_static::lazy_static! {
    pub static ref REGISTERED_SERVER_FUNCTIONS: Arc<RwLock<HashMap<&'static str, Arc<dyn Fn(&[u8]) -> Pin<Box<dyn Future<Output = Result<String, ServerFnError>>>> + Send + Sync>>>> = Default::default();
}

#[async_trait]
pub trait ServerFn
where
    Self: Sized + DeserializeOwned + 'static,
{
    type Output: Serializable;

    fn url() -> &'static str;

    fn as_form_data(&self) -> Vec<(&'static str, String)>;

    #[cfg(feature = "ssr")]
    async fn call_fn(self) -> Result<Self::Output, ServerFnError>;

    #[cfg(feature = "ssr")]
    fn register() -> Result<(), ServerFnError> {
        // create the handler for this server function
        // takes a String -> returns its async value
        let run_server_fn = Arc::new(|data: &[u8]| {
            // decode the args
            let value = serde_urlencoded::from_bytes::<Self>(&data)
                .map_err(|e| ServerFnError::Args(e.to_string()));
            Box::pin(async move {
                let value = match value {
                    Ok(v) => v,
                    Err(e) => return Err(e),
                };

                // call the function
                let result = match value.call_fn().await {
                    Ok(r) => r,
                    Err(e) => return Err(e),
                };

                // serialize the output
                let result = match result
                    .to_json()
                    .map_err(|e| ServerFnError::Serialization(e.to_string()))
                {
                    Ok(r) => r,
                    Err(e) => return Err(e),
                };

                Ok(result)
            }) as Pin<Box<dyn Future<Output = Result<String, ServerFnError>>>>
        });

        // store it in the hashmap
        let mut write = REGISTERED_SERVER_FUNCTIONS
            .write()
            .map_err(|e| ServerFnError::Registration(e.to_string()))?;
        write.insert(Self::url(), run_server_fn);

        Ok(())
    }
}

#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum ServerFnError {
    #[error("error while trying to register the server function: {0}")]
    Registration(String),
    #[error("error reaching server to call server function: {0}")]
    Request(String),
    #[error("error running server function: {0}")]
    ServerError(String),
    #[error("error deserializing server function results {0}")]
    Deserialization(String),
    #[error("error serializing server function results {0}")]
    Serialization(String),
    #[error("error deserializing server function arguments {0}")]
    Args(String),
}

pub async fn call_server_fn<T>(url: &str, args: impl ServerFn) -> Result<T, ServerFnError>
where
    T: Serializable + Sized,
{
    let window = window();

    let args_form_data = web_sys::FormData::new().expect_throw("could not create FormData");
    for (field_name, value) in args.as_form_data().into_iter() {
        args_form_data
            .append_with_str(field_name, &value)
            .expect_throw("could not append form field");
    }
    let args_form_data = web_sys::UrlSearchParams::new_with_str_sequence_sequence(&args_form_data)
        .expect_throw("could not URL encode FormData");
    let args_form_data = args_form_data.to_string().as_string().unwrap_or_default();

    let resp = gloo::net::http::Request::post(url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Accept", "application/json")
        .body(args_form_data.to_string())
        .send()
        .await
        .map_err(|e| ServerFnError::Request(e.to_string()))?;

    // check for error status
    let status = resp.status();
    if status >= 500 && status <= 599 {
        return Err(ServerFnError::ServerError(resp.status_text()));
    }

    let text = resp
        .text()
        .await
        .map_err(|e| ServerFnError::Deserialization(e.to_string()))?;

    log::debug!("text is {text:?}");
    T::from_json(&text).map_err(|e| ServerFnError::Deserialization(e.to_string()))
}

#[derive(Clone)]
pub struct RouteAction<T>
where
    T: 'static,
{
    pub version: RwSignal<usize>,
    pending: RwSignal<bool>,
    action_fn: Rc<dyn Fn() -> Pin<Box<dyn Future<Output = T>>>>,
}

impl<T> RouteAction<T>
where
    T: 'static,
{
    pub fn invalidator(&self) {
        _ = self.version.get();
    }

    pub fn pending(&self) -> ReadSignal<bool> {
        self.pending.read_only()
    }

    pub fn dispatch(&self) {
        let fut = (self.action_fn)();
        let version = self.version;
        let pending = self.pending;
        pending.set(true);
        spawn_local(async move {
            let new_count = fut.await;
            pending.set(false);
            version.update(|n| *n += 1);
        })
    }
}

pub fn create_route_action<T, F, Fu>(cx: Scope, action_fn: F) -> RouteAction<T>
where
    T: 'static,
    F: Fn() -> Fu + 'static,
    Fu: Future<Output = T> + 'static,
{
    let version = create_rw_signal(cx, 0);
    let pending = create_rw_signal(cx, false);
    let action_fn = Rc::new(move || {
        log::debug!("running route action");
        let fut = action_fn();
        Box::pin(async move { fut.await }) as Pin<Box<dyn Future<Output = T>>>
    });

    RouteAction {
        version,
        pending,
        action_fn,
    }
}
