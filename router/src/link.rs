use crate::{components::RouterContext, hooks::use_resolved_path};
use leptos::{children::Children, oco::Oco, prelude::*};
use reactive_graph::{computed::ArcMemo, owner::use_context};
use std::{borrow::Cow, rc::Rc};

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

impl ToHref for Rc<str> {
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
///
/// ### DOM Properties
///
/// `<a>` elements can take several additional DOM properties with special meanings.
/// - **`prop:state`**: An object of any type that will be pushed to router state.
/// - **`prop:replace`**: If `true`, the link will not add to the browser's history (so, pressing `Back`
/// will skip this page.)
///
/// Previously, this component took these as component props. Now, they can be added using the
/// `prop:` syntax, and will be added directly to the DOM. They can work with either `<a>` elements
/// or the `<A/>` component.
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
    /// If `true`, and when `href` has a trailing slash, `aria-current` be only be set if `current_url` also has
    /// a trailing slash.
    #[prop(optional)]
    strict_trailing_slash: bool,
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
        children: Children,
        strict_trailing_slash: bool,
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
                            is_active_for(path, loc, strict_trailing_slash)
                        }
                    })
                })
            }
        });

        view! {
            <a
                href=move || href.get().unwrap_or_default()
                target=target
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
    inner(href, target, exact, children, strict_trailing_slash)
}

// Test if `href` is active for `location`.  Assumes _both_ `href` and `location` begin with a `'/'`.
fn is_active_for(
    href: &str,
    location: &str,
    strict_trailing_slash: bool,
) -> bool {
    let mut href_f = href.split('/');
    // location _must_ be consumed first to avoid draining href_f early
    // also using enumerate to special case _the first two_ so that the allowance for ignoring the comparison
    // with the loc fragment on an emtpy href fragment for non root related parts.
    std::iter::zip(location.split('/'), href_f.by_ref())
        .enumerate()
        .all(|(c, (loc_p, href_p))| {
            loc_p == href_p || href_p.is_empty() && c > 1
        })
        && match href_f.next() {
            // when no href fragments remain, location is definitely somewhere nested inside href
            None => true,
            // when an outstanding href fragment is an empty string, default `strict_trailing_slash` setting will
            // have the typical expected case where href="/item/" is active for location="/item", but when toggled
            // to true it becomes inactive; please refer to test case comments for explanation.
            Some("") => !strict_trailing_slash,
            // inactive when href fragments remain (otherwise false postive for href="/item/one", location="/item")
            _ => false,
        }
}

#[cfg(test)]
mod tests {
    use super::is_active_for;

    #[test]
    fn is_active_for_matched() {
        [false, true].into_iter().for_each(|f| {
            // root
            assert!(is_active_for("/", "/", f));

            // both at one level for all combinations of trailing slashes
            assert!(is_active_for("/item", "/item", f));
            // assert!(is_active_for("/item/", "/item", f));
            assert!(is_active_for("/item", "/item/", f));
            assert!(is_active_for("/item/", "/item/", f));

            // plus sub one level for all combinations of trailing slashes
            assert!(is_active_for("/item", "/item/one", f));
            assert!(is_active_for("/item", "/item/one/", f));
            assert!(is_active_for("/item/", "/item/one", f));
            assert!(is_active_for("/item/", "/item/one/", f));

            // both at two levels for all combinations of trailing slashes
            assert!(is_active_for("/item/1", "/item/1", f));
            // assert!(is_active_for("/item/1/", "/item/1", f));
            assert!(is_active_for("/item/1", "/item/1/", f));
            assert!(is_active_for("/item/1/", "/item/1/", f));

            // plus sub various levels for all combinations of trailing slashes
            assert!(is_active_for("/item/1", "/item/1/two", f));
            assert!(is_active_for("/item/1", "/item/1/three/four/", f));
            assert!(is_active_for("/item/1/", "/item/1/three/four", f));
            assert!(is_active_for("/item/1/", "/item/1/two/", f));

            // both at various levels for various trailing slashes
            assert!(is_active_for("/item/1/two/three", "/item/1/two/three", f));
            assert!(is_active_for(
                "/item/1/two/three/444",
                "/item/1/two/three/444/",
                f
            ));
            // assert!(is_active_for(
            //     "/item/1/two/three/444/FIVE/",
            //     "/item/1/two/three/444/FIVE",
            //     f
            // ));
            assert!(is_active_for(
                "/item/1/two/three/444/FIVE/final/",
                "/item/1/two/three/444/FIVE/final/",
                f
            ));

            // sub various levels for various trailing slashes
            assert!(is_active_for(
                "/item/1/two/three",
                "/item/1/two/three/three/two/1/item",
                f
            ));
            assert!(is_active_for(
                "/item/1/two/three/444",
                "/item/1/two/three/444/just_one_more/",
                f
            ));
            assert!(is_active_for(
                "/item/1/two/three/444/final/",
                "/item/1/two/three/444/final/just/kidding",
                f
            ));

            // edge/weird/unexpected cases?

            // since empty fragments are not checked, these all highlight
            assert!(is_active_for(
                "/item/////",
                "/item/one/two/three/four/",
                f
            ));
            assert!(is_active_for(
                "/item/////",
                "/item/1/two/three/three/two/1/item",
                f
            ));
            assert!(is_active_for(
                "/item/1///three//1",
                "/item/1/two/three/three/two/1/item",
                f
            ));

            // artifact of the checking algorithm, as it assumes empty segments denote termination of sort, so
            // omission acts like a wildcard that isn't checked.
            assert!(is_active_for(
                "/item//foo",
                "/item/this_is_not_empty/foo/bar/baz",
                f
            ));
        });

        // Refer to comment on the similar scenario on the next test case for explanation, as this assumes the
        // "typical" case where the strict trailing slash flag is unset or false.
        assert!(is_active_for("/item/", "/item", false));
        assert!(is_active_for("/item/1/", "/item/1", false));
        assert!(is_active_for(
            "/item/1/two/three/444/FIVE/",
            "/item/1/two/three/444/FIVE",
            false
        ));
    }

