// The component and view macro should still be able to work together when another component used in
// the view macro was used through a renamed identifier.
// Using a component with props requires usage of the components' builder, which makes this test
// differ from the previous one.

use leptos::prelude::*;
use my_mod::Inner as MyOtherComp;

#[component]
fn UsingRenamedComponentTakingProps() -> impl IntoView {
    view! {
        <div>
            <MyOtherComp concrete_i32=42 />
        </div>
    }
}

mod my_mod {
    use leptos::prelude::*;

    #[component]
    pub fn Inner(concrete_i32: i32) -> impl IntoView {
        let _ = concrete_i32;
        ()
    }
}

fn main() {}
