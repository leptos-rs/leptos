use cfg_if::cfg_if;
use leptos::leptos_dom::IntoView;
use leptos::*;
use typed_builder::TypedBuilder;

#[cfg(any(feature = "csr", feature = "hydrate"))]
use wasm_bindgen::JsCast;

use crate::{use_location, use_resolved_path, State};

/// Describes a value that is either a static or a reactive URL, i.e.,
/// a [String], a [&str], or a reactive `Fn() -> String`.
pub trait ToHref {
    /// Converts the (static or reactive) URL into a function that can be called to
    /// return the URL.
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
pub struct AProps<H>
where
    H: ToHref + 'static,
{
    /// Used to calculate the link's `href` attribute. Will be resolved relative
    /// to the current route.
    pub href: H,
    /// If `true`, the link is marked active when the location matches exactly;
    /// if false, link is marked active if the current route starts with it.
    #[builder(default)]
    pub exact: bool,
    /// An object of any type that will be pushed to router state
    #[builder(default, setter(strip_option))]
    pub state: Option<State>,
    /// If `true`, the link will not add to the browser's history (so, pressing `Back`
    /// will skip this page.)
    #[builder(default)]
    pub replace: bool,
    /// The nodes or elements to be shown inside the link.
    pub children: Box<dyn Fn() -> Fragment>
}

/// An HTML [`a`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/a)
/// progressively enhanced to use client-side routing.
#[allow(non_snake_case)]
pub fn A<H>(cx: Scope, props: AProps<H>) -> impl IntoView
where
    H: ToHref + 'static,
{
    let location = use_location(cx);
    let href = use_resolved_path(cx, move || props.href.to_href()());
    let is_active = create_memo(cx, move |_| match href.get() {
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

    Component::new("A", move |cx| {
        cfg_if! {
            if #[cfg(any(feature = "csr", feature = "hydrate"))] {
                view! { cx,
                    <a
                        href=move || href.get().unwrap_or_default()
                        prop:state={props.state.map(|s| s.to_js_value())}
                        prop:replace={props.replace}
                        aria-current=move || if is_active.get() { Some("page") } else { None }
                    >
                        {props.children}
                    </a>
                }
            } else {
                view! { cx,
                    <a
                        href=move || href().unwrap_or_default()
                        aria-current=move || if is_active() { Some("page") } else { None }
                    >
                        {props.children}
                    </a>
                }
            }
        }
    }).into_view(cx)
}