    #[test]
    fn is_active_for_mismatched() {
        [false, true].into_iter().for_each(|f| {
            // href="/"
            assert!(!is_active_for("/", "/item", f));
            assert!(!is_active_for("/", "/somewhere/", f));
            assert!(!is_active_for("/", "/else/where", f));
            assert!(!is_active_for("/", "/no/where/", f));

            // non root href but location at root
            assert!(!is_active_for("/somewhere", "/", f));
            assert!(!is_active_for("/somewhere/", "/", f));
            assert!(!is_active_for("/else/where", "/", f));
            assert!(!is_active_for("/no/where/", "/", f));

            // mismatch either side all cominations of trailing slashes
            assert!(!is_active_for("/level", "/item", f));
            assert!(!is_active_for("/level", "/item/", f));
            assert!(!is_active_for("/level/", "/item", f));
            assert!(!is_active_for("/level/", "/item/", f));

            // one level parent for all combinations of trailing slashes
            assert!(!is_active_for("/item/one", "/item", f));
            assert!(!is_active_for("/item/one/", "/item", f));
            assert!(!is_active_for("/item/one", "/item/", f));
            assert!(!is_active_for("/item/one/", "/item/", f));

            // various parent levels for all combinations of trailing slashes
            assert!(!is_active_for("/item/1/two", "/item/1", f));
            assert!(!is_active_for("/item/1/three/four/", "/item/1", f));
            assert!(!is_active_for("/item/1/three/four", "/item/", f));
            assert!(!is_active_for("/item/1/two/", "/item/", f));

            // sub various levels for various trailing slashes
            assert!(!is_active_for(
                "/item/1/two/three/three/two/1/item",
                "/item/1/two/three",
                f
            ));
            assert!(!is_active_for(
                "/item/1/two/three/444/just_one_more/",
                "/item/1/two/three/444",
                f
            ));
            assert!(!is_active_for(
                "/item/1/two/three/444/final/just/kidding",
                "/item/1/two/three/444/final/",
                f
            ));

            // edge/weird/unexpected cases?

            // default trailing slash has the expected behavior of non-matching of any non-root location
            // this checks as if `href="/"`
            assert!(!is_active_for(
                "//////",
                "/item/1/two/three/three/two/1/item",
                f
            ));
            // some weird root location?
            assert!(!is_active_for(
                "/item/1/two/three/three/two/1/item",
                "//////",
                f
            ));

            assert!(!is_active_for(
                "/item/one/two/three/four/",
                "/item/////",
                f
            ));
            assert!(!is_active_for(
                "/item/one/two/three/four/",
                "/item////four/",
                f
            ));
        });

        // The following tests enables the `strict_trailing_slash` flag, which allows the less common
        // interpretation of `/item/` being a resource with proper subitems while `/item` just simply browsing
        // the flat `item` while still currently at `/`, as the user hasn't "initiate the descent" into it
        // (e.g. a certain filesystem tried to implement a feature where a directory can be opened as a file),
        // it may be argued that when user is simply checking what `/item` is by going to that location, they
        // are still active at `/` - only by actually going into `/item/` that they are truly active there.
        //
        // In any case, the algorithm currently assumes the more "typical" case where the non-slash version is
        // an "alias" of the trailing-slash version (so aria-current is set), as "ordinarily" this is the case
        // expected by "ordinary" end-users who almost never encounter this particular scenario.

        assert!(!is_active_for("/item/", "/item", true));
        assert!(!is_active_for("/item/1/", "/item/1", true));
        assert!(!is_active_for(
            "/item/1/two/three/444/FIVE/",
            "/item/1/two/three/444/FIVE",
            true
        ));

        // That said, in this particular scenario, the definition above should result the following be asserted
        // as true, but then it follows that every scenario may be true as the root was special cased - in
        // which case it becomes a bit meaningless?
        //
        // assert!(is_active_for("/", "/item", true));
        //
        // Perhaps there needs to be a flag such that aria-curently applies only the _same level_, e.g
        // assert!(is_same_level("/", "/"))
        // assert!(is_same_level("/", "/anything"))
        // assert!(!is_same_level("/", "/some/"))
        // assert!(!is_same_level("/", "/some/level"))
        // assert!(is_same_level("/some/", "/some/"))
        // assert!(is_same_level("/some/", "/some/level"))
        // assert!(!is_same_level("/some/", "/some/level/"))
        // assert!(!is_same_level("/some/", "/some/level/deeper"))
    }
}
