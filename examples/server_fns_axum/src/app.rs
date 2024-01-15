use crate::error_template::ErrorTemplate;
use leptos::{html::Input, *};
use leptos_meta::*;
use leptos_router::*;
use serde::{Deserialize, Serialize};
use server_fn::codec::SerdeLite;
#[cfg(feature = "ssr")]
use std::sync::{
    atomic::{AtomicU8, Ordering},
    Mutex,
};

#[component]
pub fn TodoApp() -> impl IntoView {
    provide_meta_context();

    view! {
        <Link rel="shortcut icon" type_="image/ico" href="/favicon.ico"/>
        <Stylesheet id="leptos" href="/pkg/server_fns_axum.css"/>
        <Router>
            <header>
                <h1>"Server Function Demo"</h1>
            </header>
            <main>
                <Routes>
                    <Route path="" view=HomePage/>
                </Routes>
            </main>
        </Router>
    }
}

#[component]
pub fn HomePage() -> impl IntoView {
    view! {
        <h2>"Some Simple Server Functions"</h2>
        <SpawnLocal/>
        <WithAnAction/>
        <WithActionForm/>
    }
}

/// A server function is really just an API call to your server. But it provides a plain async
/// function as a wrapper around that. This means you can call it like any other async code, just
/// by spawning a task with `spawn_local`.
///
/// In reality, you usually want to use a resource to load data from the server or an action to
/// mutate data on the server. But a simple `spawn_local` can make it more obvious what's going on.
#[component]
pub fn SpawnLocal() -> impl IntoView {
    /// A basic server function can be called like any other async function.
    ///
    /// You can define a server function at any scope. This one, for example, is only available
    /// inside the SpawnLocal component. **However**, note that all server functions are publicly
    /// available API endpoints: This scoping means you can only call this server function
    /// from inside this component, but it is still available at its URL to any caller, from within
    /// your app or elsewhere.
    #[server]
    pub async fn shouting_text(input: String) -> Result<String, ServerFnError> {
        // insert a simulated wait
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
        Ok(input.to_ascii_uppercase())
    }

    let input_ref = NodeRef::<Input>::new();
    let (shout_result, set_shout_result) =
        create_signal("Click me".to_string());

    view! {
        <h3>Using <code>spawn_local</code></h3>
        <p>
            "You can call a server function by using "<code>"spawn_local"</code> " in an event listener. "
            "Clicking this button should alert with the uppercase version of the input."
        </p>
        <input node_ref=input_ref placeholder="Type something here."/>
        <button
            on:click=move |_| {
                let value = input_ref.get().unwrap().value();
                spawn_local(async move {
                    let uppercase_text = shouting_text(value).await.unwrap_or_else(|e| e.to_string());
                    set_shout_result(uppercase_text);
                });
            }
        >
            {shout_result}
        </button>
    }
}

/// Pretend this is a database and we're storing some rows in memory!
/// This exists only on the server.
#[cfg(feature = "ssr")]
static ROWS: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Imagine this server function mutates some state on the server, like a database row.
/// Every third time, it will return an error.
///
/// This kind of mutation is often best handled by an Action.
/// Remember, if you're loading data, use a resource; if you're running an occasional action,
/// use an action.
#[server]
pub async fn add_row(text: String) -> Result<usize, ServerFnError> {
    static N: AtomicU8 = AtomicU8::new(0);

    // insert a simulated wait
    tokio::time::sleep(std::time::Duration::from_millis(250)).await;

    let nth_run = N.fetch_add(1, Ordering::Relaxed);
    // this will print on the server, like any server function
    println!("Adding {text:?} to the database!");
    if nth_run % 3 == 2 {
        Err(ServerFnError::new("Oh no! Couldn't add to database!"))
    } else {
        let mut rows = ROWS.lock().unwrap();
        rows.push(text);
        Ok(rows.len())
    }
}

/// Simply returns the number of rows.
#[server]
pub async fn get_rows() -> Result<usize, ServerFnError> {
    // insert a simulated wait
    tokio::time::sleep(std::time::Duration::from_millis(250)).await;

    Ok(ROWS.lock().unwrap().len())
}

/// An action abstracts over the process of spawning a future and setting a signal when it
/// resolves. Its .input() signal holds the most recent argument while it's still pending,
/// and its .value() signal holds the most recent result. Its .version() signal can be fed
/// into a resource, telling it to refetch whenever the action has successfully resolved.
///
/// This makes actions useful for mutations, i.e., some server function that invalidates
/// loaded previously loaded from another server function.
#[component]
pub fn WithAnAction() -> impl IntoView {
    let input_ref = NodeRef::<Input>::new();

    // a server action can be created by using the server function's type name as a generic
    // the type name defaults to the PascalCased function name
    let action = create_server_action::<AddRow>();

    // this resource will hold the total number of rows
    // passing it action.version() means it will refetch whenever the action resolves successfully
    let row_count = create_resource(action.version(), |_| get_rows());

    view! {
        <h3>Using <code>create_action</code></h3>
        <p>
            "Some server functions are conceptually \"mutations,\", which change something on the server. "
            "These often work well as actions."
        </p>
        <input node_ref=input_ref placeholder="Type something here."/>
        <button
            on:click=move |_| {
                let text = input_ref.get().unwrap().value();
                action.dispatch(AddRow { text });
            }
        >
            Submit
        </button>
        <p>You submitted: {move || format!("{:?}", action.input().get())}</p>
        <p>The result was: {move || format!("{:?}", action.value().get())}</p>
        <Transition>
            <p>Total rows: {row_count}</p>
        </Transition>
    }
}

/// An <ActionForm/> lets you do the same thing as dispatching an action, but automates the
/// creation of the dispatched argument struct using a <form>. This means it also gracefully
/// degrades well when JS/WASM are not available.
///
/// Try turning off WASM in your browser. The form still works, and successfully displays the error
/// message if the server function returns an error. Otherwise, it loads the new resource data.
#[component]
pub fn WithActionForm() -> impl IntoView {
    let action = create_server_action::<AddRow>();
    let row_count = create_resource(action.version(), |_| get_rows());

    view! {
        <h3>Using <code>"<ActionForm/>"</code></h3>
        <p>
            <code>"<ActionForm/>"</code> "lets you use an HTML " <code>"<form>"</code>
            "to call a server function in a way that gracefully degrades."
        </p>
        <ActionForm action>
            <input
                // the `name` of the input corresponds to the argument name
                name="text"
                placeholder="Type something here."
            />
            <button> Submit </button>
        </ActionForm>
        <p>You submitted: {move || format!("{:?}", action.input().get())}</p>
        <p>The result was: {move || format!("{:?}", action.value().get())}</p>
        <Transition>
            <p>Total rows: {row_count}</p>
        </Transition>
    }
}
