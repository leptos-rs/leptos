use leptos::prelude::*;

#[component]
pub fn App() -> impl IntoView {
    view! {
        <header>
            <h1>"My Application"</h1>
        </header>
        <main>
            <OuterIsland/>
        </main>
    }
}

#[island]
pub fn OuterIsland() -> impl IntoView {
    view! {
        <div class="outer-island">
            <h2>"Outer Island"</h2>
            <button on:click=|_| leptos::logging::log!("clicked button in island!")>
                "Click me"
            </button>
        </div>
    }
}
