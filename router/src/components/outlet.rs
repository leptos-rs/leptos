use crate::use_route;
use leptos::*;

/// Displays the child route nested in a parent route, allowing you to control exactly where
/// that child route is displayed. Renders nothing if there is no nested child.
#[component]
pub fn Outlet(cx: Scope) -> Child {
    let route = use_route(cx);
    (move || {
        route.child().map(|child| {
            provide_context(child.cx(), child.clone());
            child.outlet()
        })
    })
    .into_child(cx)
}
