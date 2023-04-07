use crate::Link;
use leptos::*;

/// Injects an [HTMLLinkElement](https://developer.mozilla.org/en-US/docs/Web/API/HTMLLinkElement) into the document
/// head that loads a stylesheet from the URL given by the `href` property.
///
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
///         <Stylesheet href="/style.css"/>
///       </main>
///     }
/// }
/// ```
#[component(transparent)]
pub fn Stylesheet(
    cx: Scope,
    /// The URL at which the stylesheet is located.
    #[prop(into)]
    href: String,
    /// An ID for the stylesheet.
    #[prop(optional, into)]
    id: Option<String>,
) -> impl IntoView {
    if let Some(id) = id {
        view! { cx,
            <Link id rel="stylesheet" href/>
        }
    } else {
        view! { cx,
            <Link rel="stylesheet" href/>
        }
    }
}
