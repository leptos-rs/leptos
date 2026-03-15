// Generic prop bounded by a user-defined trait imported via `use`.
// The companion module system generates the check trait impl
// *outside* the module, so it should see `use`-imported traits.
// This test confirms correct usage compiles.

use leptos::prelude::*;

mod my_traits {
    pub trait Greetable {
        fn greet(&self) -> String;
    }

    impl Greetable for String {
        fn greet(&self) -> String {
            format!("Hello, {self}!")
        }
    }
}

use my_traits::Greetable;

#[component]
fn UserTraitBoundCorrect() -> impl IntoView {
    view! {
        <div>
            <Inner value="world".to_string()/>
        </div>
    }
}

#[component]
fn Inner<T: Greetable>(value: T) -> impl IntoView {
    let _ = value.greet();
    ()
}

fn main() {}
