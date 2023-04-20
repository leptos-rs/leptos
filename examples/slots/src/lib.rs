use leptos::*;

#[slot]
struct Then {
    children: ChildrenFn,
}

#[slot]
struct Fallback {
    children: ChildrenFn,
}

#[component]
fn SlotIf<C>(
    cx: Scope,
    cond: C,
    then: Then,
    #[prop(optional)] fallback: Option<Fallback>,
) -> impl IntoView
where
    C: Fn() -> bool + 'static,
{
    move || {
        if (cond)() {
            (then.children)(cx).into_view(cx)
        } else if let Some(fallback) = &fallback {
            (fallback.children)(cx).into_view(cx)
        } else {
            ().into_view(cx)
        }
    }
}

#[component]
pub fn App(cx: Scope) -> impl IntoView {
    let (count, set_count) = create_signal(cx, 0);
    view! { cx,
        <button on:click=move |_| set_count.update(|value| *value += 1)>"+1"</button>
        <br/>
        <SlotIf cond=move || count() % 2 == 0>
            <Then slot>{count()}" is even"</Then>
            <Fallback slot>{count()}" is odd"</Fallback>
        </SlotIf>
    }
}
