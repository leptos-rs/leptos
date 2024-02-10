use leptos::*;

#[slot]
struct Then {
    children: ChildrenFn,
}

#[slot]
struct ElseIf {
    #[prop(into)]
    cond: MaybeSignal<bool>,
    children: ChildrenFn,
}

#[component]
fn SlotIf(
    #[prop(into)] cond: MaybeSignal<bool>,
    then: Then,
    #[prop(optional)] else_if: Vec<ElseIf>,
) -> impl IntoView {
    _ = cond;
    _ = then;
    _ = else_if;
}

fn main() {
    let (count, set_count) = create_signal(0);
    let is_even = MaybeSignal::derive(move || count.get() % 2 == 0);
    let is_div5 = move || count.get() % 5 == 0;

    view! {
        <SlotIf cond=is_even>
            <Then slot>"even"</Then>
            <ElseIf slot cond=is_div5>"divisible by 5"</ElseIf>
        </SlotIf>
    };
}
