use crate::use_route;
use leptos_core as leptos;
use leptos_dom::Child;
use leptos_dom::IntoChild;
use leptos_macro::component;
use leptos_macro::Props;
use leptos_reactive::create_effect;
use leptos_reactive::provide_context;
use leptos_reactive::Scope;

#[component]
pub fn Outlet(cx: Scope) -> Child {
    let route = use_route(cx);
    create_effect(cx, move |_| {
        log::debug!("<Outlet> RouteContext is {:#?}", use_route(cx).path())
    });
    if let Some(child) = route.child() {
        log::debug!("<Outlet> providing context {child:#?}");
        provide_context(child.cx(), child.clone());
        child.outlet().into_child(child.cx())
    } else {
        String::new().into_child(cx)
    }
}
