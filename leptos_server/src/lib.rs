pub use async_trait::async_trait;
use leptos_reactive::*;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{future::Future, pin::Pin, rc::Rc};
use thiserror::Error;

#[cfg(feature = "ssr")]
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

#[cfg(feature = "ssr")]
type ServerFnTraitObj =
    dyn Fn(&[u8]) -> Pin<Box<dyn Future<Output = Result<String, ServerFnError>>>> + Send + Sync;

#[cfg(feature = "ssr")]
lazy_static::lazy_static! {
    pub static ref REGISTERED_SERVER_FUNCTIONS: Arc<RwLock<HashMap<&'static str, Arc<ServerFnTraitObj>>>> = Default::default();
}

#[cfg(feature = "ssr")]
pub fn server_fn_by_path(path: &str) -> Option<Arc<ServerFnTraitObj>> {
    REGISTERED_SERVER_FUNCTIONS
        .read()
        .ok()
        .and_then(|fns| fns.get(path).cloned())
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

#[cfg(any(feature = "csr", feature = "hydrate"))]
pub async fn call_server_fn<T>(url: &str, args: impl ServerFn) -> Result<T, ServerFnError>
where
    T: Serializable + Sized,
{
    use leptos_dom::*;

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

    let resp = gloo_net::http::Request::post(url)
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

    T::from_json(&text).map_err(|e| ServerFnError::Deserialization(e.to_string()))
}

#[derive(Clone)]
pub struct AsyncAction<T>
where
    T: 'static,
{
    pub version: RwSignal<usize>,
    value: RwSignal<Option<T>>,
    pending: RwSignal<bool>,
    action_fn: Rc<dyn Fn() -> Pin<Box<dyn Future<Output = T>>>>,
}

impl<T> AsyncAction<T>
where
    T: 'static,
{
    pub fn invalidator(&self) {
        _ = self.version.get();
    }

    pub fn pending(&self) -> impl Fn() -> bool {
        let value = self.value;
        move || value.with(|val| val.is_some())
    }

    pub fn value(&self) -> ReadSignal<Option<T>> {
        self.value.read_only()
    }

    pub fn dispatch(&self) {
        let fut = (self.action_fn)();
        let version = self.version;
        let pending = self.pending;
        let value = self.value;
        pending.set(true);
        spawn_local(async move {
            let new_value = fut.await;
            value.set(Some(new_value));
            pending.set(false);
            version.update(|n| *n += 1);
        })
    }
}

pub fn create_async_action<T, F, Fu>(cx: Scope, action_fn: F) -> AsyncAction<T>
where
    T: 'static,
    F: Fn() -> Fu + 'static,
    Fu: Future<Output = T> + 'static,
{
    let version = create_rw_signal(cx, 0);
    let value = create_rw_signal(cx, None);
    let pending = create_rw_signal(cx, false);
    let action_fn = Rc::new(move || {
        let fut = action_fn();
        Box::pin(async move { fut.await }) as Pin<Box<dyn Future<Output = T>>>
    });

    AsyncAction {
        version,
        value,
        pending,
        action_fn,
    }
}
