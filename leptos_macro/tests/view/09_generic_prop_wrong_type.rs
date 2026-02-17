use leptos::prelude::*;

// Wrong type for generic prop `generic_fun`. Primary error expected:
// - E0277 on the value `true` (from per-prop `__check_generic_fun()`)
// Secondary error (known limitation):
// - E0599 on the component name `Inner` (from `__check()` / PropsCheck)

#[component]
fn InvalidGenericPropPassed() -> impl IntoView {
    view! {
        <div>
            <Inner concrete_i32=42 generic_fun=true>
                "foo"
            </Inner>
        </div>
    }
}

#[component]
fn Inner<F>(
    concrete_i32: i32,
    generic_fun: F,
    children: ChildrenFn,
) -> impl IntoView
where
    F: Fn() -> bool,
{
    let _ = concrete_i32;
    let _ = generic_fun();
    children()
}

fn main() {}
