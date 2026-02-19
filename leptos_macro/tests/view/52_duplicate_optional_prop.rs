use leptos::prelude::*;

// Duplicate optional prop — same detection as required props.

#[component]
fn DuplicateOptionalProp() -> impl IntoView {
    view! {
        <div>
            <Inner optional_prop=1 optional_prop=2/>
        </div>
    }
}

#[component]
fn Inner(
    #[prop(optional)] optional_prop: Option<i32>,
) -> impl IntoView {
    let _ = optional_prop;
    ()
}

fn main() {}
