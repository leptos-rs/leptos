use leptos::{component, config::LeptosOptions, html::base, IntoView};

/// A component that sets the <base> property of an HTML page.
#[component]
pub fn Base(
    /// Leptos options, which potentially contains a `site_base` that
    /// informs the `href` for the `<base>` element.
    options: LeptosOptions,
) -> impl IntoView {
    let base_href = if options.site_base.is_empty() {
        format!("{}/", options.site_addr)
    } else {
        // Handles `href` for a variety of `site_base` arrangements
        // outside of the expected `foo` (no leading/trailing slashes),
        // e.g. `/`, `/foo`, `/foo/`
        format!(
            "{}/{}/",
            options.site_addr,
            options.site_base.trim_matches("/")
        )
    };

    base().href(base_href)
}
