use leptos::prelude::*;

// Component with `#[prop(into)]`, invoked with a type that implements the
// required `Into` conversion (`&str` -> `String`). This should compile
// without errors.

#[component]
fn PropIntoConversion() -> impl IntoView {
    view! {
        <div>
            <Inner label="hello"/>
        </div>
    }
}

#[component]
fn Inner(#[prop(into)] label: String) -> impl IntoView {
    let _ = label;
    ()
}

fn main() {}
