use std::{
    fmt::Debug,
    future::Future,
    sync::atomic::{AtomicI32, Ordering},
};

use leptos::*;

use serde::{Deserialize, Serialize};
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

pub async fn call_server_fn<T>(url: &str) -> Result<T, ServerFnError>
where
    T: Serializable + Sized,
{
    let window = window();
    let resp = gloo_net::http::Request::get(url)
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

#[cfg(feature = "ssr")]
static COUNT: AtomicI32 = AtomicI32::new(0);

#[cfg(feature = "ssr")]
pub async fn get_server_count(args: ()) -> Result<i32, ServerFnError> {
    Ok(COUNT.load(Ordering::Relaxed))
}
#[cfg(not(feature = "ssr"))]
pub async fn get_server_count(args: ()) -> Result<i32, ServerFnError> {
    call_server_fn("/api/get_server_count").await
}

#[cfg(feature = "ssr")]
pub async fn increment_server_count(args: ()) -> Result<i32, ServerFnError> {
    let new = COUNT.load(Ordering::Relaxed) + 1;
    COUNT.store(new, Ordering::Relaxed);
    Ok(new)
}
#[cfg(not(feature = "ssr"))]
pub async fn increment_server_count(args: ()) -> Result<i32, ServerFnError> {
    call_server_fn("/api/increment_server_count").await
}

#[cfg(feature = "ssr")]
pub async fn decrement_server_count(args: ()) -> Result<i32, ServerFnError> {
    let new = COUNT.load(Ordering::Relaxed) - 1;
    COUNT.store(new, Ordering::Relaxed);
    Ok(new)
}
#[cfg(not(feature = "ssr"))]
pub async fn decrement_server_count(args: ()) -> Result<i32, ServerFnError> {
    call_server_fn("/api/decrement_server_count").await
}
impl decrement_server_count {}

#[cfg(feature = "ssr")]
pub async fn clear_server_count(args: ()) -> Result<i32, ServerFnError> {
    COUNT.store(0, Ordering::Relaxed);
    Ok(0)
}
#[cfg(not(feature = "ssr"))]
pub async fn clear_server_count(args: ()) -> Result<i32, ServerFnError> {
    call_server_fn("/api/clear_server_count").await
}

#[derive(Clone)]
pub struct RouteAction<T, U>
where
    T: 'static,
    U: 'static,
{
    version: RwSignal<usize>,
    pending: RwSignal<bool>,
    current_args: RwSignal<Option<T>>,
    action_fn: Rc<dyn Fn(T) -> Pin<Box<dyn Future<Output = U>>>>,
}

impl<T, U> RouteAction<T, U>
where
    T: 'static,
    U: 'static,
{
    pub fn invalidator(&self) {
        _ = self.version.get();
    }

    pub fn pending(&self) -> ReadSignal<bool> {
        self.pending.read_only()
    }

    pub fn input(&self) -> ReadSignal<Option<T>> {
        self.current_args.read_only()
    }

    pub fn dispatch(&self, args: T) {
        let fut = (self.action_fn)(args);
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

fn create_route_action<T, U, Fu>(
    cx: Scope,
    action_fn: impl Fn(T) -> Fu + 'static,
) -> RouteAction<T, U>
where
    T: 'static,
    Fu: Future<Output = U> + 'static,
{
    let version = create_rw_signal(cx, 0);
    let pending = create_rw_signal(cx, false);
    let current_args = create_rw_signal(cx, None);
    let action_fn = Rc::new(move |args| {
        let fut = action_fn(args);
        Box::pin(async move { fut.await }) as Pin<Box<dyn Future<Output = U>>>
    });

    RouteAction {
        version,
        pending,
        current_args,
        action_fn,
    }
}

#[component]
pub fn Counter(cx: Scope) -> Element {
    let (update, set_update) = create_signal(cx, 0);

    let dec = create_route_action(cx, decrement_server_count);
    let inc = create_route_action(cx, increment_server_count);
    let clear = create_route_action(cx, clear_server_count);

    let counter = create_resource(
        cx,
        move || (dec.version.get(), inc.version.get(), clear.version.get()),
        |_| get_server_count(()),
    );

    let value = move || counter.read().map(|count| count.unwrap_or(0)).unwrap_or(0);
    let error_msg = move || {
        counter
            .read()
            .map(|res| match res {
                Ok(_) => None,
                Err(e) => Some(e),
            })
            .flatten()
    };

    view! {
        cx,
        <div>
            <button on:click=move |_| clear.dispatch(())>"Clear"</button>
            <button on:click=move |_| dec.dispatch(())>"-1"</button>
            <span>"Value: " {move || value().to_string()} "!"</span>
            <button on:click=move |_| inc.dispatch(())>"+1"</button>
        </div>
    }
}
