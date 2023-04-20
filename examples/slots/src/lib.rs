use leptos::*;

// Slots are created in simillar manner to components, except that they use the #[slot] macro.
#[slot]
struct Then {
    children: ChildrenFn,
}

// Props work just like component props, for example, you can specify a prop as optional by prefixing
// the type with Option<...> and marking the option as #[prop(optional)].
#[slot]
struct ElseIf {
    cond: MaybeSignal<bool>,
    children: ChildrenFn,
}

#[slot]
struct Fallback {
    children: ChildrenFn,
}

// Slots are added to components like any other prop, with a single caveat for now:
//     If you want to have 0+ of the same prop, you have to also mark it as #[prop(into)], instead of
//     just #[prop(optional)].
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
            // The slot name can be emitted if it would match the slot struct name (in snake case).
            <Then slot>"even"</Then>
            // Props are passed just like on normal components.
            <ElseIf slot cond=is_div5>"divisible by 5"</ElseIf>
            <ElseIf slot cond=is_div7>"divisible by 7"</ElseIf>
            <Fallback slot>"odd"</Fallback>
        </SlotIf>
    }
}
