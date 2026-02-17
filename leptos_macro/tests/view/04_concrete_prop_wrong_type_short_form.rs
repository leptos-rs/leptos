use leptos::prelude::*;

// This test passes a value of incorrect type for the `concrete_bool` prop, using the shorthand
// form available when an equally named variable is in scope, consuming the value of the variable.
// We expect the error to be on the prop key this time (left-hand side of `=`), `concrete_bool`
// in this case.
// We do not expect an error to be reported at any other location.

#[component]
fn InvalidPropPassed() -> impl IntoView {
    let concrete_bool = 42;

    view! {
        <div>
            <Inner concrete_bool/>
        </div>
    }
}

#[component]
fn Inner(concrete_bool: bool) -> impl IntoView {
    let _ = concrete_bool;
    ()
}

fn main() {}
