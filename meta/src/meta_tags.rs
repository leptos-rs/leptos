use crate::{use_head, TextProp};
use leptos::{component, IntoView, Scope};

/// Injects an [HTMLMetaElement](https://developer.mozilla.org/en-US/docs/Web/API/HTMLMetaElement) into the document
/// head to set metadata
///
/// ```
/// use leptos::*;
/// use leptos_meta::*;
///
/// #[component]
/// fn MyApp(cx: Scope) -> impl IntoView {
///   provide_meta_context(cx);
///
///   view! { cx,
///     <main>
///       <Meta charset="utf-8"/>
///       <Meta name="description" content="A Leptos fan site."/>
///       <Meta http_equiv="refresh" content="3;url=https://github.com/leptos-rs/leptos"/>
///     </main>
///   }
/// }
/// ```
#[component(transparent)]
pub fn Meta(
    cx: Scope,
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
    /// The [`content`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/meta#attr-content) attribute.
    #[prop(optional, into)]
    content: Option<TextProp>,
) -> impl IntoView {
    let meta = use_head(cx);
    let next_id = meta.tags.get_next_id();
    let id = format!("leptos-link-{}", next_id.0);

    let builder_el = leptos::leptos_dom::html::as_meta_tag(move || {
        leptos::leptos_dom::html::meta(cx)
            .attr("charset", move || charset.as_ref().map(|v| v.get()))
            .attr("name", move || name.as_ref().map(|v| v.get()))
            .attr("property", move || property.as_ref().map(|v| v.get()))
            .attr("http-equiv", move || http_equiv.as_ref().map(|v| v.get()))
            .attr("content", move || content.as_ref().map(|v| v.get()))
    });

    meta.tags.register(cx, id.into(), builder_el.into_any());
}
