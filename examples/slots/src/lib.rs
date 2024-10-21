use leptos::prelude::*;

// Slots are created in similar manner to components, except that they use the #[slot] macro.
#[slot]
struct Then {
    children: ChildrenFn,
}

// Props work just like component props, for example, you can specify a prop as optional by prefixing
// the type with Option<...> and marking the option as #[prop(optional)].
#[slot]
struct ElseIf {
    cond: Signal<bool>,
    children: ChildrenFn,
}

#[slot]
struct Fallback {
    children: ChildrenFn,
}

// Slots are added to components like any other prop.
#[component]
fn SlotIf(
    cond: Signal<bool>,
    then: Then,
    #[prop(default=vec![])] else_if: Vec<ElseIf>,
    #[prop(optional)] fallback: Option<Fallback>,
) -> impl IntoView {
    move || {
        if cond.get() {
            (then.children)().into_any()
        } else if let Some(else_if) = else_if.iter().find(|i| i.cond.get()) {
            (else_if.children)().into_any()
        } else if let Some(fallback) = &fallback {
            (fallback.children)().into_any()
        } else {
            ().into_any()
        }
    }
}

#[component]
pub fn App() -> impl IntoView {
    let (count, set_count) = signal(0);
    let is_even = Signal::derive(move || count.get() % 2 == 0);
    let is_div5 = Signal::derive(move || count.get() % 5 == 0);
    let is_div7 = Signal::derive(move || count.get() % 7 == 0);

    view! {
        <button on:click=move |_| set_count.update(|value| *value += 1)>"+1"</button>
        " "{count}" is "
        <SlotIf cond=is_even>
            // The slot name can be emitted if it would match the slot struct name (in snake case).
            <Then slot>"even"</Then>
            // Props are passed just like on normal components.
            <ElseIf slot cond=is_div5>"divisible by 5"</ElseIf>
            <ElseIf slot cond=is_div7>"divisible by 7"</ElseIf>
            <Fallback slot>"odd"</Fallback>
        </SlotIf>
    }
}
