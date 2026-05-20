use leptos::prelude::*;
#[allow(unused_imports)]
use leptos_router::{
    components::Route, path, Lazy, MatchNestedRoutes, NavigateOptions,
};

#[component]
pub fn Routes4324() -> impl MatchNestedRoutes + Clone {
    view! {
        <Route path=path!("4324") view=Issue4324/>
    }
    .into_inner()
}

#[component]
pub fn Issue4324() -> impl IntoView {
    view! {
        <a href="/4324/">"This page"</a>
        <p id="result">"Issue4324"</p>
    }
}
