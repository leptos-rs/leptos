use leptos::*;

#[slot]
struct Then {}

#[slot]
struct ElseIf {}

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
            <ElseIf slot:then />
        </If>
    };

    view! {
        <SlotIf>
            <ElseIf slot:then />
        </SlotIf>
    };
}
