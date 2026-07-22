use leptos::{component, config::LeptosOptions, html::base, IntoView};

/// A component that sets the <base> property of an HTML page.
#[component]
pub fn Base(
    /// Leptos options, which potentially contains a `site_base` that
    /// we use as the `href` for the `<base>` element.
    options: LeptosOptions,
) -> impl IntoView {
    base().href(options.site_base)
}
