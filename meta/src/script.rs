use crate::use_head;
use leptos::{nonce::use_nonce, *};
use std::borrow::Cow;

/// Injects an [HTMLScriptElement](https://developer.mozilla.org/en-US/docs/Web/API/HTMLScriptElement) into the document
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
///         <Script>
///           "console.log('Hello, world!');"
///         </Script>
///       </main>
///     }
/// }
/// ```
#[component(transparent)]
pub fn Script(
    cx: Scope,
    /// An ID for the `<script>` tag.
    #[prop(optional, into)]
    id: Option<Cow<'static, str>>,
    /// The [`async`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/script#attr-async) attribute.
    #[prop(optional, into)]
    async_: Option<Cow<'static, str>>,
    /// The [`crossorigin`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/script#attr-crossorigin) attribute.
    #[prop(optional, into)]
    crossorigin: Option<Cow<'static, str>>,
    /// The [`defer`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/script#attr-defer) attribute.
    #[prop(optional, into)]
    defer: Option<Cow<'static, str>>,
    /// The [`fetchpriority `](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/script#attr-fetchpriority ) attribute.
    #[prop(optional, into)]
    fetchpriority: Option<Cow<'static, str>>,
    /// The [`integrity`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/script#attr-integrity) attribute.
    #[prop(optional, into)]
    integrity: Option<Cow<'static, str>>,
    /// The [`nomodule`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/script#attr-nomodule) attribute.
    #[prop(optional, into)]
    nomodule: Option<Cow<'static, str>>,
    /// The [`nonce`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/script#attr-nonce) attribute.
    #[prop(optional, into)]
    nonce: Option<Cow<'static, str>>,
    /// The [`referrerpolicy`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/script#attr-referrerpolicy) attribute.
    #[prop(optional, into)]
    referrerpolicy: Option<Cow<'static, str>>,
    /// The [`src`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/script#attr-src) attribute.
    #[prop(optional, into)]
    src: Option<Cow<'static, str>>,
    /// The [`type`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/script#attr-type) attribute.
    #[prop(optional, into)]
    type_: Option<Cow<'static, str>>,
    /// The [`blocking`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/script#attr-blocking) attribute.
    #[prop(optional, into)]
    blocking: Option<Cow<'static, str>>,
    /// The content of the `<script>` tag.
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
            leptos::leptos_dom::html::script(cx)
                .attr("id", id)
                .attr("async", async_)
                .attr("crossorigin", crossorigin)
                .attr("defer", defer)
                .attr("fetchpriority ", fetchpriority)
                .attr("integrity", integrity)
                .attr("nomodule", nomodule)
                .attr("nonce", nonce)
                .attr("referrerpolicy", referrerpolicy)
                .attr("src", src)
                .attr("type", type_)
                .attr("blocking", blocking)
                .attr("nonce", use_nonce(cx))
        }
    });
    let builder_el = if let Some(children) = children {
        let frag = children(cx);
        let mut script = String::new();
        for node in frag.nodes {
            match node {
                View::Text(text) => script.push_str(&text.content),
                _ => leptos::warn!(
                    "Only text nodes are supported as children of <Script/>."
                ),
            }
        }
        builder_el.child(script)
    } else {
        builder_el
    };

    meta.tags.register(cx, id, builder_el.into_any());
}
