// Closures with untyped parameters should compile when the component's
// generic bound provides enough information for type inference.
//
// The pre-check block must be inlined into the builder setter argument
// so that Rust's bidirectional type inference propagates the expected
// type from the setter into the closure.

use leptos::prelude::*;

#[component]
fn Inner<F: Fn(i32) -> String + 'static>(transform: F) -> impl IntoView {
    let _ = transform;
}

fn main() {
    // Closure with UNTYPED parameter: Type is inferred from Fn(i32) -> String bound, no error.
    let _ = view! {
        <Inner transform=|x| x.to_string() />
    };
}
