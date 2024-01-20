use futures::StreamExt;
use leptos::{html::Input, *};
use leptos_meta::{provide_meta_context, Link, Meta, Stylesheet};
use leptos_router::{ActionForm, Route, Router, Routes};
use server_fn::codec::{
    GetUrl, MultipartData, MultipartFormData, Rkyv, SerdeLite, StreamingText,
    TextStream,
};
#[cfg(feature = "ssr")]
use std::sync::{
    atomic::{AtomicU8, Ordering},
    Mutex,
};
use strum::{Display, EnumString};
use wasm_bindgen::JsCast;
use web_sys::{FormData, HtmlFormElement, SubmitEvent};

#[component]
pub fn TodoApp() -> impl IntoView {
    provide_meta_context();

    view! {
        <Meta name="color-scheme" content="dark light"/>
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
        <h2>"Custom Error Types"</h2>
        <CustomErrorTypes/>
        <h2>"Alternative Encodings"</h2>
        <ServerFnArgumentExample/>
        <RkyvExample/>
        <FileUpload/>
        <FileWatcher/>
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
                action.dispatch(text.into());
                // note: technically, this `action` takes `AddRow` (the server fn type) as its
                // argument
                //
                // however, for any one-argument server functions, `From<_>` is implemented between
                // the server function type and the type of this single argument
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
    output = SerdeLite,
)]
// You can use the `#[middleware]` macro to add appropriate middleware
// In this case, any `tower::Layer` that takes services of `Request<Body>` will work
#[middleware(crate::middleware::LoggingLayer)]
pub async fn length_of_input(input: String) -> Result<usize, ServerFnError> {
    println!("2. Running server function.");
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
            This example shows how to specify additional behavior including
            <ul>
                <li>Specific server function <strong>paths</strong></li>
                <li>Mixing and matching input and output <strong>encodings</strong></li>
                <li>Adding custom <strong>middleware</strong> on a per-server-fn basis</li>
            </ul>
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

#[component]
pub fn FileUpload() -> impl IntoView {
    /// A simple file upload function, which does just returns the length of the file.
    ///
    /// On the server, this uses the `multer` crate, which provides a streaming API.
    #[server(
    input = MultipartFormData,
)]
    pub async fn file_length(
        data: MultipartData,
    ) -> Result<usize, ServerFnError> {
        // `.into_inner()` returns the inner `multer` stream
        // it is `None` if we call this on the client, but always `Some(_)` on the server, so is safe to
        // unwrap
        let mut data = data.into_inner().unwrap();

        // this will just measure the total number of bytes uploaded
        let mut count = 0;
        while let Ok(Some(mut field)) = data.next_field().await {
            println!("\n[NEXT FIELD]\n");
            let name = field.name().unwrap_or_default().to_string();
            println!("  [NAME] {name}");
            while let Ok(Some(chunk)) = field.chunk().await {
                let len = chunk.len();
                count += len;
                println!("      [CHUNK] {len}");
                // in a real server function, you'd do something like saving the file here
            }
        }

        Ok(count)
    }

    let upload_action = create_action(|data: &FormData| {
        let data = data.clone();
        // `MultipartData` implements `From<FormData>`
        file_length(data.into())
    });

    view! {
        <h3>File Upload</h3>
        <p>Uploading files is fairly easy using multipart form data.</p>
        <form on:submit=move |ev: SubmitEvent| {
            ev.prevent_default();
            let target = ev.target().unwrap().unchecked_into::<HtmlFormElement>();
            let form_data = FormData::new_with_form(&target).unwrap();
            upload_action.dispatch(form_data);
        }>
            <input type="file" name="file_to_upload"/>
            <input type="submit"/>
        </form>
        <p>
            {move || if upload_action.input().get().is_none() && upload_action.value().get().is_none() {
                "Upload a file.".to_string()
            } else if upload_action.pending().get() {
                "Uploading...".to_string()
            } else if let Some(Ok(value)) = upload_action.value().get() {
                value.to_string()
            } else {
                format!("{:?}", upload_action.value().get())
            }}
        </p>
    }
}

#[component]
pub fn FileWatcher() -> impl IntoView {
    #[server(input = GetUrl, output = StreamingText)]
    pub async fn watched_files() -> Result<TextStream, ServerFnError> {
        use notify::{
            Config, Error, Event, RecommendedWatcher, RecursiveMode, Watcher,
        };
        use std::path::Path;

        let (tx, rx) = futures::channel::mpsc::unbounded();

        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, Error>| {
                if let Ok(ev) = res {
                    if let Some(path) = ev.paths.last() {
                        let filename = path
                            .file_name()
                            .unwrap()
                            .to_str()
                            .unwrap()
                            .to_string();
                        _ = tx.unbounded_send(filename); //res);
                    }
                }
            },
            Config::default(),
        )?;
        watcher
            .watch(Path::new("./watched_files"), RecursiveMode::Recursive)?;
        std::mem::forget(watcher);

        Ok(TextStream::from(rx))
    }

    let (files, set_files) = create_signal(Vec::new());

    create_effect(move |_| {
        spawn_local(async move {
            while let Some(res) =
                watched_files().await.unwrap().into_inner().next().await
            {
                if let Ok(filename) = res {
                    set_files.update(|n| n.push(filename));
                }
            }
        });
    });

    view! {
        <h3>Watching files and returning a streaming response</h3>
        <p>Files changed since you loaded the page:</p>
        <ul>
            {move || files.get().into_iter().map(|file| view! { <li><code>{file}</code></li> }).collect::<Vec<_>>()}
        </ul>
        <p><em>Add or remove some text files in the <code>watched_files</code> directory and see the list of changes here.</em></p>
    }
}

/// The `ServerFnError` type is generic over a custom error type, which defaults to `NoCustomError`
/// for backwards compatibility and to support the most common use case.
///
/// A custom error type should implement `FromStr` and `Display`, which allows it to be converted
/// into and from a string easily to be sent over the network. It does *not* need to implement
/// `Serialize` and `Deserialize`, although these can be used to generate the `FromStr`/`Display`
/// implementations if you'd like. However, it's much lighter weight to use something like `strum`
/// simply to generate those trait implementations.
#[server]
pub async fn ascii_uppercase(
    text: String,
) -> Result<String, ServerFnError<InvalidArgument>> {
    if text.len() < 5 {
        Err(InvalidArgument::TooShort.into())
    } else if text.len() > 15 {
        Err(InvalidArgument::TooLong.into())
    } else if text.is_ascii() {
        Ok(text.to_ascii_uppercase())
    } else {
        Err(InvalidArgument::NotAscii.into())
    }
}

// The EnumString and Display derive macros are provided by strum
#[derive(Debug, Clone, EnumString, Display)]
pub enum InvalidArgument {
    TooShort,
    TooLong,
    NotAscii,
}

#[component]
pub fn CustomErrorTypes() -> impl IntoView {
    let input_ref = NodeRef::<Input>::new();
    let (result, set_result) = create_signal(None);

    view! {
        <h3>Using custom error types</h3>
        <p>
            "Server functions can use a custom error type that is preserved across the network boundary."
        </p>
        <p>
            "Try typing a message that is between 5 and 15 characters of ASCII text below. Then try breaking \
            the rules!"
        </p>
        <input node_ref=input_ref placeholder="Type something here."/>
        <button
            on:click=move |_| {
                let value = input_ref.get().unwrap().value();
                spawn_local(async move {
                    let data = ascii_uppercase(value).await;
                    set_result(Some(data));
                });
            }
        >
            "Submit"
        </button>
        <p>
            {move || format!("{:?}", result.get())}
        </p>
    }
}
