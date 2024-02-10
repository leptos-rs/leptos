use leptos::*;

#[component]
fn Component<T: Into<String>>(prop: T) -> impl IntoView {
    _ = prop;
}

fn main() {
    view! {
        <Component<i32> prop=0 />
    };

    view! {
        <Component prop=0 />
    };
}
