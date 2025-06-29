use leptos::{context::Provider, prelude::*};
use leptos_router::{
    components::{ParentRoute, Route},
    nested_router::Outlet,
    path,
};

#[component]
pub fn Routes4015() -> impl leptos_router::MatchNestedRoutes + Clone {
    view! {
        <ParentRoute path=path!("4015") view=|| view! {
            <Provider value=42i32>
                <Outlet/>
            </Provider>
        }>
            <Route path=path!("") view=Child/>
        </ParentRoute>
    }
    .into_inner()
}

#[component]
fn Child() -> impl IntoView {
    let value = use_context::<i32>();

    view! {
        <p id="result">{format!("{value:?}")}</p>
    }
}
