use leptos::*;

#[derive(::leptos::typed_builder::TypedBuilder)]
struct Slot {
    #[builder(default, setter(strip_option))]
    children: Option<Children>,
}

#[component]
fn Slottable(
    cx: Scope,
    slot: Slot,
    #[prop(optional)] children: Option<Children>,
) -> impl IntoView {
    let _children = children;

    view! { cx,
        {if let Some(children) = slot.children {
            (children)(cx).into_view(cx)
        } else {
            ().into_view(cx)
        }}
    }
}

#[component]
pub fn App(cx: Scope) -> impl IntoView {
    view! { cx,
        <Slottable>
            <Slot slot:slot>
                <h1>"Hello, World!"</h1>
            </Slot>
        </Slottable>
    }
}
