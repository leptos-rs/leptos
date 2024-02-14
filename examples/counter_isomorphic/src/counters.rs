use leptos::*;
use leptos_meta::*;
use leptos_router::*;
#[cfg(feature = "ssr")]
use tracing::instrument;

#[cfg(feature = "ssr")]
pub mod ssr_imports {
    pub use broadcaster::BroadcastChannel;
    pub use once_cell::sync::OnceCell;
    pub use std::sync::atomic::{AtomicI32, Ordering};

    pub static COUNT: AtomicI32 = AtomicI32::new(0);

    lazy_static::lazy_static! {
        pub static ref COUNT_CHANNEL: BroadcastChannel<i32> = BroadcastChannel::new();
    }

    static LOG_INIT: OnceCell<()> = OnceCell::new();

    pub fn init_logging() {
        LOG_INIT.get_or_init(|| {
            simple_logger::SimpleLogger::new().env().init().unwrap();
        });
    }
}

#[server]
#[cfg_attr(feature = "ssr", instrument)]
pub async fn get_server_count() -> Result<i32, ServerFnError> {
    use ssr_imports::*;

    Ok(COUNT.load(Ordering::Relaxed))
}

#[server]
#[cfg_attr(feature = "ssr", instrument)]
pub async fn adjust_server_count(
    delta: i32,
    msg: String,
) -> Result<i32, ServerFnError> {
    use ssr_imports::*;

    let new = COUNT.load(Ordering::Relaxed) + delta;
    COUNT.store(new, Ordering::Relaxed);
    _ = COUNT_CHANNEL.send(&new).await;
    println!("message = {:?}", msg);
    Ok(new)
}

#[server]
#[cfg_attr(feature = "ssr", instrument)]
pub async fn clear_server_count() -> Result<i32, ServerFnError> {
    use ssr_imports::*;

    COUNT.store(0, Ordering::Relaxed);
    _ = COUNT_CHANNEL.send(&0).await;
    Ok(0)
}
#[component]
pub fn Counters() -> impl IntoView {
    #[cfg(feature = "ssr")]
    ssr_imports::init_logging();

    provide_meta_context();
    view! {
        <Router>
            <header>
                <h1>"Server-Side Counters"</h1>
                <p>"Each of these counters stores its data in the same variable on the server."</p>
                <p>
                    "The value is shared across connections. Try opening this is another browser tab to see what I mean."
                </p>
            </header>
            <nav>
                <ul>
                    <li>
                        <A href="">"Simple"</A>
                    </li>
                    <li>
                        <A href="form">"Form-Based"</A>
                    </li>
                    <li>
                        <A href="multi">"Multi-User"</A>
                    </li>
                </ul>
            </nav>
            <Link rel="shortcut icon" type_="image/ico" href="/favicon.ico"/>
            <main>
                <Routes>
                    <Route
                        path=""
                        view=|| {
                            view! { <Counter/> }
                        }
                    />
                    <Route
                        path="form"
                        view=|| {
                            view! { <FormCounter/> }
                        }
                    />
                    <Route
                        path="multi"
                        view=|| {
                            view! { <MultiuserCounter/> }
                        }
                    />
                </Routes>
            </main>
        </Router>
    }
}

// This is an example of "single-user" server functions
// The counter value is loaded from the server, and re-fetches whenever
// it's invalidated by one of the user's own actions
// This is the typical pattern for a CRUD app
#[component]
pub fn Counter() -> impl IntoView {
    let dec = create_action(|_: &()| adjust_server_count(-1, "decing".into()));
    let inc = create_action(|_: &()| adjust_server_count(1, "incing".into()));
    let clear = create_action(|_: &()| clear_server_count());
    let counter = create_resource(
        move || {
            (
                dec.version().get(),
                inc.version().get(),
                clear.version().get(),
            )
        },
        |_| get_server_count(),
    );

    view! {
        <div>
            <h2>"Simple Counter"</h2>
            <p>
                "This counter sets the value on the server and automatically reloads the new value."
            </p>
            <div>
                <button on:click=move |_| clear.dispatch(())>"Clear"</button>
                <button on:click=move |_| dec.dispatch(())>"-1"</button>
                <span>
                    "Value: "
                    <Suspense>
                        {move || counter.and_then(|count| *count)} "!"
                    </Suspense>
                </span>
                <button on:click=move |_| inc.dispatch(())>"+1"</button>
            </div>
            <Suspense>
              {move || {
                counter.get().and_then(|res| match res {
                  Ok(_) => None,
                  Err(e) => Some(e),
                }).map(|msg| {
                  view! { <p>"Error: " {msg.to_string()}</p> }
                })
              }}
            </Suspense>
        </div>
    }
}

