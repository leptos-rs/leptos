use leptos::*;

#[component]
fn Component() -> impl IntoView {}
fn highlight(el: HtmlElement<html::AnyElement>, prop: i32) {
    _ = prop;
}

fn main() {
    let data = "Hello World!";

    view! {
        <Component use:highlight />
    };

    view! {
        <Component use:highlight="asd" />
    };

    view! {
        <Component use:highlight=data />
    };
}
