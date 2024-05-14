use leptos::prelude::*;

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
