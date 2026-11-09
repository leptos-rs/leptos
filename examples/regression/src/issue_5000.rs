use leptos::prelude::*;
use leptos_router::{
    components::{Outlet, ParentRoute, Route},
    hooks::use_location,
    path, MatchNestedRoutes,
};

// https://github.com/leptos-rs/leptos/issues/5000

#[component]
pub fn Routes5000() -> impl MatchNestedRoutes + Clone {
    view! {
        <ParentRoute path=path!("5000") view=Outlet>
            <Route path=path!("") view=Links5000/>
            <Route path=path!(":id") view=Item5000/>
        </ParentRoute>
    }
    .into_inner()
}

#[component]
fn Links5000() -> impl IntoView {
    view! {
        <a href="/5000/x%2541">"x%2541"</a>
        <a href="/5000/a%5Cb">"a%5Cb"</a>
    }
}

#[component]
fn Item5000() -> impl IntoView {
    let pathname = use_location().pathname;

    view! {
        <p id="pathname">{move || pathname.get()}</p>
        <a href="/5000">"back to links"</a>
    }
}
