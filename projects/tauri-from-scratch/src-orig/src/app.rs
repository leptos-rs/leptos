use leptos::prelude::*;

#[cfg(feature = "ssr")]
pub fn shell(options: LeptosOptions) -> impl IntoView {
    use leptos_meta::MetaTags;
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <AutoReload options=options.clone() />
                <HydrationScripts options/>
                <MetaTags/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

#[server(endpoint = "hello_world")]
pub async fn hello_world_server() -> Result<String, ServerFnError> {
    Ok("Hey.".to_string())
}

#[component]
pub fn App() -> impl IntoView {
    let action = ServerAction::<HelloWorldServer>::new();
    let vals = RwSignal::new(String::new());
    Effect::new(move |_| {
        if let Some(resp) = action.value().get() {
            match resp {
                Ok(val) => vals.set(val),
                Err(err) => vals.set(format!("{err:?}")),
            }
        }
    });

    view! {
        <button
            on:click=move |_| {
                action.dispatch(HelloWorldServer{});
            }
        >"Hello world."</button>
        <br/><br/>
        <span>"Server says: "</span>
        {move || vals.get()}
    }
}
