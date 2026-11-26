use leptos::prelude::*;
use leptos_router::{
    components::{Outlet, ParentRoute, Route},
    hooks::use_params_map,
    path, MatchNestedRoutes,
};

// https://github.com/leptos-rs/leptos/issues/4991

#[component]
pub fn Routes4991() -> impl MatchNestedRoutes + Clone {
    view! {
        <ParentRoute path=path!("4991/:section") view=Outlet>
            <Route path=path!("tags/:tag") view=Page4991/>
        </ParentRoute>
    }
    .into_inner()
}

#[component]
fn Page4991() -> impl IntoView {
    let params = use_params_map();

    view! {
        <a href="/4991/x%2541/tags/c%2526d">"other tag"</a>
        <p id="section">{move || format!("{:?}", params.read().get("section"))}</p>
        <p id="tag">{move || format!("{:?}", params.read().get("tag"))}</p>
    }
}
