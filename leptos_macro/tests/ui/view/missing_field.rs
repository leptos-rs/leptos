use leptos::*;

#[component]
fn Component(prop: i32) -> impl IntoView {
    _ = prop;
}

fn main() {
    view! {
        <Component />
    };
}
