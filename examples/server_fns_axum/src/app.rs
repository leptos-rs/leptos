use futures::StreamExt;
use http::Method;
use leptos::{html::Input, prelude::*, task::spawn_local};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use server_fn::{
    client::{browser::BrowserClient, Client},
    codec::{
        Encoding, FromReq, FromRes, GetUrl, IntoReq, IntoRes, MultipartData,
        MultipartFormData, Postcard, Rkyv, SerdeLite, StreamingText,
        TextStream,
    },
    request::{browser::BrowserRequest, ClientReq, Req},
    response::{browser::BrowserResponse, ClientRes, Res},
};
use std::future::Future;
#[cfg(feature = "ssr")]
use std::sync::{
    atomic::{AtomicU8, Ordering},
    Mutex,
};
use strum::{Display, EnumString};
use wasm_bindgen::JsCast;
use web_sys::{FormData, HtmlFormElement, SubmitEvent};

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <AutoReload options=options.clone()/>
                <HydrationScripts options/>
                <meta name="color-scheme" content="dark light"/>
                <link rel="shortcut icon" type="image/ico" href="/favicon.ico"/>
                <link rel="stylesheet" id="leptos" href="/pkg/server_fns_axum.css"/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    view! {
        <header>
            <h1>"Server Function Demo"</h1>
        </header>
        <main>
            <HomePage/>
        </main>
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
        <PostcardExample/>
        <FileUpload/>
        <FileUploadWithProgress/>
        <FileWatcher/>
        <CustomEncoding/>
        <CustomClientExample/>
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
    let (shout_result, set_shout_result) = signal("Click me".to_string());

    view! {
        <h3>Using <code>spawn_local</code></h3>
        <p>
            "You can call a server function by using " <code>"spawn_local"</code>
            " in an event listener. "
            "Clicking this button should alert with the uppercase version of the input."
        </p>
        <input node_ref=input_ref placeholder="Type something here."/>
        <button on:click=move |_| {
            let value = input_ref.get().unwrap().value();
            spawn_local(async move {
                let uppercase_text = shouting_text(value).await.unwrap_or_else(|e| e.to_string());
                set_shout_result.set(uppercase_text);
            });
        }>

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
    let action = ServerAction::<AddRow>::new();

    // this resource will hold the total number of rows
    // passing it action.version() means it will refetch whenever the action resolves successfully
    let row_count =
        Resource::new(move || action.version().get(), |_| get_rows());

    view! {
        <h3>Using <code>Action::new</code></h3>
        <p>
            "Some server functions are conceptually \"mutations,\", which change something on the server. "
            "These often work well as actions."
        </p>
        <input node_ref=input_ref placeholder="Type something here."/>
        <button on:click=move |_| {
            let text = input_ref.get().unwrap().value();
            action.dispatch(text.into());
        }>

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
    let action = ServerAction::<AddRow>::new();
    let row_count =
        Resource::new(move || action.version().get(), |_| get_rows());

    view! {
        <h3>Using <code>"<ActionForm/>"</code></h3>
        <p>
            <code>"<ActionForm/>"</code>
            "lets you use an HTML "
            <code>"<form>"</code>
            "to call a server function in a way that gracefully degrades."
        </p>
        <ActionForm action>
            <input
                // the `name` of the input corresponds to the argument name
                name="text"
                placeholder="Type something here."
            />
            <button>Submit</button>
        </ActionForm>
        <p>You submitted: {move || format!("{:?}", action.input().get())}</p>
        <p>The result was: {move || format!("{:?}", action.value().get())}</p>
        <Transition>
            archive underaligned: need alignment 4 but have alignment 1
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
    let (result, set_result) = signal(0);

    view! {
        <h3>Custom arguments to the <code>#[server]</code> " macro"</h3>
        <p>This example shows how to specify additional behavior, including:</p>
        <ul>
            <li>Specific server function <strong>paths</strong></li>
            <li>Mixing and matching input and output <strong>encodings</strong></li>
            <li>Adding custom <strong>middleware</strong> on a per-server-fn basis</li>
        </ul>
        <input node_ref=input_ref placeholder="Type something here."/>
        <button on:click=move |_| {
            let value = input_ref.get().unwrap().value();
            spawn_local(async move {
                let length = length_of_input(value).await.unwrap_or(0);
                set_result.set(length);
            });
        }>

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
    let (input, set_input) = signal(String::new());
    let rkyv_result = Resource::new(move || input.get(), rkyv_example);

    view! {
        <h3>Using <code>rkyv</code> encoding</h3>
        <input node_ref=input_ref placeholder="Type something here."/>
        <button on:click=move |_| {
            let value = input_ref.get().unwrap().value();
            set_input.set(value);
        }>

            Click to capitalize
        </button>
        <p>{input}</p>
        <Transition>{rkyv_result}</Transition>
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

    let upload_action = Action::new_local(|data: &FormData| {
        // `MultipartData` implements `From<FormData>`
        file_length(data.clone().into())
    });

    view! {
        <h3>File Upload</h3>
        <p>Uploading files is fairly easy using multipart form data.</p>
        <form on:submit=move |ev: SubmitEvent| {
            ev.prevent_default();
            let target = ev.target().unwrap().unchecked_into::<HtmlFormElement>();
            let form_data = FormData::new_with_form(&target).unwrap();
            upload_action.dispatch_local(form_data);
        }>
            <input type="file" name="file_to_upload"/>
            <input type="submit"/>
        </form>
        <p>
            {move || {
                if upload_action.input_local().read().is_none() && upload_action.value().read().is_none()
                {
                    "Upload a file.".to_string()
                } else if upload_action.pending().get() {
                    "Uploading...".to_string()
                } else if let Some(Ok(value)) = upload_action.value().get() {
                    value.to_string()
                } else {
                    format!("{:?}", upload_action.value().get())
                }
            }}

        </p>
    }
}

/// This component uses server functions to upload a file, while streaming updates on the upload
/// progress.
#[component]
pub fn FileUploadWithProgress() -> impl IntoView {
    /// In theory, you could create a single server function which
    /// 1) received multipart form data
    /// 2) returned a stream that contained updates on the progress
    ///
    /// In reality, browsers do not actually support duplexing requests in this way. In other
    /// words, every existing browser actually requires that the request stream be complete before
    /// it begins processing the response stream.
    ///
    /// Instead, we can create two separate server functions:
    /// 1) one that receives multipart form data and begins processing the upload
    /// 2) a second that returns a stream of updates on the progress
    ///
    /// This requires us to store some global state of all the uploads. In a real app, you probably
    /// shouldn't do exactly what I'm doing here in the demo. For example, this map just
    /// distinguishes between files by filename, not by user.
    #[cfg(feature = "ssr")]
    mod progress {
        use async_broadcast::{broadcast, Receiver, Sender};
        use dashmap::DashMap;
        use futures::Stream;
        use once_cell::sync::Lazy;

        struct File {
            total: usize,
            tx: Sender<usize>,
            rx: Receiver<usize>,
        }

        static FILES: Lazy<DashMap<String, File>> = Lazy::new(DashMap::new);

        pub async fn add_chunk(filename: &str, len: usize) {
            println!("[{filename}]\tadding {len}");
            let mut entry =
                FILES.entry(filename.to_string()).or_insert_with(|| {
                    println!("[{filename}]\tinserting channel");
                    let (tx, rx) = broadcast(128);
                    File { total: 0, tx, rx }
                });
            entry.total += len;
            let new_total = entry.total;

            // we're about to do an async broadcast, so we don't want to hold a lock across it
            let tx = entry.tx.clone();
            drop(entry);

            // now we send the message and don't have to worry about it
            tx.broadcast(new_total)
                .await
                .expect("couldn't send a message over channel");
        }

        pub fn for_file(filename: &str) -> impl Stream<Item = usize> {
            let entry =
                FILES.entry(filename.to_string()).or_insert_with(|| {
                    println!("[{filename}]\tinserting channel");
                    let (tx, rx) = broadcast(128);
                    File { total: 0, tx, rx }
                });
            entry.rx.clone()
        }
    }

    #[server(
        input = MultipartFormData,
    )]
    pub async fn upload_file(data: MultipartData) -> Result<(), ServerFnError> {
        let mut data = data.into_inner().unwrap();

        while let Ok(Some(mut field)) = data.next_field().await {
            let name =
                field.file_name().expect("no filename on field").to_string();
            while let Ok(Some(chunk)) = field.chunk().await {
                let len = chunk.len();
                println!("[{name}]\t{len}");
                progress::add_chunk(&name, len).await;
                // in a real server function, you'd do something like saving the file here
            }
        }

        Ok(())
    }

    #[server(output = StreamingText)]
    pub async fn file_progress(
        filename: String,
    ) -> Result<TextStream, ServerFnError> {
        println!("getting progress on {filename}");
        // get the stream of current length for the file
        let progress = progress::for_file(&filename);
        // separate each number with a newline
        // the HTTP response might pack multiple lines of this into a single chunk
        // we need some way of dividing them up
        let progress = progress.map(|bytes| Ok(format!("{bytes}\n")));
        Ok(TextStream::new(progress))
    }

    let (filename, set_filename) = signal(None);
    let (max, set_max) = signal(None);
    let (current, set_current) = signal(None);
    let on_submit = move |ev: SubmitEvent| {
        ev.prevent_default();
        let target = ev.target().unwrap().unchecked_into::<HtmlFormElement>();
        let form_data = FormData::new_with_form(&target).unwrap();
        let file = form_data
            .get("file_to_upload")
            .unchecked_into::<web_sys::File>();
        let filename = file.name();
        let size = file.size() as usize;
        set_filename.set(Some(filename.clone()));
        set_max.set(Some(size));
        set_current.set(None);

        spawn_local(async move {
            let mut progress = file_progress(filename)
                .await
                .expect("couldn't initialize stream")
                .into_inner();
            while let Some(Ok(len)) = progress.next().await {
                // the TextStream from the server function will be a series of `usize` values
                // however, the response itself may pack those chunks into a smaller number of
                // chunks, each with more text in it
                // so we've padded them with newspace, and will split them out here
                // each value is the latest total, so we'll just take the last one
                let len = len
                    .split('\n')
                    .filter(|n| !n.is_empty())
                    .last()
                    .expect(
                        "expected at least one non-empty value from \
                         newline-delimited rows",
                    )
                    .parse::<usize>()
                    .expect("invalid length");
                set_current.set(Some(len));
            }
        });
        spawn_local(async move {
            upload_file(form_data.into())
                .await
                .expect("couldn't upload file");
        });
    };

    view! {
        <h3>File Upload with Progress</h3>
        <p>A file upload with progress can be handled with two separate server functions.</p>
        <aside>See the doc comment on the component for an explanation.</aside>
        <form on:submit=on_submit>
            <input type="file" name="file_to_upload"/>
            <input type="submit"/>
        </form>
        {move || filename.get().map(|filename| view! { <p>Uploading {filename}</p> })}
        {move || {
            max.get()
                .map(|max| {
                    view! {
                        <progress
                            max=max
                            value=move || current.get().unwrap_or_default()
                        ></progress>
                    }
                })
        }}
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

    let (files, set_files) = signal(Vec::new());

    Effect::new(move |_| {
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
            {move || {
                files
                    .get()
                    .into_iter()
                    .map(|file| {
                        view! {
                            <li>
                                <code>{file}</code>
                            </li>
                        }
                    })
                    .collect::<Vec<_>>()
            }}

        </ul>
        <p>
            <em>
                Add or remove some text files in the <code>watched_files</code>
                directory and see the list of changes here.
            </em>
        </p>
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
    let (result, set_result) = signal(None);

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
        <button on:click=move |_| {
            let value = input_ref.get().unwrap().value();
            spawn_local(async move {
                let data = ascii_uppercase(value).await;
                set_result.set(Some(data));
            });
        }>

            "Submit"
        </button>
        <p>{move || format!("{:?}", result.get())}</p>
    }
}

/// Server function encodings are just types that implement a few traits.
/// This means that you can implement your own encodings, by implementing those traits!
///
/// Here, we'll create a custom encoding that serializes and deserializes the server fn
/// using TOML. Why would you ever want to do this? I don't know, but you can!
pub struct Toml;

/// A newtype wrapper around server fn data that will be TOML-encoded.
///
/// This is needed because of Rust rules around implementing foreign traits for foreign types.
/// It will be fed into the `custom = ` argument to the server fn below.
#[derive(Serialize, Deserialize)]
pub struct TomlEncoded<T>(T);

impl Encoding for Toml {
    const CONTENT_TYPE: &'static str = "application/toml";
    const METHOD: Method = Method::POST;
}

impl<T, Request, Err> IntoReq<Toml, Request, Err> for TomlEncoded<T>
where
    Request: ClientReq<Err>,
    T: Serialize,
{
    fn into_req(
        self,
        path: &str,
        accepts: &str,
    ) -> Result<Request, ServerFnError<Err>> {
        let data = toml::to_string(&self.0)
            .map_err(|e| ServerFnError::Serialization(e.to_string()))?;
        Request::try_new_post(path, Toml::CONTENT_TYPE, accepts, data)
    }
}

impl<T, Request, Err> FromReq<Toml, Request, Err> for TomlEncoded<T>
where
    Request: Req<Err> + Send,
    T: DeserializeOwned,
{
    async fn from_req(req: Request) -> Result<Self, ServerFnError<Err>> {
        let string_data = req.try_into_string().await?;
        toml::from_str::<T>(&string_data)
            .map(TomlEncoded)
            .map_err(|e| ServerFnError::Args(e.to_string()))
    }
}

impl<T, Response, Err> IntoRes<Toml, Response, Err> for TomlEncoded<T>
where
    Response: Res<Err>,
    T: Serialize + Send,
{
    async fn into_res(self) -> Result<Response, ServerFnError<Err>> {
        let data = toml::to_string(&self.0)
            .map_err(|e| ServerFnError::Serialization(e.to_string()))?;
        Response::try_from_string(Toml::CONTENT_TYPE, data)
    }
}

impl<T, Response, Err> FromRes<Toml, Response, Err> for TomlEncoded<T>
where
    Response: ClientRes<Err> + Send,
    T: DeserializeOwned,
{
    async fn from_res(res: Response) -> Result<Self, ServerFnError<Err>> {
        let data = res.try_into_string().await?;
        toml::from_str(&data)
            .map(TomlEncoded)
            .map_err(|e| ServerFnError::Deserialization(e.to_string()))
    }
}

#[derive(Serialize, Deserialize)]
pub struct WhyNotResult {
    original: String,
    modified: String,
}

#[server(
    input = Toml,
    output = Toml,
    custom = TomlEncoded
)]
pub async fn why_not(
    original: String,
    addition: String,
) -> Result<TomlEncoded<WhyNotResult>, ServerFnError> {
    // insert a simulated wait
    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    Ok(TomlEncoded(WhyNotResult {
        modified: format!("{original}{addition}"),
        original,
    }))
}

#[component]
pub fn CustomEncoding() -> impl IntoView {
    let input_ref = NodeRef::<Input>::new();
    let (result, set_result) = signal("foo".to_string());

    view! {
        <h3>Custom encodings</h3>
        <p>
            "This example creates a custom encoding that sends server fn data using TOML. Why? Well... why not?"
        </p>
        <input node_ref=input_ref placeholder="Type something here."/>
        <button on:click=move |_| {
            let value = input_ref.get().unwrap().value();
            spawn_local(async move {
                let new_value = why_not(value, ", but in TOML!!!".to_string()).await.unwrap();
                set_result.set(new_value.0.modified);
            });
        }>

            Submit
        </button>
        <p>{result}</p>
    }
}

/// Middleware lets you modify the request/response on the server.
///
/// On the client, you might also want to modify the request. For example, you may need to add a
/// custom header for authentication on every request. You can do this by creating a "custom
/// client."
#[component]
pub fn CustomClientExample() -> impl IntoView {
    // Define a type for our client.
    pub struct CustomClient;

    // Implement the `Client` trait for it.
    impl<CustErr> Client<CustErr> for CustomClient {
        // BrowserRequest and BrowserResponse are the defaults used by other server functions.
        // They are wrappers for the underlying Web Fetch API types.
        type Request = BrowserRequest;
        type Response = BrowserResponse;

        // Our custom `send()` implementation does all the work.
        fn send(
            req: Self::Request,
        ) -> impl Future<Output = Result<Self::Response, ServerFnError<CustErr>>>
               + Send {
            // BrowserRequest derefs to the underlying Request type from gloo-net,
            // so we can get access to the headers here
            let headers = req.headers();
            // modify the headers by appending one
            headers.append("X-Custom-Header", "foobar");
            // delegate back out to BrowserClient to send the modified request
            BrowserClient::send(req)
        }
    }

    // Specify our custom client with `client = `
    #[server(client = CustomClient)]
    pub async fn fn_with_custom_client() -> Result<(), ServerFnError> {
        use http::header::HeaderMap;
        use leptos_axum::extract;

        let headers: HeaderMap = extract().await?;
        let custom_header = headers.get("X-Custom-Header");
        println!("X-Custom-Header = {custom_header:?}");
        Ok(())
    }

    view! {
        <h3>Custom clients</h3>
        <p>
            You can define a custom server function client to do something like adding a header to every request.
        </p>
        <p>
            Check the network request in your browser devtools to see how this client adds a custom header.
        </p>
        <button on:click=|_| spawn_local(async {
            fn_with_custom_client().await.unwrap()
        })>Click me</button>
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct PostcardData {
    name: String,
    age: u32,
    hobbies: Vec<String>,
}

/// This server function uses Postcard for both input and output encoding.
/// Postcard provides efficient binary serialization, almost as fast as rkyv, while also being
/// serde compatible
#[server(input = Postcard, output = Postcard)]
pub async fn postcard_example(
    data: PostcardData,
) -> Result<PostcardData, ServerFnError> {
    // Simulate some processing time
    tokio::time::sleep(std::time::Duration::from_millis(250)).await;

    // Modify the data to demonstrate server-side changes
    let mut modified_data = data.clone();
    modified_data.age += 1;
    modified_data.hobbies.push("Rust programming".to_string());

    Ok(modified_data)
}

/// This component demonstrates the usage of Postcard encoding with server functions.
/// It allows incrementing the age of a person and shows how the data is
/// serialized, sent to the server, processed, and returned.
#[component]
pub fn PostcardExample() -> impl IntoView {
    // Initialize the input data
    let (input, set_input) = signal(PostcardData {
        name: "Alice".to_string(),
        age: 30,
        hobbies: vec!["reading".to_string(), "hiking".to_string()],
    });

    // Create a resource that will call the server function whenever the input changes
    let postcard_result = Resource::new(
        move || input.get(),
        |data| async move { postcard_example(data).await },
    );

    view! {
        <h3>Using <code>postcard</code> encoding</h3>
        <p>"This example demonstrates using Postcard for efficient binary serialization."</p>
        <button on:click=move |_| {
            // Update the input data when the button is clicked
            set_input.update(|data| {
                data.age += 1;
            });
        }>
            "Increment Age"
        </button>
        // Display the current input data
        <p>"Input: " {move || format!("{:?}", input.get())}</p>
        <Transition>
            // Display the result from the server, which will update automatically
            // when the input changes due to the resource
            <p>"Result: " {move || postcard_result.get().map(|r| format!("{:?}", r))}</p>
        </Transition>
    }
}
