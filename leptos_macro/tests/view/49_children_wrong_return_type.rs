use leptos::prelude::*;

// This test provides children that are a non-renderable type.
// We expect the `IntoRender` `on_unimplemented` message:
// "`NotAView` cannot be rendered as a view element"

struct NotAView;

#[component]
fn WrongChildrenType() -> impl IntoView {
    view! {
        <div>
            <Inner>
                {NotAView}
            </Inner>
        </div>
    }
}

#[component]
fn Inner(children: Children) -> impl IntoView {
    children()
}

fn main() {}
