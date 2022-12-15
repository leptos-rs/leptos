use std::{cell::RefCell, rc::Rc};

use crate::use_route;
use leptos::*;

/// Displays the child route nested in a parent route, allowing you to control exactly where
/// that child route is displayed. Renders nothing if there is no nested child.
#[component]
pub fn Outlet(cx: Scope) -> impl IntoView {
    let route = use_route(cx);
    let is_showing = Rc::new(RefCell::new(None));
    let (outlet, set_outlet) = create_signal(cx, None);
    create_effect(cx, move |_| {
        let is_showing_val = { is_showing.borrow().clone() };
        let child = route.child();
        match (route.child(), &is_showing_val) {
            (None, _) => {
                set_outlet.set(None);
            }
            (Some(child), Some(path))
                if Some(child.original_path().to_string()) == is_showing_val =>
            {
                // do nothing: we don't need to rerender the component, because it's the same
            }
            (Some(child), _) => {
                *is_showing.borrow_mut() = Some(child.original_path().to_string());
                provide_context(child.cx(), child.clone());
                set_outlet.set(Some(child.outlet().into_view(cx)))
            }
        }
    });

    move || outlet.get()
}
