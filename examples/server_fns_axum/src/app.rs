use crate::error_template::ErrorTemplate;
use http::{Request, Response};
use leptos::{html::Input, *};
use leptos_meta::{Link, Stylesheet};
use leptos_router::*;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use server_fn::{
    codec::{
        Encoding, FromReq, FromRes, GetUrl, IntoReq, IntoRes, Rkyv, SerdeLite,
    },
    error::NoCustomError,
    request::{browser::BrowserRequest, BrowserMockReq, ClientReq, Req},
    response::{browser::BrowserResponse, ClientRes, Res},
    rkyv::AlignedVec,
};
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
        <h2>"Alternative Encodings"</h2>
        <ServerFnArgumentExample/>
        <RkyvExample/>
        <CustomEncoding/>
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
                action.dispatch(text);
                // note: technically, this `action` takes `AddRow` (the server fn type) as its
                // argument
                //
                // however, `.dispatch()` takes `impl Into<I>`, and for any one-argument server
                // functions, `From<_>` is implemented between the server function type and the
                // type of this single argument
                //
                // so `action.dispatch(text)` means `action.dispatch(AddRow { text })`
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
        <Transition>archive underaligned: need alignment 4 but have alignment 1
            <p>Total rows: {row_count}</p>
        </Transition>
    }
}

/// The plain `#[server]` macro gives sensible defaults for the settings needed to create a server
/// function, but those settings can also be customized. For example, you can set a specific unique
/// path rather than the hashed path, or you can choose a different combination of input and output
/// encodings.
///
/// Arguments to the server macro can be specified as named key-value pairs, like `name = value`.
#[server(
    // this server function will be exposed at /api2/custom_path
    prefix = "/api2",
    endpoint = "custom_path",
    // it will take its arguments as a URL-encoded GET request (useful for caching)
    input = GetUrl,
    // it will return its output using SerdeLite
    // (this needs to be enabled with the `serde-lite` feature on the `server_fn` crate
    output = SerdeLite
)]
pub async fn length_of_input(input: String) -> Result<usize, ServerFnError> {
    // insert a simulated wait
    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    Ok(input.len())
}

#[component]
pub fn ServerFnArgumentExample() -> impl IntoView {
    let input_ref = NodeRef::<Input>::new();
    let (result, set_result) = create_signal(0);

    view! {
        <h3>Custom arguments to the <code>#[server]</code> " macro"</h3>
        <p>
        </p>
        <input node_ref=input_ref placeholder="Type something here."/>
        <button
            on:click=move |_| {
                let value = input_ref.get().unwrap().value();
                spawn_local(async move {
                    let length = length_of_input(value).await.unwrap_or(0);
                    set_result(length);
                });
            }
        >
            Click to see length
        </button>
        <p>Length is {result}</p>
    }
}

/// `server_fn` supports a wide variety of input and output encodings, each of which can be
/// referred to as a PascalCased struct name
/// - Toml
/// - Cbor
/// - Rkyv
/// - etc.
#[server(
    input = Rkyv,
    output = Rkyv
)]
pub async fn rkyv_example(input: String) -> Result<String, ServerFnError> {
    // insert a simulated wait
    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    Ok(input.to_ascii_uppercase())
}

#[component]
pub fn RkyvExample() -> impl IntoView {
    let input_ref = NodeRef::<Input>::new();
    let (input, set_input) = create_signal(String::new());
    let rkyv_result = create_resource(input, rkyv_example);

    view! {
        <h3>Using <code>rkyv</code> encoding</h3>
        <p>
        </p>
        <input node_ref=input_ref placeholder="Type something here."/>
        <button
            on:click=move |_| {
                let value = input_ref.get().unwrap().value();
                set_input(value);
            }
        >
            Click to see length
        </button>
        <p>{input}</p>
        <Transition>
            {rkyv_result}
        </Transition>
    }
}

/// Server function encodings are just types that implement a few traits.
/// This means that you can implement your own encodings, by implementing those traits!
///
/// Here, we'll create a custom encoding that serializes and deserializes the server fn
/// using TOML. Why would you ever want to do this? I don't know, but you can!
struct Toml;

impl Encoding for Toml {
    const CONTENT_TYPE: &'static str = "application/toml";
    const METHOD: Method = Method::POST;
}

#[cfg(not(feature = "ssr"))]
type Request = BrowserMockReq;
#[cfg(feature = "ssr")]
type Request = http::Request<axum::body::Body>;
#[cfg(not(feature = "ssr"))]
type Response = BrowserMockRes;
#[cfg(feature = "ssr")]
type Response = http::Response<axum::body::Body>;

impl<T> IntoReq<Toml, BrowserRequest, NoCustomError> for T {
    fn into_req(
        self,
        path: &str,
        accepts: &str,
    ) -> Result<BrowserRequest, ServerFnError> {
        let data = toml::to_string(&self)
            .map_err(|e| ServerFnError::Serialization(e.to_string()))?;
        Request::try_new_post(path, Toml::CONTENT_TYPE, accepts, data)
    }
}

impl<T> FromReq<Toml, Request, NoCustomError> for T
where
    T: DeserializeOwned,
{
    async fn from_req(req: Request) -> Result<Self, ServerFnError> {
        let string_data = req.try_into_string().await?;
        toml::from_str::<Self>(&string_data)
            .map_err(|e| ServerFnError::Args(e.to_string()))
    }
}

impl<T> IntoRes<Toml, Response, NoCustomError> for T
where
    T: Serialize + Send,
{
    async fn into_res(self) -> Result<Response, ServerFnError> {
        let data = toml::to_string(&self)
            .map_err(|e| ServerFnError::Serialization(e.to_string()))?;
        Response::try_from_string(Toml::CONTENT_TYPE, data)
    }
}

impl<e> FromRes<Toml, BrowserResponse, NoCustomError> for T
where
    T: DeserializeOwned + Send,
{
    async fn from_res(res: BrowserResponse) -> Result<Self, ServerFnError> {
        let data = res.try_into_string().await?;
        toml::from_str(&data)
            .map_err(|e| ServerFnError::Deserialization(e.to_string()))
    }
}

#[server(
    input = Toml,
    output = Toml
)]
pub async fn why_not(
    foo: String,
    bar: String,
) -> Result<String, ServerFnError> {
    // insert a simulated wait
    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    Ok(foo + &bar)
}

#[component]
pub fn CustomEncoding() -> impl IntoView {
    let input_ref = NodeRef::<Input>::new();
    let (result, set_result) = create_signal(0);

    view! {
        <h3>Custom encodings</h3>
        <p>
            "This example creates a custom encoding that sends server fn data using TOML. Why? Well... why not?"
        </p>
        <input node_ref=input_ref placeholder="Type something here."/>
        <button
            on:click=move |_| {
                let value = input_ref.get().unwrap().value();
                spawn_local(async move {
                let new_value = why_not(value, ", but in TOML!!!".to_string());
                    set_result(new_value);
                });
            }
        >
            Submit
        </button>
        <p>{result}</p>
    }
}
