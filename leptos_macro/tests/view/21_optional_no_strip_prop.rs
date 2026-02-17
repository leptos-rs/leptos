use leptos::prelude::*;

// Component with `#[prop(optional_no_strip)]` prop. Unlike `optional`,
// this keeps the `Option<T>` wrapper so callers must pass `Some(...)` or
// omit the prop entirely. This should compile without errors.

#[component]
fn OptionalNoStripProvided() -> impl IntoView {
    view! {
        <div>
            <Inner required=42 val=Some("hi".into())/>
        </div>
    }
}

#[component]
fn OptionalNoStripOmitted() -> impl IntoView {
    view! {
        <div>
            <Inner required=42/>
        </div>
    }
}

#[component]
fn Inner(
    required: i32,
    #[prop(optional_no_strip)] val: Option<String>,
) -> impl IntoView {
    let _ = required;
    let _ = val;
    ()
}

fn main() {}
