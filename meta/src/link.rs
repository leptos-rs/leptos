use crate::use_head;
use leptos::{nonce::use_nonce, *};
use std::borrow::Cow;

/// Injects an [HTMLLinkElement](https://developer.mozilla.org/en-US/docs/Web/API/HTMLLinkElement) into the document
/// head, accepting any of the valid attributes for that tag.
/// ```
/// use leptos::*;
/// use leptos_meta::*;
///
/// #[component]
/// fn MyApp(cx: Scope) -> impl IntoView {
///     provide_meta_context(cx);
///
///     view! { cx,
///       <main>
///         <Link rel="preload"
///           href="myFont.woff2"
///           as_="font"
///           type_="font/woff2"
///           crossorigin="anonymous"
///         />
///       </main>
///     }
/// }
/// ```
#[component(transparent)]
pub fn Link(
    cx: Scope,
    /// The [`id`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/link#attr-id) attribute.
    #[prop(optional, into)]
    id: Option<Cow<'static, str>>,
    /// The [`as`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/link#attr-as) attribute.
    #[prop(optional, into)]
    as_: Option<Cow<'static, str>>,
    /// The [`crossorigin`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/link#attr-crossorigin) attribute.
    #[prop(optional, into)]
    crossorigin: Option<Cow<'static, str>>,
    /// The [`disabled`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/link#attr-disabled) attribute.
    #[prop(optional, into)]
    disabled: Option<bool>,
    /// The [`fetchpriority`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/link#attr-fetchpriority) attribute.
    #[prop(optional, into)]
    fetchpriority: Option<Cow<'static, str>>,
    /// The [`href`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/link#attr-href) attribute.
    #[prop(optional, into)]
    href: Option<Cow<'static, str>>,
    /// The [`hreflang`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/link#attr-hreflang) attribute.
    #[prop(optional, into)]
    hreflang: Option<Cow<'static, str>>,
    /// The [`imagesizes`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/link#attr-imagesizes) attribute.
    #[prop(optional, into)]
    imagesizes: Option<Cow<'static, str>>,
    /// The [`imagesrcset`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/link#attr-imagesrcset) attribute.
    #[prop(optional, into)]
    imagesrcset: Option<Cow<'static, str>>,
    /// The [`integrity`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/link#attr-integrity) attribute.
    #[prop(optional, into)]
    integrity: Option<Cow<'static, str>>,
    /// The [`media`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/link#attr-media) attribute.
    #[prop(optional, into)]
    media: Option<Cow<'static, str>>,
    /// The [`prefetch`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/link#attr-prefetch) attribute.
    #[prop(optional, into)]
    prefetch: Option<Cow<'static, str>>,
    /// The [`referrerpolicy`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/link#attr-referrerpolicy) attribute.
    #[prop(optional, into)]
    referrerpolicy: Option<Cow<'static, str>>,
    /// The [`rel`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/link#attr-rel) attribute.
    #[prop(optional, into)]
    rel: Option<Cow<'static, str>>,
    /// The [`sizes`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/link#attr-sizes) attribute.
    #[prop(optional, into)]
    sizes: Option<Cow<'static, str>>,
    /// The [`title`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/link#attr-title) attribute.
    #[prop(optional, into)]
    title: Option<Cow<'static, str>>,
    /// The [`type`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/link#attr-type) attribute.
    #[prop(optional, into)]
    type_: Option<Cow<'static, str>>,
    /// The [`blocking`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/link#attr-blocking) attribute.
    #[prop(optional, into)]
    blocking: Option<Cow<'static, str>>,
) -> impl IntoView {
    let meta = use_head(cx);
    let next_id = meta.tags.get_next_id();
    let id: Cow<'static, str> =
        id.unwrap_or_else(|| format!("leptos-link-{}", next_id.0).into());

    let builder_el = leptos::leptos_dom::html::as_meta_tag({
        let id = id.clone();
        move || {
            leptos::leptos_dom::html::link(cx)
                .attr("id", id)
                .attr("as", as_)
                .attr("crossorigin", crossorigin)
                .attr("disabled", disabled.unwrap_or(false))
                .attr("fetchpriority", fetchpriority)
                .attr("href", href)
                .attr("hreflang", hreflang)
                .attr("imagesizes", imagesizes)
                .attr("imagesrcset", imagesrcset)
                .attr("integrity", integrity)
                .attr("media", media)
                .attr("prefetch", prefetch)
                .attr("referrerpolicy", referrerpolicy)
                .attr("rel", rel)
                .attr("sizes", sizes)
                .attr("title", title)
                .attr("type", type_)
                .attr("blocking", blocking)
                .attr("nonce", use_nonce(cx))
        }
    });

    meta.tags.register(cx, id, builder_el.into_any());
}
