// Multiple components with the same prop names defined in the same module.
// Ensures marker traits (__required_Comp_foo) don't collide across
// components. Should compile without errors.

use leptos::prelude::*;

#[component]
fn Usage() -> impl IntoView {
    view! {
        <div>
            <Alpha label="hello".to_string() count=1/>
            <Beta label="world".to_string() count=2/>
        </div>
    }
}

#[component]
fn Alpha(label: String, count: i32) -> impl IntoView {
    let _ = label;
    let _ = count;
}

#[component]
fn Beta(label: String, count: i32) -> impl IntoView {
    let _ = label;
    let _ = count;
}

fn main() {}
