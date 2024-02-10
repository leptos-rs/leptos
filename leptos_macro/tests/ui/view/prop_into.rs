use leptos::*;

#[component]
fn Component(#[prop(into)] prop: MaybeSignal<String>) -> impl IntoView {
    _ = prop;
}

fn main() {
    view! {
        <Component prop=move || String::new() />
    };

    let prop = move || String::new();

    view! {
        <Component prop />
    };
}
