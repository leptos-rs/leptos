use leptos::prelude::*;
#[allow(unused_imports)]
use leptos_router::{
    components::Route, path, MatchNestedRoutes, NavigateOptions,
};

#[component]
pub fn Routes4217() -> impl MatchNestedRoutes + Clone {
    view! {
        <Route path=path!("4217") view=Issue4217/>
    }
    .into_inner()
}

#[component]
fn Issue4217() -> impl IntoView {
    view! {
        <select multiple=true>
            <option id="option1" value="1" selected>"Option 1"</option>
            <option id="option2" value="2" selected>"Option 2"</option>
            <option id="option3" value="3" selected>"Option 3"</option>
        </select>
    }
}
