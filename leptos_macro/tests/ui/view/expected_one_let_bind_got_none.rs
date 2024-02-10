use leptos::*;

#[slot]
struct Then<T, C, RC>
where
    C: Fn(T) -> RC,
    RC: IntoView,
{
    data: T,
    children: C,
}

#[component]
fn Slot<T, C, RC>(then: Then<T, C, RC>) -> impl IntoView
where
    C: Fn(T) -> RC,
    RC: IntoView,
{
    _ = then;
}

#[component]
fn Component<T, C, RC>(data: T, children: C) -> impl IntoView
where
    C: Fn(T) -> RC,
    RC: IntoView,
{
    _ = data;
    _ = children;
}

fn main() {
    view! {
        <Component
            data=0
        >
            <p/>
        </Component>
    };

    view! {
        <Slot>
            <Then
                slot
                data=0
            >
                <p/>
            </Then>
        </Slot>
    };
}
