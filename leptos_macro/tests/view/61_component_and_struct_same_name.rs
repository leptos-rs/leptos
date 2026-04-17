// A component function and a struct with the same name can coexist in the same
// scope because the companion module uses a `__` prefix (`__Foo`), avoiding a
// collision in the type namespace.

use leptos::prelude::*;

struct Foo {
    pub value: i32,
}

#[component]
fn Foo(foo: Foo) -> impl IntoView {
    let _ = foo;
    ()
}

#[component]
fn App() -> impl IntoView {
    view! {
        <Foo foo=Foo { value: 42 } />
    }
}

fn main() {}
