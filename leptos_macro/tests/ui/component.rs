use leptos::*;

#[component]
fn missing_scope() {}

#[component]
fn missing_return_type(cx: Scope) {
    _ = cx;
}

#[component]
fn unknown_prop_option(cx: Scope, #[prop(hello)] test: bool) -> impl IntoView {
    _ = cx;
    _ = test;
}

#[component]
fn optional_and_optional_no_strip(
    cx: Scope,
    #[prop(optional, optional_no_strip)] conflicting: bool,
) -> impl IntoView {
    _ = cx;
    _ = conflicting;
}

#[component]
fn optional_and_strip_option(
    cx: Scope,
    #[prop(optional, strip_option)] conflicting: bool,
) -> impl IntoView {
    _ = cx;
    _ = conflicting;
}

#[component]
fn optional_no_strip_and_strip_option(
    cx: Scope,
    #[prop(optional_no_strip, strip_option)] conflicting: bool,
) -> impl IntoView {
    _ = cx;
    _ = conflicting;
}

#[component]
fn default_without_value(
    cx: Scope,
    #[prop(default)] default: bool,
) -> impl IntoView {
    _ = cx;
    _ = default;
}

#[component]
fn default_with_invalid_value(
    cx: Scope,
    #[prop(default= |)] default: bool,
) -> impl IntoView {
    _ = cx;
    _ = default;
}

fn main() {}
