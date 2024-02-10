use leptos::*;

#[slot]
struct Then {
    a: i32,
}

#[component]
fn SlotIf(then: Then) -> impl IntoView {
    _ = then;
}

fn main() {
    view! {
        <SlotIf>
            <Then slot />
        </SlotIf>
    };
}
