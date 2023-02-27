use crate::{Script, ScriptProps};
use leptos::*;

fn script_content(href: String) -> String {
    format!("(function() {{
        var head = document.head || document.getElementsByTagName('head')[0];
        var hide_style = document.createElement('style');
        hide_style.textContent = 'html{{visibility: hidden;opacity:0;}}';
        head.appendChild(hide_style);
        var style = document.createElement('style');
        style.textContent = '@import \"{href}\"';
        var fi = setInterval(function() {{
            try {{
                style.sheet.cssRules;
                head.removeChild(hide_style);
                clearInterval(fi);
            }} catch (e){{}}
        }}, 10);
        head.appendChild(style);
    }})();")
}

/// Injects an [HTMLStyleElement](https://developer.mozilla.org/en-US/docs/Web/API/HTMLStyleElement) into the document
/// head that loads a stylesheet from the URL given by the `href` property.
///
/// Additionally, this component first injects a temporary stylesheet to hide the rendered content until the target
/// stylesheet is confirmed to be loaded.
/// 
/// This is done to avoid the visible Flash of Unstyled Content (FOUC).
/// [Detecting CSS Load](https://www.phpied.com/when-is-a-stylesheet-really-loaded/)
/// [Eliminate Flash of Unstyled Content](https://stackoverflow.com/questions/3221561/eliminate-flash-of-unstyled-content/43823506)
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
///         <SsrStylesheet href="/style.css"/>
///       </main>
///     }
/// }
/// ```
#[component(transparent)]
pub fn SsrStylesheet(
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
            <Script id>{script_content(href)}</Script>
        }
    } else {
        view! { cx,
            <Script>{script_content(href)}</Script>
        }
    }
}
