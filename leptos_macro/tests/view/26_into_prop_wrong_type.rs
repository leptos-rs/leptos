use leptos::prelude::*;

// Wrong type passed for a `#[prop(into)]` prop.
// `Vec<i32>` does not implement `IntoReactiveValue<String, _>`.
// The error should point to the value expression (`vec![1, 2, 3]`).

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
