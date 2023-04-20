use leptos::*;

#[derive(::leptos::typed_builder::TypedBuilder)]
struct Then {
    children: Box<dyn Fn(Scope) -> Fragment>,
}

#[derive(::leptos::typed_builder::TypedBuilder)]
struct Else {
    children: Box<dyn Fn(Scope) -> Fragment>,
}

#[component]
fn SlotIf<C>(cx: Scope, cond: C, then: Then, else_: Else) -> impl IntoView
where
    C: Fn() -> bool + 'static,
{
    move || {
        if (cond)() {
            (then.children)(cx)
        } else {
            (else_.children)(cx)
        }
    }
}

#[component]
pub fn App(cx: Scope) -> impl IntoView {
    let (toggle, set_toggle) = create_signal(cx, false);

    view! { cx,
        <button on:click=move |_| set_toggle.update(|value| *value = !*value)>"Toggle"</button>

        <SlotIf cond=toggle>
            <Then slot:then>" True"</Then>
            <Else slot:else_>" False"</Else>
        </SlotIf>
    }
}
