use leptos::prelude::*;

#[cfg(all(feature = "nightly", rustc_nightly))]
#[component]
fn missing_scope() {}

#[cfg(all(feature = "nightly", rustc_nightly))]
#[component]
fn missing_return_type() {}

#[cfg(all(feature = "nightly", rustc_nightly))]
#[component]
fn unknown_prop_option(#[prop(hello)] test: bool) -> impl IntoView {
    _ = test;
}

#[cfg(all(feature = "nightly", rustc_nightly))]
#[component]
fn optional_and_optional_no_strip(
    #[prop(optional, optional_no_strip)] conflicting: bool,
) -> impl IntoView {
    _ = conflicting;
}

#[cfg(all(feature = "nightly", rustc_nightly))]
#[component]
fn optional_and_strip_option(
    #[prop(optional, strip_option)] conflicting: bool,
) -> impl IntoView {
    _ = conflicting;
}

#[cfg(all(feature = "nightly", rustc_nightly))]
#[component]
fn optional_no_strip_and_strip_option(
    #[prop(optional_no_strip, strip_option)] conflicting: bool,
) -> impl IntoView {
    _ = conflicting;
}

#[cfg(all(feature = "nightly", rustc_nightly))]
#[component]
fn default_without_value(#[prop(default)] default: bool) -> impl IntoView {
    _ = default;
}

#[cfg(all(feature = "nightly", rustc_nightly))]
#[component]
fn default_with_invalid_value(
    #[prop(default= |)] default: bool,
) -> impl IntoView {
    _ = default;
}

#[cfg(all(feature = "nightly", rustc_nightly))]
#[component]
fn destructure_without_name((default, value): (bool, i32)) -> impl IntoView {
    _ = default;
    _ = value;
}

fn main() {}
