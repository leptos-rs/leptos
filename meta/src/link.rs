use crate::use_head;
use cfg_if::cfg_if;
use leptos::*;
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
};

/// Manages all of the Links set by [Link] components.
#[derive(Clone, Default)]
pub struct LinkContext {
    #[allow(clippy::type_complexity)]
    els: Rc<RefCell<HashMap<String, (HtmlElement<Link>, Scope, Option<web_sys::HtmlLinkElement>)>>>,
    next_id: Rc<Cell<LinkId>>,
}

impl std::fmt::Debug for LinkContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LinkContext").finish()
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
struct LinkId(usize);

impl LinkContext {
    fn get_next_id(&self) -> LinkId {
        let current_id = self.next_id.get();
        let next_id = LinkId(current_id.0 + 1);
        self.next_id.set(next_id);
        next_id
    }
}

#[cfg(feature = "ssr")]
impl LinkContext {
    /// Converts the set of Links into an HTML string that can be injected into the `<head>`.
    pub fn as_string(&self) -> String {
        self.els
            .borrow()
            .iter()
            .map(|(_, (builder_el, cx, _))| builder_el.clone().into_view(*cx).render_to_string(*cx))
            .collect()
    }
}

/// Injects an [HTMLLinkElement](https://developer.mozilla.org/en-US/docs/Web/API/HTMLLinkElement) into the document
/// head.
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
///       <Link rel="preload"
///         href="myFont.woff2"
///         as="font"
///         type="font/woff2"
///         crossorigin="anonymous"
///       />
///     </main>
///   }
/// }
/// ```
#[component(transparent)]
pub fn Link(
    cx: Scope,
    /// The [`id`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/link#attr-id) attribute.
    #[prop(optional, into)]
    id: Option<String>,
    /// The [`as`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/link#attr-as) attribute.
    #[prop(optional, into)]
    as_: Option<String>,
    /// The [`crossorigin`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/link#attr-crossorigin) attribute.
    #[prop(optional, into)]
    crossorigin: Option<String>,
    /// The [`disabled`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/link#attr-disabled) attribute.
    #[prop(optional, into)]
    disabled: Option<bool>,
    /// The [`fetchpriority`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/link#attr-fetchpriority) attribute.
    #[prop(optional, into)]
    fetchpriority: Option<String>,
    /// The [`href`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/link#attr-href) attribute.
    #[prop(optional, into)]
    href: Option<String>,
    /// The [`hreflang`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/link#attr-hreflang) attribute.
    #[prop(optional, into)]
    hreflang: Option<String>,
    /// The [`imagesizes`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/link#attr-imagesizes) attribute.
    #[prop(optional, into)]
    imagesizes: Option<String>,
    /// The [`imagesrcset`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/link#attr-imagesrcset) attribute.
    #[prop(optional, into)]
    imagesrcset: Option<String>,
    /// The [`integrity`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/link#attr-integrity) attribute.
    #[prop(optional, into)]
    integrity: Option<String>,
    /// The [`media`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/link#attr-media) attribute.
    #[prop(optional, into)]
    media: Option<String>,
    /// The [`prefetch`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/link#attr-prefetch) attribute.
    #[prop(optional, into)]
    prefetch: Option<String>,
    /// The [`referrerpolicy`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/link#attr-referrerpolicy) attribute.
    #[prop(optional, into)]
    referrerpolicy: Option<String>,
    /// The [`rel`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/link#attr-rel) attribute.
    #[prop(optional, into)]
    rel: Option<String>,
    /// The [`sizes`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/link#attr-sizes) attribute.
    #[prop(optional, into)]
    sizes: Option<String>,
    /// The [`title`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/link#attr-title) attribute.
    #[prop(optional, into)]
    title: Option<String>,
    /// The [`type`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/link#attr-type) attribute.
    #[prop(optional, into)]
    type_: Option<String>,
    /// The [`blocking`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/link#attr-blocking) attribute.
    #[prop(optional, into)]
    blocking: Option<String>,
) -> impl IntoView {
    let meta = use_head(cx);
    let links = &meta.links;
    let next_id = links.get_next_id();
    let id = id.unwrap_or_else(|| format!("leptos-link-{}", next_id.0));

    let builder_el = leptos::link(cx)
        .attr("id", &id)
        .attr("as_", as_)
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
        .attr("blocking", blocking);

    cfg_if! {
        if #[cfg(any(feature = "csr", feature = "hydrate"))] {
            use leptos::document;

            let element_to_hydrate = document()
                .get_element_by_id(&id)
                .map(|el| el.unchecked_into::<web_sys::HtmlLinkElement>());

            let el = element_to_hydrate.unwrap_or_else({
                let builder_el = builder_el.clone();
                move || {
                    let head = document().head().unwrap_throw();
                    head
                        .append_child(&builder_el)
                        .unwrap_throw();

                    (*builder_el).clone()
                }
            });

            on_cleanup(cx, {
                let el = el.clone();
                let els = meta.links.els.clone();
                let id = id.clone();
                move || {
                    let head = document().head().unwrap_throw();
                    _ = head.remove_child(&el);
                    els.borrow_mut().remove(&id);
                }
            });

            meta.links
                .els
                .borrow_mut()
                .insert(id, (builder_el, cx, Some(el)));

        } else {
            let meta = use_head(cx);
            meta.links.els.borrow_mut().insert(id, (builder_el, cx, None));
        }
    }
}
