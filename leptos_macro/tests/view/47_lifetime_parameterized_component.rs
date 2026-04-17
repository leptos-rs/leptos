// Component with a lifetime parameter. Should compile without errors.

use leptos::prelude::*;

#[component]
fn LifetimeUsage() -> impl IntoView {
    let data = "hello";
    view! {
        <div>
            <Inner label=data/>
        </div>
    }
}

#[component]
fn Inner<'a>(label: &'a str) -> impl IntoView {
    view! { <span>{label.to_owned()}</span> }
}

fn main() {}
