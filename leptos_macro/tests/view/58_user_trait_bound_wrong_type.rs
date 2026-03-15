// Generic prop bounded by a user-defined trait, but the wrong type
// is passed (`i32` does not implement `Greetable`).
// Verifies `on_unimplemented` shows the user-defined trait name
// and the Fn hint is NOT appended.

use leptos::prelude::*;

mod my_traits {
    pub trait Greetable {
        fn greet(&self) -> String;
    }
}

use my_traits::Greetable;

#[component]
fn UserTraitBoundWrongType() -> impl IntoView {
    view! {
        <div>
            <Inner value=42/>
        </div>
    }
}

#[component]
fn Inner<T: Greetable>(value: T) -> impl IntoView {
    let _ = value;
    ()
}

fn main() {}
