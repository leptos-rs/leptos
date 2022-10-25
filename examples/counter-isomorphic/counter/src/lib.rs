use std::{
    fmt::Debug,
    sync::atomic::{AtomicI32, Ordering},
};

use leptos::*;

mod action;
use action::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "ssr")]
static COUNT: AtomicI32 = AtomicI32::new(0);

#[derive(Serialize, Deserialize, Debug)]
pub struct GetServerCount {}
impl AsFormData for GetServerCount {
    fn as_form_data(&self) -> Vec<(&'static str, String)> {
        vec![]
    }
}

#[cfg(feature = "ssr")]
pub async fn get_server_count() -> Result<i32, ServerFnError> {
    Ok(COUNT.load(Ordering::Relaxed))
}
#[cfg(not(feature = "ssr"))]
pub async fn get_server_count() -> Result<i32, ServerFnError> {
    call_server_fn("/api/get_server_count", GetServerCount {}).await
}
#[cfg(not(feature = "ssr"))]
pub async fn get_server_count_helper(args: GetServerCount) -> Result<i32, ServerFnError> {
    call_server_fn("/api/get_server_count", args).await
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AdjustServerCount {
    pub delta: i32,
}
impl AsFormData for AdjustServerCount {
    fn as_form_data(&self) -> Vec<(&'static str, String)> {
        vec![("delta", self.delta.to_string())]
    }
}

#[cfg(feature = "ssr")]
pub async fn adjust_server_count(delta: i32) -> Result<i32, ServerFnError> {
    let new = COUNT.load(Ordering::Relaxed) + delta;
    COUNT.store(new, Ordering::Relaxed);
    Ok(new)
}
#[cfg(not(feature = "ssr"))]
pub async fn adjust_server_count(delta: i32) -> Result<i32, ServerFnError> {
    adjust_server_count_helper(AdjustServerCount { delta }).await
}
#[cfg(not(feature = "ssr"))]
pub async fn adjust_server_count_helper(args: AdjustServerCount) -> Result<i32, ServerFnError> {
    call_server_fn("/api/adjust_server_count", args).await
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ClearServerCount {}
impl AsFormData for ClearServerCount {
    fn as_form_data(&self) -> Vec<(&'static str, String)> {
        vec![]
    }
}

#[cfg(feature = "ssr")]
pub async fn clear_server_count() -> Result<i32, ServerFnError> {
    COUNT.store(0, Ordering::Relaxed);
    Ok(0)
}
#[cfg(not(feature = "ssr"))]
pub async fn clear_server_count() -> Result<i32, ServerFnError> {
    clear_server_count_helper(ClearServerCount {}).await
}
#[cfg(not(feature = "ssr"))]
pub async fn clear_server_count_helper(args: ClearServerCount) -> Result<i32, ServerFnError> {
    call_server_fn("/api/clear_server_count", args).await
}

#[component]
pub fn Counter(cx: Scope) -> Element {
    let (update, set_update) = create_signal(cx, 0);

    let dec = create_route_action(cx, || adjust_server_count(-1));
    let inc = create_route_action(cx, || adjust_server_count(1));
    let clear = create_route_action(cx, clear_server_count);
    let counter = create_resource(
        cx,
        move || (dec.version.get(), inc.version.get(), clear.version.get()),
        |_| get_server_count(),
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
            <button on:click=move |_| clear.dispatch()>"Clear"</button>
            <button on:click=move |_| dec.dispatch()>"-1"</button>
            <span>"Value: " {move || value().to_string()} "!"</span>
            <button on:click=move |_| inc.dispatch()>"+1"</button>
            <form method="POST" action="/api/adjust_server_count">
                <input type="hidden" name="delta" value="1"/>
                <input type="submit" value="+1 (with Form)"/>
            </form>
        </div>
    }
}
