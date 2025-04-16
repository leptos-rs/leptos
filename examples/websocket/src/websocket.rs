use leptos::{prelude::*, task::spawn_local};
use server_fn::{codec::JsonEncoding, BoxedStream, ServerFnError, Websocket};

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <AutoReload options=options.clone() />
                <HydrationScripts options />
                <link rel="shortcut icon" type="image/ico" href="/favicon.ico" />
            </head>
            <body>
                <App />
            </body>
        </html>
    }
}

// The websocket protocol can be used on any server function that accepts and returns a [`BoxedStream`]
// with items that can be encoded by the input and output encoding generics.
//
// In this case, the input and output encodings are [`Json`] and [`Json`], respectively which requires
// the items to implement [`Serialize`] and [`Deserialize`].
#[server(protocol = Websocket<JsonEncoding, JsonEncoding>)]
async fn echo_websocket(
    input: BoxedStream<String, ServerFnError>,
) -> Result<BoxedStream<String, ServerFnError>, ServerFnError> {
    use futures::{channel::mpsc, SinkExt, StreamExt};
    let mut input = input; // FIXME :-) server fn fields should pass mut through to destructure

    // create a channel of outgoing websocket messages
    // we'll return rx, so sending a message to tx will send a message to the client via the websocket
    let (mut tx, rx) = mpsc::channel(1);

    // spawn a task to listen to the input stream of messages coming in over the websocket
    tokio::spawn(async move {
        let mut x = 0;
        while let Some(msg) = input.next().await {
            // do some work on each message, and then send our responses
            x += 1;
            println!("In server: {} {:?}", x, msg);
            if x % 3 == 0 {
                let _ = tx
                    .send(Err(ServerFnError::Registration(
                        "Error generated from server".to_string(),
                    )))
                    .await;
            } else {
                let _ = tx.send(msg.map(|msg| msg.to_ascii_uppercase())).await;
            }
        }
    });

    Ok(rx.into())
}

#[component]
pub fn App() -> impl IntoView {
    use futures::{channel::mpsc, StreamExt};
    let (mut tx, rx) = mpsc::channel(1);
    let latest = RwSignal::new(Ok("".into()));

    // we'll only listen for websocket messages on the client
    if cfg!(feature = "hydrate") {
        spawn_local(async move {
            match echo_websocket(rx.into()).await {
                Ok(mut messages) => {
                    while let Some(msg) = messages.next().await {
                        leptos::logging::log!("{:?}", msg);
                        latest.set(msg);
                    }
                }
                Err(e) => leptos::logging::warn!("{e}"),
            }
        });
    }

    let mut x = 0;
    view! {
        <h1>Simple Echo WebSocket Communication</h1>
        <input
            type="text"
            on:input:target=move |ev| {
                x += 1;
                let msg = ev.target().value();
                leptos::logging::log!("In client: {} {:?}", x, msg);
                if x % 5 == 0 {
                    let _ = tx
                        .try_send(
                            Err(
                                ServerFnError::Registration(
                                    "Error generated from client".to_string(),
                                ),
                            ),
                        );
                } else {
                    let _ = tx.try_send(Ok(msg));
                }
            }
        />
        <div>
            <ErrorBoundary fallback=|errors| {
                view! {
                    <p>
                        {move || {
                            errors
                                .get()
                                .into_iter()
                                .map(|(_, e)| format!("{e:?}"))
                                .collect::<Vec<String>>()
                                .join(" ")
                        }}
                    </p>
                }
            }>
                <p>{latest}</p>
            </ErrorBoundary>
        </div>
    }
}
