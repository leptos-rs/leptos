use leptos::prelude::*;

// Second of two generic props has wrong type (`|| true` instead of `|| 42`).
// Expected error:
// - E0271 on the closure value (compiler infers return type mismatch)
// - E0599 on the value `|| true` (from per-prop `__pass_fun_b()`)
// Note: The Fn hint does NOT appear because the compiler produces its own
// E0271 ("expected closure to return `i32`, but it returns `bool`") instead
// of the generic `on_unimplemented` E0277.

#[component]
fn MultipleGenericsSecondInvalid() -> impl IntoView {
    view! {
        <div>
            <Inner fun_a=|| true fun_b=|| true/>
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
