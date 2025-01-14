use crate::{register, OrDefaultNonce};
use leptos::{
    component, oco::Oco, prelude::*, tachys::html::element::script, IntoView,
};

/// Injects an [`HTMLScriptElement`](https://developer.mozilla.org/en-US/docs/Web/API/HTMLScriptElement) into the document
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
///         <Script>
///           "console.log('Hello, world!');"
///         </Script>
///       </main>
///     }
/// }
/// ```
#[component]
pub fn Script(
    /// An ID for the `<script>` tag.
    #[prop(optional, into)]
    id: Option<Oco<'static, str>>,
    /// The [`async`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/script#attr-async) attribute.
    #[prop(optional, into)]
    async_: Option<Oco<'static, str>>,
    /// The [`crossorigin`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/script#attr-crossorigin) attribute.
    #[prop(optional, into)]
    crossorigin: Option<Oco<'static, str>>,
    /// The [`defer`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/script#attr-defer) attribute.
    #[prop(optional, into)]
    defer: Option<Oco<'static, str>>,
    /// The [`fetchpriority `](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/script#attr-fetchpriority ) attribute.
    #[prop(optional, into)]
    fetchpriority: Option<Oco<'static, str>>,
    /// The [`integrity`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/script#attr-integrity) attribute.
    #[prop(optional, into)]
    integrity: Option<Oco<'static, str>>,
    /// The [`nomodule`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/script#attr-nomodule) attribute.
    #[prop(optional, into)]
    nomodule: Option<Oco<'static, str>>,
    /// The [`nonce`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/script#attr-nonce) attribute.
    #[prop(optional, into)]
    nonce: Option<Oco<'static, str>>,
    /// The [`referrerpolicy`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/script#attr-referrerpolicy) attribute.
    #[prop(optional, into)]
    referrerpolicy: Option<Oco<'static, str>>,
    /// The [`src`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/script#attr-src) attribute.
    #[prop(optional, into)]
    src: Option<Oco<'static, str>>,
    /// The [`type`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/script#attr-type) attribute.
    #[prop(optional, into)]
    type_: Option<Oco<'static, str>>,
    /// The [`blocking`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/script#attr-blocking) attribute.
    #[prop(optional, into)]
    blocking: Option<Oco<'static, str>>,
    /// The content of the `<script>` tag.
    #[prop(optional)]
    children: Option<Children>,
) -> impl IntoView {
    register(
        script()
            .id(id)
            .r#async(async_)
            .crossorigin(crossorigin)
            .defer(defer)
            .fetchpriority(fetchpriority)
            .integrity(integrity)
            .nomodule(nomodule)
            .nonce(nonce.or_default_nonce())
            .referrerpolicy(referrerpolicy)
            .src(src)
            .r#type(type_)
            .blocking(blocking)
            .child(children.map(|c| c())),
    )
}
