// An empty view! invocation returning `()`.
// This should compile without errors.

use leptos::prelude::*;

#[component]
fn Empty() -> () {
    view! {}
}

fn main() {}
