use leptos::*;

#[slot]
struct Then {
    children: Children,
}

#[component]
fn Component(children: Children) -> impl IntoView {
    _ = children;
}

#[component]
fn Slot(then: Then) -> impl IntoView {
    _ = then;
}

struct A;

fn main() {
    let a = A;

    view! {
        <Component clone:a>
            <p />
        </Component>
    };

    view! {
        <Slot>
            <Then slot clone:a>
                <p/>
            </Then>
        </Slot>
    };
}
