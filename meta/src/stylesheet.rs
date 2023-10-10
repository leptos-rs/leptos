use crate::Link;
use leptos::*;

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
#[component(transparent)]
pub fn Stylesheet(
    /// The URL at which the stylesheet is located.
    #[prop(into)]
    href: String,
    /// An ID for the stylesheet.
    #[prop(optional, into)]
    id: Option<String>,
    /// Custom attributes.
    #[prop(attrs, optional)]
    attrs: Vec<(&'static str, Attribute)>,
) -> impl IntoView {
    if let Some(id) = id {
        view! {
            <Link id rel="stylesheet" href attrs/>
        }
    } else {
        view! {
            <Link rel="stylesheet" href attrs/>
        }
    }
}
