use leptos::*;

#[component]
fn Component(children: Children) -> impl IntoView {
    _ = children;
}

fn main() {
    view! {
        <Component let:a>
            <p />
        </Component>
    };
}
