use leptos::prelude::*;

// Component with `#[prop(into, optional)]`. Callers can pass a type that
// implements `Into<T>` (e.g. `&str` for `String`), or omit the prop
// entirely. This should compile without errors.

#[component]
fn PropIntoOptionalProvided() -> impl IntoView {
    view! {
        <div>
            <Inner required=42 val="hello"/>
        </div>
    }
}

#[component]
fn PropIntoOptionalOmitted() -> impl IntoView {
    view! {
        <div>
            <Inner required=42/>
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
