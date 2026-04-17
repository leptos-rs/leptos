use leptos::prelude::*;

// Component with multiple generic type parameters, all correctly provided.
// This should compile without errors.

#[component]
fn MultipleGenerics() -> impl IntoView {
    view! {
        <div>
            <Inner fun_a=|| true fun_b=|| 42/>
        </div>
    }
}

#[component]
fn Inner<F, G>(fun_a: F, fun_b: G) -> impl IntoView
where
    F: Fn() -> bool,
    G: Fn() -> i32,
{
    let _ = fun_a();
    let _ = fun_b();
    ()
}

fn main() {}
