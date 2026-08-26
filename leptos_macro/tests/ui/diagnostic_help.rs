use leptos_macro::{component, island, slot, view};

fn invalid_view() {
    let _ = view! { class = "scope" <div/> };
}

#[component(unsupported)]
fn invalid_component_argument() -> impl leptos::prelude::IntoView {}

#[island(unsupported)]
fn invalid_island_argument() -> impl leptos::prelude::IntoView {}

#[slot(unsupported)]
struct InvalidSlotArgument;

#[component]
fn invalid_option(
    #[prop(strip_option, into)] value: bool,
) -> impl leptos::prelude::IntoView {
    value.to_string()
}

fn main() {}
