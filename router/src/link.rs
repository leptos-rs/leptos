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
                            is_active_for(path, loc)
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

    let href = use_resolved_path(move || href.to_href()());
    inner(href, target, exact, state, replace, children)
}

// Test if `href` is active for `location`.  Assumes _both_ `href` and `location` begin with a `'/'`.
fn is_active_for(href: &str, location: &str) -> bool {
    let mut href_f = href.split('/');
    // location _must_ be consumed first to avoid draining href_f early
    // also using enumerate to special case _the first two_ so that the allowance for ignoring the comparison
    // with the loc fragment on an emtpy href fragment for non root related parts.
    std::iter::zip(location.split('/'), href_f.by_ref())
        .enumerate()
        .all(|(c, (loc_p, href_p))| loc_p == href_p || href_p == "" && c > 1)
    // ensure inactive if more href fragments remain (otherwise falsely set to active when href="/item/one",
    // location="/item")
    // or it's an empty string (otherwise href="/item/" is not active for location="/item")
    && matches!(href_f.next(), None | Some(""))
}

#[cfg(test)]
mod tests {
    use super::is_active_for;

    #[test]
    fn is_active_for_matched() {
        // root
        assert!(is_active_for("/", "/"));

        // both at one level for all combinations of trailing slashes
        assert!(is_active_for("/item", "/item"));
        assert!(is_active_for("/item", "/item/"));
        assert!(is_active_for("/item/", "/item"));
        assert!(is_active_for("/item/", "/item/"));

        // plus sub one level for all combinations of trailing slashes
        assert!(is_active_for("/item", "/item/one"));
        assert!(is_active_for("/item", "/item/one/"));
        assert!(is_active_for("/item/", "/item/one"));
        assert!(is_active_for("/item/", "/item/one/"));

        // both at two levels for all combinations of trailing slashes
        assert!(is_active_for("/item/1", "/item/1"));
        assert!(is_active_for("/item/1", "/item/1/"));
        assert!(is_active_for("/item/1/", "/item/1"));
        assert!(is_active_for("/item/1/", "/item/1/"));

        // plus sub various levels for all combinations of trailing slashes
        assert!(is_active_for("/item/1", "/item/1/two"));
        assert!(is_active_for("/item/1", "/item/1/three/four/"));
        assert!(is_active_for("/item/1/", "/item/1/three/four"));
        assert!(is_active_for("/item/1/", "/item/1/two/"));

        // both at various levels for various trailing slashes
        assert!(is_active_for("/item/1/two/three", "/item/1/two/three"));
        assert!(is_active_for(
            "/item/1/two/three/444",
            "/item/1/two/three/444/"
        ));
        assert!(is_active_for(
            "/item/1/two/three/444/FIVE/",
            "/item/1/two/three/444/FIVE"
        ));
        assert!(is_active_for(
            "/item/1/two/three/444/FIVE/final/",
            "/item/1/two/three/444/FIVE/final/"
        ));

        // sub various levels for various trailing slashes
        assert!(is_active_for(
            "/item/1/two/three",
            "/item/1/two/three/three/two/1/item"
        ));
        assert!(is_active_for(
            "/item/1/two/three/444",
            "/item/1/two/three/444/just_one_more/"
        ));
        assert!(is_active_for(
            "/item/1/two/three/444/final/",
            "/item/1/two/three/444/final/just/kidding"
        ));

        // edge/weird/unexpected cases?

        // since empty fragments are not checked, these all highlight
        assert!(is_active_for(
            "/item/////",
            "/item/1/two/three/three/two/1/item"
        ));
        assert!(is_active_for(
            "/item/1///three//1",
            "/item/1/two/three/three/two/1/item"
        ));

        // artifact of the checking algorithm, as it assumes empty segments denote termination of sort, so
        // omission acts like a wildcard that isn't checked.
        assert!(is_active_for(
            "/item//foo",
            "/item/this_is_not_empty/foo/bar/baz"
        ));
    }

    #[test]
    fn is_active_for_mismatched() {
        // root
        assert!(!is_active_for("/somewhere", "/"));
        assert!(!is_active_for("/somewhere/", "/"));
        assert!(!is_active_for("/else/where", "/"));
        assert!(!is_active_for("/no/where/", "/"));
        assert!(!is_active_for("/", "/somewhere"));
        assert!(!is_active_for("/", "/somewhere/"));
        assert!(!is_active_for("/", "/else/where"));
        assert!(!is_active_for("/", "/no/where/"));

        // mismatch either side all cominations of trailing slashes
        assert!(!is_active_for("/level", "/item"));
        assert!(!is_active_for("/level", "/item/"));
        assert!(!is_active_for("/level/", "/item"));
        assert!(!is_active_for("/level/", "/item/"));

        // one level parent for all combinations of trailing slashes
        assert!(!is_active_for("/item/one", "/item"));
        assert!(!is_active_for("/item/one/", "/item"));
        assert!(!is_active_for("/item/one", "/item/"));
        assert!(!is_active_for("/item/one/", "/item/"));

        // various parent levels for all combinations of trailing slashes
        assert!(!is_active_for("/item/1/two", "/item/1"));
        assert!(!is_active_for("/item/1/three/four/", "/item/1"));
        assert!(!is_active_for("/item/1/three/four", "/item/"));
        assert!(!is_active_for("/item/1/two/", "/item/"));

        // sub various levels for various trailing slashes
        assert!(!is_active_for(
            "/item/1/two/three/three/two/1/item",
            "/item/1/two/three"
        ));
        assert!(!is_active_for(
            "/item/1/two/three/444/just_one_more/",
            "/item/1/two/three/444"
        ));
        assert!(!is_active_for(
            "/item/1/two/three/444/final/just/kidding",
            "/item/1/two/three/444/final/"
        ));

        // edge/weird/unexpected cases?

        // first non-empty one is checked anyway, so it checks as if `href="/"`
        assert!(!is_active_for(
            "//////",
            "/item/1/two/three/three/two/1/item"
        ));

        // The following tests assumes the less common interpretation of `/item/` being a resource with proper
        // subitems while `/item` just simply browsing the flat `item` while still currently at `/`, as the
        // user hasn't "initiate the descent" into it (e.g. a certain filesystem tried to implement a feature
        // where a directory can be opened as a file), it may be argued that when user is simply checking what
        // `/item` is by going to that location, they are still active at `/` - only by actually going into
        // `/item/` that they are truly active there.
        //
        // In any case, the algorithm currently assumes the more "typical" case where the non-slash version is
        // an "alias" of the trailing-slash version (so aria-current is set), as "ordinarily" this is the case
        // expected by "ordinary" end-users who almost never encounter this particular scenario.

        // assert!(!is_active_for("/item/", "/item"));
        // assert!(!is_active_for("/item/1/", "/item/1"));
        // assert!(!is_active_for("/item/1/two/three/444/FIVE/", "/item/1/two/three/444/FIVE"));
    }
}
