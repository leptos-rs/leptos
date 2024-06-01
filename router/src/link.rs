use crate::{
    components::RouterContext, hooks::use_resolved_path, location::State,
};
use leptos::{children::Children, oco::Oco, prelude::*, *};
use reactive_graph::{computed::ArcMemo, owner::use_context};
use std::borrow::Cow;

/// Describes a value that is either a static or a reactive URL, i.e.,
/// a [`String`], a [`&str`], or a reactive `Fn() -> String`.
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

impl ToHref for Cow<'_, str> {
    fn to_href(&self) -> Box<dyn Fn() -> String + '_> {
        let s = self.to_string();
        Box::new(move || s.clone())
    }
}

impl ToHref for Oco<'_, str> {
    fn to_href(&self) -> Box<dyn Fn() -> String + '_> {
        let s = self.to_string();
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
///
/// Client-side routing also works with ordinary HTML `<a>` tags, but `<A>` does two additional things:
/// 1) Correctly resolves relative nested routes. Relative routing with ordinary `<a>` tags can be tricky.
///    For example, if you have a route like `/post/:id`, `<A href="1">` will generate the correct relative
///    route, but `<a href="1">` likely will not (depending on where it appears in your view.)
/// 2) Sets the `aria-current` attribute if this link is the active link (i.e., it’s a link to the page you’re on).
///    This is helpful for accessibility and for styling. For example, maybe you want to set the link a
///    different color if it’s a link to the page you’re currently on.
#[component]
pub fn A<H>(
    /// Used to calculate the link's `href` attribute. Will be resolved relative
    /// to the current route.
    href: H,
    /// Where to display the linked URL, as the name for a browsing context (a tab, window, or `<iframe>`).
    #[prop(optional, into)]
    target: Option<Oco<'static, str>>,
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
    /// The nodes or elements to be shown inside the link.
    children: Children,
) -> impl IntoView
where
    H: ToHref + Send + Sync + 'static,
{
    fn inner(
        href: ArcMemo<Option<String>>,
        target: Option<Oco<'static, str>>,
        exact: bool,
        #[allow(unused)] state: Option<State>,
        #[allow(unused)] replace: bool,
        children: Children,
    ) -> impl IntoView {
        let RouterContext { current_url, .. } =
            use_context().expect("tried to use <A/> outside a <Router/>.");
        let is_active = ArcMemo::new({
            let href = href.clone();
            move |_| {
                href.read().as_deref().is_some_and(|to| {
                    let path = to.split(['?', '#']).next().unwrap_or_default();
                    current_url.with(|loc| {
                        let loc = loc.path();
                        if exact {
                            loc == path
                        } else {
                            std::iter::zip(loc.split('/'), path.split('/'))
                                .all(|(loc_p, path_p)| loc_p == path_p)
                        }
                    })
                })
            }
        });

        view! {
            <a
                href=move || href.get().unwrap_or_default()
                target=target
                prop:state=state.map(|s| s.to_js_value())
                prop:replace=replace
                aria-current={
                    let is_active = is_active.clone();
                    move || if is_active.get() { Some("page") } else { None }
                }
            >

                {children()}
            </a>
        }
    }

    let href = use_resolved_path::<Dom>(move || href.to_href()());
    inner(href, target, exact, state, replace, children)
}
