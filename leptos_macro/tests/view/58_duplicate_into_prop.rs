// Duplicate `#[prop(into)]` prop.
// Into props go through a different setter code path (setter name
// carries value span). This test ensures duplicate detection fires
// before the setter runs.

use leptos::prelude::*;

#[component]
fn DuplicateIntoProp() -> impl IntoView {
    view! {
        <div>
            <Inner label="hello" label="world"/>
        </div>
    }
}

#[component]
fn Inner(#[prop(into)] label: String) -> impl IntoView {
    let _ = label;
    ()
}

fn main() {}
