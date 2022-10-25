use leptos::*;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use thiserror::Error;

#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum ServerFnError {
    #[error("error reaching server to call server function: {0}")]
    Request(String),
    #[error("error running server function: {0}")]
    ServerError(String),
    #[error("error deserializing server function results {0}")]
    Deserialization(String),
}

pub async fn call_server_fn<T>(url: &str, args: impl AsFormData) -> Result<T, ServerFnError>
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

    let resp = gloo_net::http::Request::post(url)
        .header("Content-Type", "application/x-www-form-urlencoded")
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
        let fut = action_fn();
        Box::pin(async move { fut.await }) as Pin<Box<dyn Future<Output = T>>>
    });

    RouteAction {
        version,
        pending,
        action_fn,
    }
}

pub trait AsFormData {
    fn as_form_data(&self) -> Vec<(&'static str, String)>;
}
