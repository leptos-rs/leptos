use crate::{use_location, use_resolved_path, State};
use leptos::{leptos_dom::IntoView, *};
use std::borrow::Cow;

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
    tracing::instrument(level = "info", skip_all,)
)]
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
    /// Provides a class to be added when the link is active. If provided, it will
    /// be added at the same time that the `aria-current` attribute is set.
    ///
    /// This supports multiple space-separated class names.
    ///
    /// **Performance**: If it’s possible to style the link using the CSS with the
    /// `[aria-current=page]` selector, you should prefer that, as it enables significant
    /// SSR optimizations.
    #[prop(optional, into)]
    active_class: Option<Cow<'static, str>>,
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
    id: Option<String>,
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
        cx: Scope,
        href: Memo<Option<String>>,
        exact: bool,
        #[allow(unused)] state: Option<State>,
        #[allow(unused)] replace: bool,
        class: Option<AttributeValue>,
        #[allow(unused)] active_class: Option<Cow<'static, str>>,
        id: Option<String>,
        children: Children,
    ) -> View {
        #[cfg(not(any(feature = "hydrate", feature = "csr")))]
        {
            _ = state;
        }

        #[cfg(not(any(feature = "hydrate", feature = "csr")))]
        {
            _ = replace;
        }

        let location = use_location(cx);
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

        #[cfg(feature = "ssr")]
        {
            // if we have `active_class`, the SSR optimization doesn't play nicely
            // so we use the builder instead
            if let Some(active_class) = active_class {
                let mut a = leptos::html::a(cx)
                    .attr("href", move || href.get().unwrap_or_default())
                    .attr("aria-current", move || {
                        if is_active.get() {
                            Some("page")
                        } else {
                            None
                        }
                    })
                    .attr(
                        "class",
                        class.map(|class| class.into_attribute_boxed(cx)),
                    );

                for class_name in active_class.split_ascii_whitespace() {
                    a = a.class(class_name.to_string(), move || is_active.get())
                }

                a.attr("id", id).child(children(cx)).into_view(cx)
            }
            // but keep the nice SSR optimization in most cases
            else {
                view! { cx,
                    <a
                        href=move || href.get().unwrap_or_default()
                        aria-current=move || if is_active.get() { Some("page") } else { None }
                        class=class
                        id=id
                    >
                        {children(cx)}
                    </a>
                }
                .into_view(cx)
            }
        }

        // the non-SSR version doesn't need the SSR optimizations
        // DRY here to avoid WASM binary size bloat
        #[cfg(not(feature = "ssr"))]
        {
            let a = view! { cx,
                <a
                    href=move || href.get().unwrap_or_default()
                    prop:state={state.map(|s| s.to_js_value())}
                    prop:replace={replace}
                    aria-current=move || if is_active.get() { Some("page") } else { None }
                    class=class
                    id=id
                >
                    {children(cx)}
                </a>
            };
            if let Some(active_class) = active_class {
                let mut a = a;
                for class_name in active_class.split_ascii_whitespace() {
                    a = a.class(class_name.to_string(), move || is_active.get())
                }
                a
            } else {
                a
            }
            .into_view(cx)
        }
    }

    let href = use_resolved_path(cx, move || href.to_href()());
    inner(
        cx,
        href,
        exact,
        state,
        replace,
        class,
        active_class,
        id,
        children,
    )
}
