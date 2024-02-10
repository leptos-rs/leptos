use leptos::*;

#[slot]
struct Then {
    children: Children,
}

#[component]
fn Component(then: Then) -> impl IntoView {
    _ = then;
}

fn main() {
    view! {
        <Component>
            <Then slot let:a>
                <p />
            </Then>
        </Component>
    };
}
