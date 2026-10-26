use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::{components::Route, path, MatchNestedRoutes};

// https://github.com/leptos-rs/leptos/issues/4968

#[component]
pub fn Routes4968() -> impl MatchNestedRoutes + Clone {
    view! { <Route path=path!("4968") view=Page4968/> }.into_inner()
}

#[component]
fn Page4968() -> impl IntoView {
    let n = RwSignal::new(0);
    let shown = RwSignal::new(true);
    let m = RwSignal::new(0);
    let shown_reactive = RwSignal::new(false);

    view! {
        <Title text="Outer"/>
        <p id="index-marker">"Issue 4968"</p>
        <button id="rename" on:click=move |_| n.update(|n| *n += 1)>"Rename"</button>
        <button id="toggle" on:click=move |_| shown.update(|s| *s = !*s)>"Toggle"</button>
        <Show when=move || shown.get()>
            {move || {
                let n = n.get();
                view! { <Title text=format!("Inner {n}")/> }
            }}
        </Show>
        <button id="rename-reactive" on:click=move |_| m.update(|m| *m += 1)>
            "Rename reactive"
        </button>
        <button id="toggle-reactive" on:click=move |_| shown_reactive.update(|s| *s = !*s)>
            "Toggle reactive"
        </button>
        <Show when=move || shown_reactive.get()>
            {move || {
                let label = RwSignal::new(format!("Reactive {}", m.get()));
                view! { <Title text=move || label.get()/> }
            }}
        </Show>
    }
}
