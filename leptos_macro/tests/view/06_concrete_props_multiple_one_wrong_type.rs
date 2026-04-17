use leptos::prelude::*;

// Component with two concrete props (`concrete_bool: bool`, `concrete_i32: i32`).
// The `concrete_bool` prop is passed using shorthand with a wrong-type variable
// (`let concrete_bool = 42`), while `concrete_i32` is passed correctly.
// We expect the error on the prop key `concrete_bool` (shorthand has no
// separate value token). We do not expect an error to be reported at any
// other location.

#[component]
fn InvalidPropPassed() -> impl IntoView {
    let concrete_bool = 42;

    view! {
        <div>
            <Inner concrete_bool concrete_i32=3/>
        </div>
    }
}

#[component]
fn Inner(concrete_bool: bool, concrete_i32: i32) -> impl IntoView {
    let _ = concrete_bool;
    let _ = concrete_i32;
    ()
}

fn main() {}
