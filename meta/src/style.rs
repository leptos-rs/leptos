use crate::use_head;
use leptos::{nonce::use_nonce, *};
use std::borrow::Cow;

/// Injects an [HTMLStyleElement](https://developer.mozilla.org/en-US/docs/Web/API/HTMLStyleElement) into the document
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
///         <Style>
///           "body { font-weight: bold; }"
///         </Style>
///       </main>
///     }
/// }
/// ```
#[component(transparent)]
pub fn Style(
    cx: Scope,
    /// An ID for the `<script>` tag.
    #[prop(optional, into)]
    id: Option<Cow<'static, str>>,
    /// The [`media`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/style#attr-media) attribute.
    #[prop(optional, into)]
    media: Option<Cow<'static, str>>,
    /// The [`nonce`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/style#attr-nonce) attribute.
    #[prop(optional, into)]
    nonce: Option<Cow<'static, str>>,
    /// The [`title`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/style#attr-title) attribute.
    #[prop(optional, into)]
    title: Option<Cow<'static, str>>,
    /// The [`blocking`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/style#attr-blocking) attribute.
    #[prop(optional, into)]
    blocking: Option<Cow<'static, str>>,
    /// The content of the `<style>` tag.
    #[prop(optional)]
    children: Option<Box<dyn FnOnce(Scope) -> Fragment>>,
) -> impl IntoView {
    let meta = use_head(cx);
    let next_id = meta.tags.get_next_id();
    let id: Cow<'static, str> =
        id.unwrap_or_else(|| format!("leptos-link-{}", next_id.0).into());

    let builder_el = leptos::leptos_dom::html::as_meta_tag({
        let id = id.clone();
        move || {
            leptos::leptos_dom::html::style(cx)
                .attr("id", id)
                .attr("media", media)
                .attr("nonce", nonce)
                .attr("title", title)
                .attr("blocking", blocking)
                .attr("nonce", use_nonce(cx))
        }
    });
    let builder_el = if let Some(children) = children {
        let frag = children(cx);
        let mut style = String::new();
        for node in frag.nodes {
            match node {
                View::Text(text) => style.push_str(&text.content),
                _ => leptos::warn!(
                    "Only text nodes are supported as children of <Style/>."
                ),
            }
        }
        builder_el.child(style)
    } else {
        builder_el
    };

    meta.tags.register(cx, id, builder_el.into_any());
}
