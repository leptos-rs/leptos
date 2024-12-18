use leptos::prelude::*;

#[component]
fn missing_scope() {}

#[component]
fn missing_return_type() {}

#[component]
fn unknown_prop_option(#[prop(hello)] test: bool) -> impl IntoView {
    _ = test;
}

#[component]
fn optional_and_optional_no_strip(
    #[prop(optional, optional_no_strip)] conflicting: bool,
) -> impl IntoView {
    _ = conflicting;
}

#[component]
fn optional_and_strip_option(
    #[prop(optional, strip_option)] conflicting: bool,
) -> impl IntoView {
    _ = conflicting;
}

#[component]
fn optional_no_strip_and_strip_option(
    #[prop(optional_no_strip, strip_option)] conflicting: bool,
) -> impl IntoView {
    _ = conflicting;
}

#[component]
fn default_without_value(#[prop(default)] default: bool) -> impl IntoView {
    _ = default;
}

#[component]
fn default_with_invalid_value(
    #[prop(default= |)] default: bool,
) -> impl IntoView {
    _ = default;
}

#[component]
fn destructure_without_name((default, value): (bool, i32)) -> impl IntoView {
    _ = default;
    _ = value;
}

fn main() {}
