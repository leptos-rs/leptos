use crate::register;
use leptos::{
    attr::global::GlobalAttributes, component, prelude::LeptosOptions,
    tachys::html::element::link, IntoView,
};

/// Injects an [`HTMLLinkElement`](https://developer.mozilla.org/en-US/docs/Web/API/HTMLLinkElement) into the document
/// head that loads a stylesheet from the URL given by the `href` property.
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

/// Injects an [`HTMLLinkElement`](https://developer.mozilla.org/en-US/docs/Web/API/HTMLLinkElement) into the document
/// head that loads a stylesheet from the URL given by the `href` property. The URL is modified to
/// include the computed hash from `cargo-leptos`.
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
///         <HashedStylesheet href="/style.css" options=options />
///       </main>
///     }
/// }
/// ```
#[component]
pub fn HashedStylesheet(
    /// The URL at which the stylesheet is located.
    #[prop(into)]
    href: String,
    options: LeptosOptions,
    /// An ID for the stylesheet.
    #[prop(optional, into)]
    id: Option<String>,
) -> impl IntoView {
    let mut href = href;
    if options.hash_files {
        let hash_path = std::env::current_exe()
            .map(|path| {
                path.parent().map(|p| p.to_path_buf()).unwrap_or_default()
            })
            .unwrap_or_default()
            .join(&options.hash_file);
        if hash_path.exists() {
            let hashes = std::fs::read_to_string(&hash_path)
                .expect("failed to read hash file");
            for line in hashes.lines() {
                let line = line.trim();
                if !line.is_empty() {
                    if let Some((file, hash)) = line.split_once(':') {
                        if file == "css" {
                            if href.ends_with(".css") {
                                href =
                                    href.trim_end_matches(".css").to_string();
                                href.push_str(&format!(".{}.css", hash));
                            } else {
                                href.push_str(&format!(".{}.css", hash));
                            }
                        }
                    }
                }
            }
        }
    }
    // TODO additional attributes
    register(link().id(id).rel("stylesheet").href(href))
}
