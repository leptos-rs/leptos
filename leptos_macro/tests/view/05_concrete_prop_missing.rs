use leptos::prelude::*;

// This test fails to pass a value for the required `concrete_bool` prop.
// We expect the error to be on the component name, `Inner` in this case.
// We do not expect an error to be reported at any other location.

#[component]
fn MissingRequiredProp() -> impl IntoView {
    view! {
        <div>
            <Inner/>
        </div>
    }
}

#[component]
fn Inner(concrete_bool: bool) -> impl IntoView {
    let _ = concrete_bool;
    ()
}

fn main() {}
