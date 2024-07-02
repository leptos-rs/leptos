use crate::register;
use leptos::{
    attr::global::GlobalAttributes, component, tachys::html::element::link,
    IntoView,
};

/// Injects an [`HTMLLinkElement`](https://developer.mozilla.org/en-US/docs/Web/API/HTMLLinkElement) into the document
/// head that loads a stylesheet from the URL given by the `href` property.
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
///         <Stylesheet href="/style.css"/>
///       </main>
///     }
/// }
/// ```
#[component]
pub fn Stylesheet(
    /// The URL at which the stylesheet is located.
    #[prop(into)]
    href: String,
    /// An ID for the stylesheet.
    #[prop(optional, into)]
    id: Option<String>,
) -> impl IntoView {
    // TODO additional attributes
    register(link().id(id).rel("stylesheet").href(href))
}
