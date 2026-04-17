use leptos::prelude::*;

// Two generic props are given wrong types.
// Each should produce its own independent error pair (E0277 + E0599).

#[component]
fn MultipleWrongType() -> impl IntoView {
    view! {
        <div>
            <Inner fun_a=true fun_b=42>
                "foo"
            </Inner>
        </div>
    }
}

#[component]
fn Inner<A, B>(
    fun_a: A,
    fun_b: B,
    children: ChildrenFn,
) -> impl IntoView
where
    A: Fn() -> bool,
    B: Fn() -> i32,
{
    let _ = fun_a();
    let _ = fun_b();
    children()
}

fn main() {}
