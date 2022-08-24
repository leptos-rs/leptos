use crate::use_route;
use leptos_core as leptos;
use leptos_dom::IntoChild;
use leptos_macro::component;
use leptos_macro::Props;
use leptos_reactive::Scope;

#[component]
pub fn Outlet(cx: Scope) -> impl IntoChild {
    let route = use_route(cx);
    log::debug!("trying to render Outlet: route.child = {:?}", route.child());
    move || {
        route.child().as_ref().map(|child| {
            log::debug!("rendering <Outlet/>");
            child.outlet()
        })
    }
}
