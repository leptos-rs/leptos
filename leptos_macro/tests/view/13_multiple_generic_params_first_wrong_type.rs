use leptos::prelude::*;

// First of two generic props has wrong type (`true` instead of `|| true`).
// Primary error expected:
// - E0277 on the value `true` (from per-prop `__check_fun_a()`)
// Secondary error (known limitation):
// - E0599 on the component name `Inner` (from `__check()` / PropsCheck)

#[component]
fn MultipleGenericsFirstInvalid() -> impl IntoView {
    view! {
        <div>
            <Inner fun_a=true fun_b=|| 42/>
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
