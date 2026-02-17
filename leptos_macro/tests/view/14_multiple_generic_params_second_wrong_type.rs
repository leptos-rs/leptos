use leptos::prelude::*;

// Second of two generic props has wrong type (`|| true` instead of `|| 42`).
// Primary error expected:
// - E0271 on the value `|| true` (from per-prop `__check_fun_b()`)
// Secondary error (known limitation):
// - E0599 on the component name `Inner` (from `__check()` / PropsCheck)

#[component]
fn MultipleGenericsFirstInvalid() -> impl IntoView {
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
