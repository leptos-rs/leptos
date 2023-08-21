/// This file is mostly copied from the counters isomorphic example
/// just to demonstrate server actions in wasm
use leptos::*;
use std::sync::atomic::{AtomicI32, Ordering};
static COUNT: AtomicI32 = AtomicI32::new(0);

// "/api" is an optional prefix that allows you to locate server functions wherever you'd like on the server
#[server(GetServerCount, "/api")]
pub async fn get_server_count() -> Result<i32, ServerFnError> {
    Ok(COUNT.load(Ordering::Relaxed))
}

#[server(AdjustServerCount, "/api")]
pub async fn adjust_server_count(
    delta: i32,
    msg: String,
) -> Result<i32, ServerFnError> {
    let new = COUNT.load(Ordering::Relaxed) + delta;
    COUNT.store(new, Ordering::Relaxed);
    println!("message = {:?}", msg);
    Ok(new)
}

#[server(ClearServerCount, "/api")]
pub async fn clear_server_count() -> Result<i32, ServerFnError> {
    COUNT.store(0, Ordering::Relaxed);
    Ok(0)
}

// This is an example of "single-user" server functions
// The counter value is loaded from the server, and re-fetches whenever
// it's invalidated by one of the user's own actions
// This is the typical pattern for a CRUD app
#[component]
pub fn Counter(cx: Scope) -> impl IntoView {
    let dec = create_action(cx, |_| adjust_server_count(-1, "decing".into()));
    let inc = create_action(cx, |_| adjust_server_count(1, "incing".into()));
    let clear = create_action(cx, |_| clear_server_count());
    let counter = create_resource(
        cx,
        move || {
            (
                dec.version().get(),
                inc.version().get(),
                clear.version().get(),
            )
        },
        |_| get_server_count(),
    );

    let value = move || {
        counter
            .read(cx)
            .map(|count| count.unwrap_or(0))
            .unwrap_or(0)
    };
    let error_msg = move || {
        counter.read(cx).and_then(|res| match res {
            Ok(_) => None,
            Err(e) => Some(e),
        })
    };

    view! { cx,
        <div>
            <h2>"Simple Counter"</h2>
            <p>
                "This counter sets the value on the server and automatically reloads the new value."
            </p>
            <div>
                <button on:click=move |_| clear.dispatch(())>"Clear"</button>
                <button on:click=move |_| dec.dispatch(())>"-1"</button>
                <span>"Value: " {value}</span>
                <button on:click=move |_| inc.dispatch(())>"+1"</button>
            </div>
            {move || {
                error_msg()
                    .map(|msg| {
                        view! { cx, <p>"Error: " {msg.to_string()}</p> }
                    })
            }}
        </div>
    }
}
