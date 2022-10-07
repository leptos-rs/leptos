use crate::use_route;
use leptos::*;

#[component]
pub fn Outlet(cx: Scope) -> Child {
    let route = use_route(cx);
    if let Some(child) = route.child() {
        provide_context(child.cx(), child.clone());
        child.outlet().into_child(child.cx())
    } else {
        String::new().into_child(cx)
    }
}
