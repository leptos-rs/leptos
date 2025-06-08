use leptos::prelude::*;

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <AutoReload options=options.clone() />
                <HydrationScripts options />
                <meta name="color-scheme" content="dark light" />
                <link rel="shortcut icon" type="image/ico" href="/favicon.ico" />
                <link rel="stylesheet" id="leptos" href="/pkg/server_fns_axum.css" />
            </head>
            <body>
                <App />
            </body>
        </html>
    }
}

#[server]
pub async fn fetch_string() -> Result<String, ServerFnError> {
    Ok("Hello, world!".to_string())
}

#[component]
pub fn App() -> impl IntoView {
    let code = Resource::new(|| (), |_| fetch_string());
    view! {
        <header>
            <h1>"Server Function Demo"</h1>
        </header>
        <main>
            <Suspense fallback=move || view! { <p>"Loading code example..."</p> }>
                {move || Suspend::new(async move {
                    view! {
                        <p>{code.await}</p>
                    }
                })}
            </Suspense>
        </main>
    }
}
