use leptos::prelude::*;

// Component with `#[prop(default = ...)]`, invoked without providing the
// defaulted prop. This should compile without errors, using the default value.

#[component]
fn DefaultPropOmitted() -> impl IntoView {
    view! {
        <div>
            <Inner required=42/>
        </div>
    }
}

#[component]
fn Inner(
    required: i32,
    #[prop(default = 10)] defaulted: i32,
) -> impl IntoView {
    let _ = required;
    let _ = defaulted;
    ()
}

fn main() {}
