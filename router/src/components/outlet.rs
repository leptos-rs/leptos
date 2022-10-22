use crate::use_route;
use leptos::*;

#[component]
pub fn Outlet(cx: Scope) -> Child {
    let route = use_route(cx);
    create_memo(cx, move |_| {
        route.child().map(|child| {
            provide_context(child.cx(), child.clone());
            child.outlet().into_child(child.cx())
        })
    })
    .into_child(cx)
}
