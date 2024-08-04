use leptos::prelude::*;

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <AutoReload options=options.clone()/>
                <HydrationScripts options=options islands=true/>
                <link rel="stylesheet" id="leptos" href="/pkg/islands.css"/>
                <link rel="shortcut icon" type="image/ico" href="/favicon.ico"/>
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
            <h1>"My Application"</h1>
        </header>
        <main>
            <OuterIsland>
                <InnerIsland/>
                <InnerIsland/>
                <InnerIsland/>
            </OuterIsland>
        </main>
    }
}

#[island]
pub fn OuterIsland(children: Children) -> impl IntoView {
    provide_context(42i32);
    view! {
        <div class="outer-island">
            <h2>"Outer Island"</h2>
            <button on:click=|_| leptos::logging::log!("clicked button in island!")>
                "Click me"
            </button>
            {children()}
        </div>
    }
}

#[island]
pub fn InnerIsland() -> impl IntoView {
    let val = use_context::<i32>();
    view! {
        <h2>"Inner Island"</h2>
        <button on:click=move |_| leptos::logging::log!("value should be Some(42) -- it's {val:?}")>
            "Click me (inner)"
        </button>
    }
}
