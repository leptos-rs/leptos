use std::sync::atomic::{AtomicI32, Ordering};

use leptos::*;

use serde::{Deserialize, Serialize};
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
pub async fn get_server_count() -> Result<i32, ServerFnError> {
    Ok(COUNT.load(Ordering::Relaxed))
}
#[cfg(not(feature = "ssr"))]
pub async fn get_server_count() -> Result<i32, ServerFnError> {
    call_server_fn("/api/get_server_count").await
}

#[cfg(feature = "ssr")]
pub async fn increment_server_count() -> Result<i32, ServerFnError> {
    let new = COUNT.load(Ordering::Relaxed) + 1;
    COUNT.store(new, Ordering::Relaxed);
    Ok(new)
}
#[cfg(not(feature = "ssr"))]
pub async fn increment_server_count() -> Result<i32, ServerFnError> {
    call_server_fn("/api/increment_server_count").await
}

#[cfg(feature = "ssr")]
pub async fn decrement_server_count() -> Result<i32, ServerFnError> {
    let new = COUNT.load(Ordering::Relaxed) - 1;
    COUNT.store(new, Ordering::Relaxed);
    Ok(new)
}
#[cfg(not(feature = "ssr"))]
pub async fn decrement_server_count() -> Result<i32, ServerFnError> {
    call_server_fn("/api/decrement_server_count").await
}

#[cfg(feature = "ssr")]
pub async fn clear_server_count() -> Result<i32, ServerFnError> {
    COUNT.store(0, Ordering::Relaxed);
    Ok(0)
}
#[cfg(not(feature = "ssr"))]
pub async fn clear_server_count() -> Result<i32, ServerFnError> {
    call_server_fn("/api/clear_server_count").await
}

#[component]
pub fn Counter(cx: Scope) -> Element {
    let (update, set_update) = create_signal(cx, 0);
    let counter = create_resource(cx, move || update(), |_| get_server_count());

    let dec = move |_| {
        spawn_local(async move {
            let new_count = decrement_server_count().await;
            if let Ok(new_count) = new_count {
                set_update.update(|n| *n += 1);
            }
        })
    };

    let inc = move |_| {
        spawn_local(async move {
            let new_count = increment_server_count().await;
            if let Ok(new_count) = new_count {
                set_update.update(|n| *n += 1);
            }
        })
    };

    let clear = move |_| {
        spawn_local(async move {
            let new_count = clear_server_count().await;
            if let Ok(new_count) = new_count {
                set_update.update(|n| *n += 1);
            }
        })
    };

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
            <button on:click=clear>"Clear"</button>
            <button on:click=dec>"-1"</button>
            <span>"Value: " {move || value().to_string()} "!"</span>
            <button on:click=inc>"+1"</button>
        </div>
    }
}
