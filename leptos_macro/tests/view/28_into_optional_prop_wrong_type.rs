use leptos::prelude::*;

// Wrong type passed for a `#[prop(into, optional)]` prop.
// `Vec<i32>` does not implement `Into<String>`.
// The error should point to the value expression (`vec![1, 2, 3]`).

#[component]
fn PropIntoOptionalInvalidType() -> impl IntoView {
    view! {
        <div>
            <Inner required=42 val=vec![1, 2, 3]/>
        </div>
    }
}

#[component]
fn Inner(
    required: i32,
    #[prop(into, optional)] val: Option<String>,
) -> impl IntoView {
    let _ = required;
    let _ = val;
    ()
}

fn main() {}
