use crate::register;
use leptos::{
    attr::global::GlobalAttributes, component, prelude::LeptosOptions,
    tachys::html::element::link, IntoView,
};

/// Injects an [`HTMLLinkElement`](https://developer.mozilla.org/en-US/docs/Web/API/HTMLLinkElement) into the document
/// head that loads a stylesheet from the URL given by the `href` property.
///
/// Note that this does *not* work with the `cargo-leptos` `hash-files` feature: if you are using file
/// hashing, you should use [`HashedStylesheet`](crate::HashedStylesheet).
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

/// Injects an [`HTMLLinkElement`](https://developer.mozilla.org/en-US/docs/Web/API/HTMLLinkElement) into the document head that loads a `cargo-leptos`-hashed stylesheet.
///
/// This should only be used in the application’s server-side `shell` function, as
/// [`LeptosOptions`] is not available in the browser. Unlike other `leptos_meta` components, it
/// will render the `<link>` it creates exactly where it is called.
#[component]
pub fn HashedStylesheet(
    /// Leptos options
    options: LeptosOptions,
    /// An ID for the stylesheet.
    #[prop(optional, into)]
    id: Option<String>,
    /// A base url, not including a trailing slash
    #[prop(optional, into)]
    root: Option<String>,
) -> impl IntoView {
    let mut css_file_name = options.output_name.to_string();
    if options.hash_files {
        let hash_path = std::env::current_exe()
            .map(|path| {
                path.parent().map(|p| p.to_path_buf()).unwrap_or_default()
            })
            .unwrap_or_default()
            .join(options.hash_file.as_ref());
        if hash_path.exists() {
            let hashes = std::fs::read_to_string(&hash_path)
                .expect("failed to read hash file");
            for line in hashes.lines() {
                let line = line.trim();
                if !line.is_empty() {
                    if let Some((file, hash)) = line.split_once(':') {
                        if file == "css" {
                            css_file_name
                                .push_str(&format!(".{}", hash.trim()));
                        }
                    }
                }
            }
        }
    }
    css_file_name.push_str(".css");
    let pkg_path = &options.site_pkg_dir;
    let root = root.unwrap_or_default();

    link()
        .id(id)
        .rel("stylesheet")
        .href(format!("{root}/{pkg_path}/{css_file_name}"))
}
