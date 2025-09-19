use leptos::prelude::*;
#[allow(unused_imports)]
use leptos_router::{
    components::Route, path, MatchNestedRoutes, NavigateOptions,
};

#[component]
pub fn Routes4005() -> impl MatchNestedRoutes + Clone {
    view! {
        <Route path=path!("4005") view=Issue4005/>
    }
    .into_inner()
}

#[component]
fn Issue4005() -> impl IntoView {
    view! {
        <select id="select" prop:value="2">
            <option value="1">"Option 1"</option>
            <option value="2">"Option 2"</option>
            <option value="3">"Option 3"</option>
        </select>
    }
}
