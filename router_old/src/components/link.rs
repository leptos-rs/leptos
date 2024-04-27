use crate::{use_location, use_resolved_path, State};
use leptos::*;
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
#[cfg_attr(
    any(debug_assertions, feature = "ssr"),
    tracing::instrument(level = "trace", skip_all,)
)]
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
    /// Provides a class to be added when the link is active. If provided, it will
    /// be added at the same time that the `aria-current` attribute is set.
    ///
    /// This supports multiple space-separated class names.
    ///
    /// **Performance**: If it’s possible to style the link using the CSS with the
    /// `[aria-current=page]` selector, you should prefer that, as it enables significant
    /// SSR optimizations.
    #[prop(optional, into)]
    active_class: Option<Oco<'static, str>>,
    /// An object of any type that will be pushed to router state
    #[prop(optional)]
    state: Option<State>,
    /// If `true`, the link will not add to the browser's history (so, pressing `Back`
    /// will skip this page.)
    #[prop(optional)]
    replace: bool,
    /// Sets the `class` attribute on the underlying `<a>` tag, making it easier to style.
    #[prop(optional, into)]
    class: Option<AttributeValue>,
    /// Sets the `id` attribute on the underlying `<a>` tag, making it easier to target.
    #[prop(optional, into)]
    id: Option<Oco<'static, str>>,
    /// Arbitrary attributes to add to the `<a>`. Attributes can be added with the
    /// `attr:` syntax in the `view` macro.
    #[prop(attrs)]
    attributes: Vec<(&'static str, Attribute)>,
    /// The nodes or elements to be shown inside the link.
    children: Children,
) -> impl IntoView
where
    H: ToHref + 'static,
{
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        tracing::instrument(level = "trace", skip_all,)
    )]
    fn inner(
        href: Memo<Option<String>>,
        target: Option<Oco<'static, str>>,
        exact: bool,
        #[allow(unused)] state: Option<State>,
        #[allow(unused)] replace: bool,
        class: Option<AttributeValue>,
        #[allow(unused)] active_class: Option<Oco<'static, str>>,
        id: Option<Oco<'static, str>>,
        #[allow(unused)] attributes: Vec<(&'static str, Attribute)>,
        children: Children,
    ) -> View {
        #[cfg(not(any(feature = "hydrate", feature = "csr")))]
        {
            _ = state;
            _ = replace;
        }

        let location = use_location();
        let is_active = create_memo(move |_| {
            href.with(|href| {
                href.as_deref().is_some_and(|to| {
                    let path = to
                        .split(['?', '#'])
                        .next()
                        .unwrap_or_default()
                        .to_lowercase();
                    location.pathname.with(|loc| {
                        let loc = loc.to_lowercase();
                        if exact {
                            loc == path
                        } else {
                            std::iter::zip(loc.split('/'), path.split('/'))
                                .all(|(loc_p, path_p)| loc_p == path_p)
                        }
                    })
                })
            })
        });

        #[cfg(feature = "ssr")]
        {
            // if we have `active_class` or arbitrary attributes,
            // the SSR optimization doesn't play nicely
            // so we use the builder instead
            let needs_builder =
                active_class.is_some() || !attributes.is_empty();
            if needs_builder {
                    let mut a = leptos::html::a()
                        .attr("href", move || href.get().unwrap_or_default())
                        .attr("target", target)
                        .attr("aria-current", move || {
                            if is_active.get() {
                                Some("page")
                            } else {
                                None
                            }
                        })
                        .attr(
                            "class",
                            class.map(|class| class.into_attribute_boxed()),
                        );

                    if let Some(active_class) = active_class {
                        for class_name in active_class.split_ascii_whitespace()
                        {
                            a = a.class(class_name.to_string(), move || {
                                is_active.get()
                            })
                        }
                    }

                    a = a.attr("id", id).child(children());

                    for (attr_name, attr_value) in attributes {
                        a = a.attr(attr_name, attr_value);
                    }

                    a
                }
                // but keep the nice SSR optimization in most cases
                else {
                    view! {
                        <a
                            href=move || href.get().unwrap_or_default()
                            target=target
                            aria-current=move || if is_active.get() { Some("page") } else { None }
                            class=class
                            id=id
                        >
                            {children()}
                        </a>
                    }
                }
                .into_view()
        }

        // the non-SSR version doesn't need the SSR optimizations
        // DRY here to avoid WASM binary size bloat
        #[cfg(not(feature = "ssr"))]
        {
            let mut a = view! {
                <a
                    href=move || href.get().unwrap_or_default()
                    target=target
                    prop:state=state.map(|s| s.to_js_value())
                    prop:replace=replace
                    aria-current=move || if is_active.get() { Some("page") } else { None }
                    class=class
                    id=id
                >
                    {children()}
                </a>
            };

            if let Some(active_class) = active_class {
                for class_name in active_class.split_ascii_whitespace() {
                    a = a.class(class_name.to_string(), move || is_active.get())
                }
            }

            for (attr_name, attr_value) in attributes {
                a = a.attr(attr_name, attr_value);
            }

            a.into_view()
        }
    }

    let href = use_resolved_path(move || href.to_href()());
    inner(
        href,
        target,
        exact,
        state,
        replace,
        class,
        active_class,
        id,
        attributes,
        children,
    )
}
