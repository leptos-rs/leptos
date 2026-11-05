use leptos::prelude::*;
use leptos_router::{
    components::Route,
    hooks::{query_signal, use_location},
    path, MatchNestedRoutes,
};

// https://github.com/leptos-rs/leptos/issues/4992

#[component]
pub fn Routes4992() -> impl MatchNestedRoutes + Clone {
    view! {
        <Route path=path!("4992") view=Page4992/>
    }
    .into_inner()
}

#[component]
fn Page4992() -> impl IntoView {
    let (q, set_q) = query_signal::<String>("q");
    let search = use_location().search;

    view! {
        <button id="set-q" on:click=move |_| set_q.set(Some("a%2Bb".to_string()))>
            "set q"
        </button>
        <p id="search">{move || search.get()}</p>
        <p id="value">{move || format!("{:?}", q.get())}</p>
    }
}
