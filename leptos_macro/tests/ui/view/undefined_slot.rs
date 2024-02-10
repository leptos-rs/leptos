use leptos::*;

#[slot]
struct Then {}

#[slot]
struct ElseIf {
    #[prop(optional)]
    then: Option<Then>,
}

#[component]
fn If(then: Then) -> impl IntoView {
    _ = then;
}

#[component]
fn SlotIf(then: Then) -> impl IntoView {
    _ = then;
}

fn main() {
    view! {
        <If>
            <ElseIf slot />
        </If>
    };

    view! {
        <SlotIf>
            <ElseIf slot />
        </SlotIf>
    };
}
