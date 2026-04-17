use leptos::prelude::*;

// Children return a non-renderable type AND a required prop is missing.
// We expect both errors to be reported independently.

struct NotAView;

#[component]
fn WrongChildrenAndMissingProp() -> impl IntoView {
    view! {
        <div>
            <Inner>
                {NotAView}
            </Inner>
        </div>
    }
}

#[component]
fn Inner(some_prop: i32, children: Children) -> impl IntoView {
    let _ = some_prop;
    children()
}

fn main() {}
