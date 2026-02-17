use leptos::prelude::*;

// Wrong type passed for a `#[prop(into)]` prop.
// `Vec<i32>` does not implement `IntoReactiveValue<String, _>`.
// The error currently spans the entire `view! { ... }` block rather
// than the specific value expression. Localizing the span to the
// value is a known limitation.
// TODO: investigate localizing the error span to the value expression.

#[component]
fn PropIntoInvalidType() -> impl IntoView {
    view! {
        <div>
            <Inner label=vec![1, 2, 3]/>
        </div>
    }
}

#[component]
fn Inner(#[prop(into)] label: String) -> impl IntoView {
    let _ = label;
    ()
}

fn main() {}
