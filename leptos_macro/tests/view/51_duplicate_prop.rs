use leptos::prelude::*;

// Duplicate prop `concrete_bool` — the macro should detect this
// at expansion time and emit `compile_error!`.
//
// Duplicate detection runs on attribute names before any per-prop-kind
// dispatch (see `view/mod.rs`), so a single test covers all prop kinds
// (concrete, optional, generic, `into`). Only slot duplicate detection
// is a separate code path and has its own test (52_slot_duplicate_prop).

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
