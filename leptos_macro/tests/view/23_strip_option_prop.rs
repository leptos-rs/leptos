use leptos::prelude::*;

// Component with `#[prop(strip_option)]`. Callers pass the inner type
// directly (e.g. `u8` instead of `Option<u8>`), and it gets wrapped in
// `Some`. This should compile without errors.

#[component]
fn StripOptionProvided() -> impl IntoView {
    view! {
        <div>
            <Inner required=42 val=9/>
        </div>
    }
}

#[component]
fn Inner(
    required: i32,
    #[prop(strip_option)] val: Option<u8>,
) -> impl IntoView {
    let _ = required;
    let _ = val;
    ()
}

fn main() {}
