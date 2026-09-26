use leptos::{component, config::LeptosOptions, html::base, IntoView};

/// A component that sets the <base> property of an HTML page.
#[component]
pub fn Base(
    /// Leptos options, which potentially contains a `site_base` that
    /// informs the `href` for the `<base>` element.
    options: LeptosOptions,
) -> impl IntoView {
    let site_base = if options.site_base.is_empty() {
        String::new()
    } else {
        // Ensure that `base` always has a trailing slash, so that
        // relative URLs work
        format!("{}/", options.site_base.trim_end_matches('/'))
    };

    base().href(site_base)
}
