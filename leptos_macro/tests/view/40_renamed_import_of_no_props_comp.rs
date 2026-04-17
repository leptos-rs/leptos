// The component and view macro should still be able to work together when another component used in
// the view macro was used through a renamed identifier.

use leptos::prelude::*;
use my_mod::Inner as MyOtherComp;

#[component]
fn UsingRenamedComponent() -> impl IntoView {
    view! {
        <div>
            <MyOtherComp />
        </div>
    }
}

mod my_mod {
    use leptos::prelude::*;

    #[component]
    pub fn Inner() -> impl IntoView {
        ()
    }
}

fn main() {}
