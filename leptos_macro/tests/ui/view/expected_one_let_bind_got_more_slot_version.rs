use leptos::*;

#[slot]
struct Then<C, IV>
where
    C: Fn(String) -> IV,
    IV: IntoView,
{
    children: C,
}

#[component]
fn Slot<C, IV>(then: Then<C, IV>) -> impl IntoView
where
    C: Fn(String) -> IV,
    IV: IntoView,
{
    _ = then;
}

fn main() {
    view! {
        <Slot>
            <Then
                slot
                let:item
                let:extra_item
            >
                <p>{item}</p>
            </Then>
        </Slot>
    };
}
