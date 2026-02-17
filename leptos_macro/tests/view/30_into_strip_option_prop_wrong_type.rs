use leptos::prelude::*;

// Wrong type passed for a `#[prop(into, strip_option)]` prop.
// `Vec<i32>` does not implement `Into<String>`.
// The error currently spans the entire `view! { ... }` block rather
// than the specific value expression (known limitation).
// TODO: investigate localizing the error span to the value expression.

#[component]
fn PropIntoStripOptionInvalidType() -> impl IntoView {
    view! {
        <div>
            <Inner required=42 val=vec![1, 2, 3]/>
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
