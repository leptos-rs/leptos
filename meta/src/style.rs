use crate::use_head;
use leptos::{nonce::use_nonce, *};

/// Injects an [`HTMLStyleElement`](https://developer.mozilla.org/en-US/docs/Web/API/HTMLStyleElement) into the document
/// head, accepting any of the valid attributes for that tag.
///
/// ```
/// use leptos::*;
/// use leptos_meta::*;
///
/// #[component]
/// fn MyApp() -> impl IntoView {
///     provide_meta_context();
///
///     view! {
///       <main>
///         <Style>
///           "body { font-weight: bold; }"
///         </Style>
///       </main>
///     }
/// }
/// ```
#[component(transparent)]
pub fn Style(
    /// An ID for the `<script>` tag.
    #[prop(optional, into)]
    id: Option<Oco<'static, str>>,
    /// The [`media`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/style#attr-media) attribute.
    #[prop(optional, into)]
    media: Option<Oco<'static, str>>,
    /// The [`nonce`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/style#attr-nonce) attribute.
    #[prop(optional, into)]
    nonce: Option<Oco<'static, str>>,
    /// The [`title`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/style#attr-title) attribute.
    #[prop(optional, into)]
    title: Option<Oco<'static, str>>,
    /// The [`blocking`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/style#attr-blocking) attribute.
    #[prop(optional, into)]
    blocking: Option<Oco<'static, str>>,
    /// The content of the `<style>` tag.
    #[prop(optional)]
    children: Option<Box<dyn FnOnce() -> Fragment>>,
    /// Custom attributes.
    #[prop(attrs, optional)]
    attrs: Vec<(&'static str, Attribute)>,
) -> impl IntoView {
    let meta = use_head();
    let next_id = meta.tags.get_next_id();
    let mut id: Oco<'static, str> =
        id.unwrap_or_else(|| format!("leptos-link-{}", next_id.0).into());

    let builder_el = leptos::leptos_dom::html::as_meta_tag({
        let id = id.clone_inplace();
        move || {
            attrs
                .into_iter()
                .fold(leptos::leptos_dom::html::style(), |el, (name, value)| {
                    el.attr(name, value)
                })
                .attr("id", id)
                .attr("media", media)
                .attr("nonce", nonce)
                .attr("title", title)
                .attr("blocking", blocking)
                .attr("nonce", use_nonce())
        }
    });
    let builder_el = if let Some(children) = children {
        let frag = children();
        let mut style = String::new();
        for node in frag.nodes {
            match node {
                View::Text(text) => style.push_str(&text.content),
                _ => leptos::logging::warn!(
                    "Only text nodes are supported as children of <Style/>."
                ),
            }
        }
        builder_el.child(style)
    } else {
        builder_el
    };

    meta.tags.register(id, builder_el.into_any());
}
