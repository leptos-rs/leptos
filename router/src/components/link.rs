use leptos_dom as leptos;
use leptos_dom::*;
use leptos_macro::view;
use leptos_reactive::{create_effect, create_memo, Scope};
use typed_builder::TypedBuilder;
use wasm_bindgen::JsCast;

use crate::{use_location, use_resolved_path, State};

#[derive(TypedBuilder)]
pub struct LinkProps<C>
where
    C: IntoChild,
{
    // <Link/> props
    /// Used to calculate the link's `href` attribute. Will be resolved relative
    /// to the current route.
    to: String,
    /// An object of any type that will be pushed to router state
    #[builder(default, setter(strip_option))]
    state: Option<State>,
    /// If `true`, the link will not add to the browser's history (so, pressing `Back`
    /// will skip this page.)
    #[builder(default)]
    replace: bool,
    #[builder(default)]
    children: Vec<C>,
}

#[allow(non_snake_case)]
pub fn Link<C>(cx: Scope, mut props: LinkProps<C>) -> Element
where
    C: IntoChild,
{
    let href = use_resolved_path(cx, move || props.to.clone());

    if props.children.len() != 1 {
        debug_warn!("[Link] Pass exactly one child to <Link/>. If you want to pass more than one child, next them within an element.");
    }
    let child = props.children.remove(0);

    view! {
        <a
            href={href().unwrap_or_default()}
            prop:state={props.state.map(|s| s.to_js_value())}
            prop:replace={props.replace}
        >
            {child}
        </a>
    }
}

#[derive(TypedBuilder)]
pub struct NavLinkProps<C>
where
    C: IntoChild,
{
    // <Link/> props
    /// Used to calculate the link's `href` attribute. Will be resolved relative
    /// to the current route.
    to: String,
    /// If `true`, the link is marked active when the location matches exactly;
    /// if false, link is marked active if the current route starts with it.
    #[builder(default)]
    exact: bool,
    /// An object of any type that will be pushed to router state
    #[builder(default, setter(strip_option))]
    state: Option<State>,
    /// If `true`, the link will not add to the browser's history (so, pressing `Back`
    /// will skip this page.)
    #[builder(default)]
    replace: bool,
    #[builder(default)]
    children: Vec<C>,
}

#[allow(non_snake_case)]
pub fn NavLink<C>(cx: Scope, mut props: NavLinkProps<C>) -> Element
where
    C: IntoChild,
{
    let location = use_location(cx);
    let href = use_resolved_path(cx, move || props.to.clone());
    let is_active = create_memo(cx, move |_| match href() {
        None => false,

        Some(to) => {
            let path = to
                .split(['?', '#'])
                .next()
                .unwrap_or_default()
                .to_lowercase();
            let loc = location.pathname.get().to_lowercase();
            if props.exact {
                loc == path
            } else {
                loc.starts_with(&path)
            }
        }
    });

    if props.children.len() != 1 {
        debug_warn!("[Link] Pass exactly one child to <Link/>. If you want to pass more than one child, next them within an element.");
    }
    let child = props.children.remove(0);

    view! {
        <a
            href={href().unwrap_or_default()}
            prop:state={props.state.map(|s| s.to_js_value())}
            prop:replace={props.replace}
            class:active={is_active}
            aria-current={move || if is_active() { Some("page") } else { None }}
        >
            {child}
        </a>
    }
}
