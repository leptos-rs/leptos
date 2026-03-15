// Two components in same `view!`, only one has an error.
// Verifies that errors on `Beta` don't bleed into `Alpha`.

use leptos::prelude::*;

#[component]
fn TwoComponentsOneError() -> impl IntoView {
    view! {
        <div>
            <Alpha count=42/>
            <Beta count="wrong"/>
        </div>
    }
}

#[component]
fn Alpha(count: i32) -> impl IntoView {
    let _ = count;
    ()
}

#[component]
fn Beta(count: i32) -> impl IntoView {
    let _ = count;
    ()
}

fn main() {}
