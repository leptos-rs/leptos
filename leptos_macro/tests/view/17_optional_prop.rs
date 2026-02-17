use leptos::prelude::*;

// Component with `#[prop(optional)]` prop, invoked without providing the
// optional prop. This should compile without errors.

#[component]
fn OptionalPropOmitted() -> impl IntoView {
    view! {
        <div>
            <Inner required=42/>
        </div>
    }
}

#[component]
fn Inner(
    required: i32,
    #[prop(optional)] optional_flag: bool,
) -> impl IntoView {
    let _ = required;
    let _ = optional_flag;
    ()
}

fn main() {}
