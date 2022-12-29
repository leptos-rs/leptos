use cfg_if::cfg_if;
use leptos::leptos_dom::IntoView;
use leptos::*;

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

/// An HTML [`a`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/a)
/// progressively enhanced to use client-side routing.
#[component]
pub fn A<H>(
    cx: Scope,
    /// Used to calculate the link's `href` attribute. Will be resolved relative
    /// to the current route.
    href: H,
    /// If `true`, the link is marked active when the location matches exactly;
    /// if false, link is marked active if the current route starts with it.
    #[prop(optional)]
    exact: bool,
    /// An object of any type that will be pushed to router state
    #[prop(optional)]
    state: Option<State>,
    /// If `true`, the link will not add to the browser's history (so, pressing `Back`
    /// will skip this page.)
    #[prop(optional)]
    replace: bool,
    /// Sets the `class` attribute on the underlying `<a>` tag, making it easier to style.
    #[prop(optional, into)]
    class: Option<MaybeSignal<String>>,
    /// The nodes or elements to be shown inside the link.
    children: Box<dyn Fn(Scope) -> Fragment>,
) -> impl IntoView
where
    H: ToHref + 'static,
{
    let location = use_location(cx);
    let href = use_resolved_path(cx, move || href.to_href()());
    let is_active = create_memo(cx, move |_| match href.get() {
        None => false,

        Some(to) => {
            let path = to
                .split(['?', '#'])
                .next()
                .unwrap_or_default()
                .to_lowercase();
            let loc = location.pathname.get().to_lowercase();
            if exact {
                loc == path
            } else {
                loc.starts_with(&path)
            }
        }
    });

    cfg_if! {
        if #[cfg(any(feature = "csr", feature = "hydrate"))] {
            view! { cx,
                <a
                    href=move || href.get().unwrap_or_default()
                    prop:state={state.map(|s| s.to_js_value())}
                    prop:replace={replace}
                    aria-current=move || if is_active.get() { Some("page") } else { None }
                    class=move || class.as_ref().map(|class| class.get())
                >
                    {children(cx)}
                </a>
            }
        } else {
            view! { cx,
                <a
                    href=move || href.get().unwrap_or_default()
                    aria-current=move || if is_active.get() { Some("page") } else { None }
                    class=move || class.as_ref().map(|class| class.get())
                >
                    {children(cx)}
                </a>
            }
        }
    }
}
