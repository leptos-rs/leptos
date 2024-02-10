use leptos::*;

#[slot]
struct Then {}

#[component]
fn SlotIf(then: Then) -> impl IntoView {
    _ = then;
}

#[component]
fn If(then: Then) -> impl IntoView {
    _ = then;
}

fn main() {
    view! {
        <If>
            <Then slot />
            <Then slot />
        </If>
    };

    view! {
        <SlotIf>
            <Then slot />
            <Then slot />
        </SlotIf>

    };
}
