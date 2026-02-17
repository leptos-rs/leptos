use leptos::prelude::*;

// Wrong type passed for an `#[prop(optional)]` prop.
// We expect the error to be on the value `"not_a_bool"`.

#[component]
fn OptionalPropInvalidType() -> impl IntoView {
    view! {
        <div>
            <Inner required=42 optional_flag="not_a_bool"/>
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
