use crate::register;
use leptos::{
    component,
    prelude::{CustomAttribute, GlobalAttributes},
    tachys::html::element::meta,
    text_prop::TextProp,
    IntoView,
};

/// Injects an [`HTMLMetaElement`](https://developer.mozilla.org/en-US/docs/Web/API/HTMLMetaElement) into the document
/// head to set metadata
///
/// ```
/// use leptos::prelude::*;
/// use leptos_meta::*;
///
/// #[component]
/// fn MyApp() -> impl IntoView {
///   provide_meta_context();
///
///   view! {
///     <main>
///       <Meta charset="utf-8"/>
///       <Meta name="description" content="A Leptos fan site."/>
///       <Meta http_equiv="refresh" content="3;url=https://github.com/leptos-rs/leptos"/>
///     </main>
///   }
/// }
/// ```
#[component]
pub fn Meta(
    /// The [`charset`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/meta#attr-charset) attribute.
    #[prop(optional, into)]
    charset: Option<TextProp>,
    /// The [`name`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/meta#attr-name) attribute.
    #[prop(optional, into)]
    name: Option<TextProp>,
    /// The [`property`](https://ogp.me/) attribute.
    #[prop(optional, into)]
    property: Option<TextProp>,
    /// The [`http-equiv`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/meta#attr-http-equiv) attribute.
    #[prop(optional, into)]
    http_equiv: Option<TextProp>,
    /// The [`itemprop`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/meta#attr-itemprop) attribute.
    #[prop(optional, into)]
    itemprop: Option<TextProp>,
    /// The [`content`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/meta#attr-content) attribute.
    #[prop(optional, into)]
    content: Option<TextProp>,
) -> impl IntoView {
    register(
        meta()
            .charset(charset.map(|v| move || v.get()))
            .name(name.map(|v| move || v.get()))
            .attr("property", property.map(|v| move || v.get()))
            .http_equiv(http_equiv.map(|v| move || v.get()))
            .itemprop(itemprop.map(|v| move || v.get()))
            .content(content.map(|v| move || v.get())),
    )
}
