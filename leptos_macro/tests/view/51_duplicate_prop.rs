use leptos::prelude::*;

// Duplicate prop `concrete_bool` — the macro should detect this
// at expansion time and emit `compile_error!`.

#[component]
fn DuplicateProp() -> impl IntoView {
    view! {
        <div>
            <Inner concrete_bool=true concrete_bool=false/>
        </div>
    }
}

#[component]
fn Inner(concrete_bool: bool) -> impl IntoView {
    let _ = concrete_bool;
    ()
}

fn main() {}
