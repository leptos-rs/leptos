#[::leptos::component]
fn missing_scope() {}

#[::leptos::component]
fn missing_return_type(cx: ::leptos::Scope) {
    _ = cx;
}

#[::leptos::component]
fn unknown_prop_option(cx: ::leptos::Scope, #[prop(hello)] test: bool) -> impl ::leptos::IntoView {
    _ = cx; 
    _ = test;
}

#[::leptos::component]
fn optional_and_optional_no_strip(
    cx: Scope,
    #[prop(optional, optional_no_strip)] conflicting: bool,
) -> impl IntoView {
    _ = cx;
    _ = conflicting;
}

#[::leptos::component]
fn optional_and_strip_option(
    cx: ::leptos::Scope,
    #[prop(optional, strip_option)] conflicting: bool,
) -> impl ::leptos::IntoView {
    _ = cx;
    _ = conflicting;
}

#[::leptos::component]
fn optional_no_strip_and_strip_option(
    cx: ::leptos::Scope,
    #[prop(optional_no_strip, strip_option)] conflicting: bool,
) -> impl ::leptos::IntoView {
    _ = cx;
    _ = conflicting;
}

#[::leptos::component]
fn default_without_value(
    cx: ::leptos::Scope,
    #[prop(default)] default: bool,
) -> impl ::leptos::IntoView {
    _ = cx;
    _ = default;
}

#[::leptos::component]
fn default_with_invalid_value(
    cx: ::leptos::Scope,
    #[prop(default= |)] default: bool,
) -> impl ::leptos::IntoView {
    _ = cx;
    _ = default;
}

#[::leptos::component]
pub fn using_the_view_macro(cx: ::leptos::Scope) -> impl ::leptos::IntoView {
    _ = cx;
    ::leptos::view! { cx,
        "ok"
    }
}

fn main() {}
