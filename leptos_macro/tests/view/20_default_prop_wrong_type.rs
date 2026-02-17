use leptos::prelude::*;

// Wrong type passed for a `#[prop(default = ...)]` prop.
// We expect the error to be on the value `"not_an_i32"`.

#[component]
fn DefaultPropInvalidType() -> impl IntoView {
    view! {
        <div>
            <Inner required=42 defaulted="not_an_i32"/>
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
