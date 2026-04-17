use leptos::prelude::*;

// This test passes a value of incorrect type for the `concrete_bool` prop.
// We expect the error to be on the value itself (right-hand side of `=`), `42` in this case.
// We do not expect an error to be reported at any other location.

#[component]
fn InvalidPropPassed() -> impl IntoView {
    view! {
        <div>
            <Inner concrete_bool=42/>
        </div>
    }
}

#[component]
fn Inner(concrete_bool: bool) -> impl IntoView {
    let _ = concrete_bool;
    ()
}

fn main() {}
