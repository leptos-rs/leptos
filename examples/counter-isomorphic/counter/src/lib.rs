use std::{
    fmt::Debug,
    sync::atomic::{AtomicI32, Ordering},
};

use leptos::*;

mod action;
use action::*;

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

#[cfg(feature = "ssr")]
pub async fn clear_server_count(args: ()) -> Result<i32, ServerFnError> {
    COUNT.store(0, Ordering::Relaxed);
    Ok(0)
}
#[cfg(not(feature = "ssr"))]
pub async fn clear_server_count(args: ()) -> Result<i32, ServerFnError> {
    call_server_fn("/api/clear_server_count").await
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
