use crate::register;
use leptos::{
    component, oco::Oco, prelude::GlobalAttributes,
    tachys::html::element::link, IntoView,
};

/// Injects an [`HTMLLinkElement`](https://developer.mozilla.org/en-US/docs/Web/API/HTMLLinkElement) into the document
/// head, accepting any of the valid attributes for that tag.
///
/// ```
/// use leptos::prelude::*;
/// use leptos_meta::*;
///
/// #[component]
/// fn MyApp() -> impl IntoView {
///     provide_meta_context();
///
///     view! {
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
#[component]
pub fn Link(
    /// The [`id`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/link#attr-id) attribute.
    #[prop(optional, into)]
    id: Option<Oco<'static, str>>,
    /// The [`as`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/link#attr-as) attribute.
    #[prop(optional, into)]
    as_: Option<Oco<'static, str>>,
    /// The [`crossorigin`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/link#attr-crossorigin) attribute.
    #[prop(optional, into)]
    crossorigin: Option<Oco<'static, str>>,
    /// The [`fetchpriority`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/link#attr-fetchpriority) attribute.
    #[prop(optional, into)]
    fetchpriority: Option<Oco<'static, str>>,
    /// The [`href`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/link#attr-href) attribute.
    #[prop(optional, into)]
    href: Option<Oco<'static, str>>,
    /// The [`hreflang`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/link#attr-hreflang) attribute.
    #[prop(optional, into)]
    hreflang: Option<Oco<'static, str>>,
    /// The [`imagesizes`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/link#attr-imagesizes) attribute.
    #[prop(optional, into)]
    imagesizes: Option<Oco<'static, str>>,
    /// The [`imagesrcset`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/link#attr-imagesrcset) attribute.
    #[prop(optional, into)]
    imagesrcset: Option<Oco<'static, str>>,
    /// The [`integrity`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/link#attr-integrity) attribute.
    #[prop(optional, into)]
    integrity: Option<Oco<'static, str>>,
    /// The [`media`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/link#attr-media) attribute.
    #[prop(optional, into)]
    media: Option<Oco<'static, str>>,
    /// The [`referrerpolicy`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/link#attr-referrerpolicy) attribute.
    #[prop(optional, into)]
    referrerpolicy: Option<Oco<'static, str>>,
    /// The [`rel`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/link#attr-rel) attribute.
    #[prop(optional, into)]
    rel: Option<Oco<'static, str>>,
    /// The [`sizes`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/link#attr-sizes) attribute.
    #[prop(optional, into)]
    sizes: Option<Oco<'static, str>>,
    /// The [`title`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/link#attr-title) attribute.
    #[prop(optional, into)]
    title: Option<Oco<'static, str>>,
    /// The [`type`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/link#attr-type) attribute.
    #[prop(optional, into)]
    type_: Option<Oco<'static, str>>,
    /// The [`blocking`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/link#attr-blocking) attribute.
    #[prop(optional, into)]
    blocking: Option<Oco<'static, str>>,
) -> impl IntoView {
    // TODO additional attributes
    register(
        link()
            .id(id)
            .r#as(as_)
            .crossorigin(crossorigin)
            .fetchpriority(fetchpriority)
            .href(href)
            .hreflang(hreflang)
            .imagesizes(imagesizes)
            .imagesrcset(imagesrcset)
            .integrity(integrity)
            .media(media)
            .referrerpolicy(referrerpolicy)
            .rel(rel)
            .sizes(sizes)
            .title(title)
            .r#type(type_)
            .blocking(blocking),
    )
}
