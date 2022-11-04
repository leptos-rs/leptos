use cfg_if::cfg_if;
use leptos::leptos_dom::IntoChild;
use leptos::*;
use typed_builder::TypedBuilder;

#[cfg(not(feature = "ssr"))]
use wasm_bindgen::JsCast;

use crate::{use_location, use_resolved_path, State};

/// Describes a value that is either a static or a reactive URL, i.e.,
/// a [String], a [&str], or a reactive `Fn() -> String`.
pub trait ToHref {
    fn to_href(&self) -> Box<dyn Fn() -> String + '_>;
}

impl ToHref for &str {
    fn to_href(&self) -> Box<dyn Fn() -> String> {
        let s = self.to_string();
        Box::new(move || s.clone())
    }
}

impl ToHref for String {
    fn to_href(&self) -> Box<dyn Fn() -> String> {
        let s = self.clone();
        Box::new(move || s.clone())
    }
}

impl<F> ToHref for F
where
    F: Fn() -> String + 'static,
{
    fn to_href(&self) -> Box<dyn Fn() -> String + '_> {
        Box::new(self)
    }
}

/// Properties that can be passed to the [A] component, which is an HTML
/// [`a`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/a)
/// progressively enhanced to use client-side routing.
#[derive(TypedBuilder)]
pub struct AProps<C, H>
where
    C: IntoChild,
    H: ToHref + 'static,
{
    /// Used to calculate the link's `href` attribute. Will be resolved relative
    /// to the current route.
    href: H,
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
    children: Box<dyn Fn() -> Vec<C>>,
}

/// An HTML [`a`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/a)
/// progressively enhanced to use client-side routing.
#[allow(non_snake_case)]
pub fn A<C, H>(cx: Scope, props: AProps<C, H>) -> Element
where
    C: IntoChild,
    H: ToHref + 'static,
{
    let location = use_location(cx);
    let href = use_resolved_path(cx, move || props.href.to_href()());
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

    let mut children = (props.children)();
    if children.len() != 1 {
        debug_warn!("[Link] Pass exactly one child to <A/>. If you want to pass more than one child, nest them within an element.");
    }
    let child = children.remove(0);

    cfg_if! {
        if #[cfg(any(feature = "csr", feature = "hydrate"))] {
            view! { cx,
                <a
                    href=move || href().unwrap_or_default()
                    prop:state={props.state.map(|s| s.to_js_value())}
                    prop:replace={props.replace}
                    aria-current=move || if is_active() { Some("page") } else { None }
                >
                    {child}
                </a>
            }
        } else {
            view! { cx,
                <a
                    href=move || href().unwrap_or_default()
                    aria-current=move || if is_active() { Some("page") } else { None }
                >
                    {child}
                </a>
            }
        }
    }
}
