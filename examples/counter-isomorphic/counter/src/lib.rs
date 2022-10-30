use leptos::*;

use std::fmt::Debug;

#[cfg(feature = "ssr")]
use std::sync::atomic::{AtomicI32, Ordering};

#[cfg(feature = "ssr")]
use broadcaster::BroadcastChannel;

use serde::{Deserialize, Serialize};

#[cfg(feature = "ssr")]
pub fn register_server_functions() {
    GetServerCount::register();
    AdjustServerCount::register();
    ClearServerCount::register();
}

#[cfg(feature = "ssr")]
static COUNT: AtomicI32 = AtomicI32::new(0);

#[cfg(feature = "ssr")]
lazy_static::lazy_static! {
    pub static ref COUNT_CHANNEL: BroadcastChannel<i32> = BroadcastChannel::new();
}

#[server(GetServerCount)]
pub async fn get_server_count() -> Result<i32, ServerFnError> {
    Ok(COUNT.load(Ordering::Relaxed))
}

#[server(AdjustServerCount)]
pub async fn adjust_server_count(delta: i32, msg: String) -> Result<i32, ServerFnError> {
    let new = COUNT.load(Ordering::Relaxed) + delta;
    COUNT.store(new, Ordering::Relaxed);
    _ = COUNT_CHANNEL.send(&new).await;
    println!("message = {:?}", msg);
    Ok(new)
}

#[server(ClearServerCount)]
pub async fn clear_server_count() -> Result<i32, ServerFnError> {
    COUNT.store(0, Ordering::Relaxed);
    _ = COUNT_CHANNEL.send(&0).await;
    Ok(0)
}

#[component]
pub fn Counters(cx: Scope) -> Element {
    view! {
        cx,
        <div>
            <h1>"Server-Side Counters"</h1>
            <p>"Each of these counters stores its data in the same variable on the server."</p>
            <p>"The value is shared across connections. Try opening this is another browser tab to see what I mean."</p>
            <div style="display: flex; justify-content: space-around">
                <div>
                    <h2>"Simple Counter"</h2>
                    <p>"This counter sets the value on the server and automatically reloads the new value."</p>
                    <Counter/>
                </div>
                <div>
                    <h2>"Form Counter"</h2>
                    <p>"This counter uses forms to set the value on the server. When progressively enhanced, it should behave identically to the “Simple Counter.”"</p>
                    <FormCounter/>
                </div>
                <div>
                    <h2>"Multi-User Counter"</h2>
                    <p>"This one uses server-sent events (SSE) to live-update when other users make changes."</p>
                    <MultiuserCounter/>
                </div>
            </div>
        </div>
    }
}

// This is an example of "single-user" server functions
// The counter value is loaded from the server, and re-fetches whenever
// it's invalidated by one of the user's own actions
// This is the typical pattern for a CRUD app
#[component]
pub fn Counter(cx: Scope) -> Element {
    let dec = create_action(cx, || adjust_server_count(-1, "decing".into()));
    let inc = create_action(cx, || adjust_server_count(1, "incing".into()));
    let clear = create_action(cx, clear_server_count);
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
        </div>
    }
}

// This is the <Form/> counter
// It uses the same invalidation pattern as the plain counter,
// but uses HTML forms to submit the actions
#[component]
pub fn FormCounter(cx: Scope) -> Element {
    let counter = create_resource(
        cx,
        move || (),
        |_| {
            log::debug!("FormCounter running fetcher");

            get_server_count()
        },
    );
    let value = move || {
        log::debug!("FormCounter looking for value");
        counter
            .read()
            .map(|n| n.ok())
            .flatten()
            .map(|n| n)
            .unwrap_or(0)
    };

    view! {
        cx,
        <div>
            // calling a server function is the same as POSTing to its API URL
            // so we can just do that with a form and button
            <form method="POST" action=ClearServerCount::url()>
                <input type="submit" value="Clear"/>
            </form>
            // We can submit named arguments to the server functions
            // by including them as input values with the same name
            <form method="POST" action=AdjustServerCount::url()>
                <input type="hidden" name="delta" value="-1"/>
                <input type="hidden" name="msg" value="\"form value down\""/>
                <input type="submit" value="-1"/>
            </form>
            <span>"Value: " {move || value().to_string()} "!"</span>
            <form method="POST" action=AdjustServerCount::url()>
                <input type="hidden" name="delta" value="1"/>
                <input type="hidden" name="msg" value="\"form value up\""/>
                <input type="submit" value="+1"/>
            </form>
        </div>
    }
}

// This is a kind of "multi-user" counter
// It relies on a stream of server-sent events (SSE) for the counter's value
// Whenever another user updates the value, it will update here
// This is the primitive pattern for live chat, collaborative editing, etc.
#[component]
pub fn MultiuserCounter(cx: Scope) -> Element {
    let dec = create_action(cx, || adjust_server_count(-1, "dec dec goose".into()));
    let inc = create_action(cx, || adjust_server_count(1, "inc inc moose".into()));
    let clear = create_action(cx, clear_server_count);

    #[cfg(not(feature = "ssr"))]
    let multiplayer_value = {
        use futures::StreamExt;

        let mut source = gloo::net::eventsource::futures::EventSource::new("/api/events")
            .expect_throw("couldn't connect to SSE stream");
        let s = create_signal_from_stream(
            cx,
            source.subscribe("message").unwrap().map(|value| {
                value
                    .expect_throw("no message event")
                    .1
                    .data()
                    .as_string()
                    .expect_throw("expected string value")
            }),
        );

        on_cleanup(cx, move || source.close());
        s
    };

    #[cfg(feature = "ssr")]
    let multiplayer_value =
        create_signal_from_stream(cx, futures::stream::once(Box::pin(async { 0.to_string() })));

    view! {
        cx,
        <div>
            <button on:click=move |_| clear.dispatch()>"Clear"</button>
            <button on:click=move |_| dec.dispatch()>"-1"</button>
            <span>"Multiplayer Value: " {move || multiplayer_value().unwrap_or_default().to_string()}</span>
            <button on:click=move |_| inc.dispatch()>"+1"</button>
        </div>
    }
}
