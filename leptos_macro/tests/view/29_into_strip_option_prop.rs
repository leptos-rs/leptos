use leptos::prelude::*;

// Component with `#[prop(into, strip_option)]`. Callers pass a type that
// implements `Into<T>` (e.g. `&str` for `String`), and it gets wrapped in
// `Some`. This should compile without errors.

#[component]
fn PropIntoStripOptionProvided() -> impl IntoView {
    view! {
        <div>
            <Inner required=42 val="hello"/>
        </div>
    }
}

#[component]
fn Inner(
    required: i32,
    #[prop(into, strip_option)] val: Option<String>,
) -> impl IntoView {
    let _ = required;
    let _ = val;
    ()
}

fn main() {}