// This is the <Form/> counter
// It uses the same invalidation pattern as the plain counter,
// but uses HTML forms to submit the actions
#[component]
pub fn FormCounter() -> impl IntoView {
    // these struct names are auto-generated by #[server]
    // they are just the PascalCased versions of the function names
    let adjust = create_server_action::<AdjustServerCount>();
    let clear = create_server_action::<ClearServerCount>();

    let counter = create_resource(
        move || (adjust.version().get(), clear.version().get()),
        |_| {
            log::debug!("FormCounter running fetcher");
            get_server_count()
        },
    );
    let value = move || {
        log::debug!("FormCounter looking for value");
        counter.get().and_then(|n| n.ok()).unwrap_or(0)
    };

    view! {
        <div>
            <h2>"Form Counter"</h2>
            <p>
                "This counter uses forms to set the value on the server. When progressively enhanced, it should behave identically to the “Simple Counter.”"
            </p>
            <div>
                // calling a server function is the same as POSTing to its API URL
                // so we can just do that with a form and button
                <ActionForm action=clear>
                    <input type="submit" value="Clear"/>
                </ActionForm>
                // We can submit named arguments to the server functions
                // by including them as input values with the same name
                <ActionForm action=adjust>
                    <input type="hidden" name="delta" value="-1"/>
                    <input type="hidden" name="msg" value="form value down"/>
                    <input type="submit" value="-1"/>
                </ActionForm>
                <span>"Value: " <Suspense>{move || value().to_string()} "!"</Suspense></span>
                <ActionForm action=adjust>
                    <input type="hidden" name="delta" value="1"/>
                    <input type="hidden" name="msg" value="form value up"/>
                    <input type="submit" value="+1"/>
                </ActionForm>
            </div>
        </div>
    }
}

// This is a kind of "multi-user" counter
// It relies on a stream of server-sent events (SSE) for the counter's value
// Whenever another user updates the value, it will update here
// This is the primitive pattern for live chat, collaborative editing, etc.
#[component]
pub fn MultiuserCounter() -> impl IntoView {
    let dec =
        create_action(|_: &()| adjust_server_count(-1, "dec dec goose".into()));
    let inc =
        create_action(|_: &()| adjust_server_count(1, "inc inc moose".into()));
    let clear = create_action(|_: &()| clear_server_count());

    #[cfg(not(feature = "ssr"))]
    let multiplayer_value = {
        use futures::StreamExt;

        let mut source =
            gloo_net::eventsource::futures::EventSource::new("/api/events")
                .expect("couldn't connect to SSE stream");
        let s = create_signal_from_stream(
            source
                .subscribe("message")
                .unwrap()
                .map(|value| match value {
                    Ok(value) => value
                        .1
                        .data()
                        .as_string()
                        .expect("expected string value"),
                    Err(_) => "0".to_string(),
                }),
        );

        on_cleanup(move || source.close());
        s
    };

    #[cfg(feature = "ssr")]
    let (multiplayer_value, _) = create_signal(None::<i32>);

    view! {
        <div>
            <h2>"Multi-User Counter"</h2>
            <p>
                "This one uses server-sent events (SSE) to live-update when other users make changes."
            </p>
            <div>
                <button on:click=move |_| clear.dispatch(())>"Clear"</button>
                <button on:click=move |_| dec.dispatch(())>"-1"</button>
                <span>
                    "Multiplayer Value: " {move || multiplayer_value.get().unwrap_or_default()}
                </span>
                <button on:click=move |_| inc.dispatch(())>"+1"</button>
            </div>
        </div>
    }
}
