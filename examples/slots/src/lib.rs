use leptos::*;

#[slot]
struct Then {
    children: ChildrenFn,
}

#[slot]
struct ElseIf {
    cond: MaybeSignal<bool>,
    children: ChildrenFn,
}

#[slot]
struct Fallback {
    children: ChildrenFn,
}

#[component]
fn SlotIf(
    cx: Scope,
    cond: MaybeSignal<bool>,
    then: Then,
    #[prop(into, optional)] else_if: Option<Vec<ElseIf>>,
    #[prop(optional)] fallback: Option<Fallback>,
) -> impl IntoView {
    move || {
        if cond() {
            (then.children)(cx).into_view(cx)
        } else {
            if let Some(else_if) = &else_if {
                if let Some(else_if) =
                    else_if.iter().find(|else_if| (else_if.cond)())
                {
                    return (else_if.children)(cx).into_view(cx);
                }
            }

            if let Some(fallback) = &fallback {
                (fallback.children)(cx).into_view(cx)
            } else {
                ().into_view(cx)
            }
        }
    }
}

#[component]
pub fn App(cx: Scope) -> impl IntoView {
    let (count, set_count) = create_signal(cx, 0);
    let is_even = MaybeSignal::derive(cx, move || count() % 2 == 0);
    let is_div5 = MaybeSignal::derive(cx, move || count() % 5 == 0);
    let is_div7 = MaybeSignal::derive(cx, move || count() % 7 == 0);

    view! { cx,
        <button on:click=move |_| set_count.update(|value| *value += 1)>"+1"</button>
        " "{count}" is "
        <SlotIf cond=is_even>
            <Then slot>"even"</Then>
            <ElseIf slot cond=is_div5>"divisible by 5"</ElseIf>
            <ElseIf slot cond=is_div7>"divisible by 7"</ElseIf>
            <Fallback slot>"odd"</Fallback>
        </SlotIf>
    }
}
