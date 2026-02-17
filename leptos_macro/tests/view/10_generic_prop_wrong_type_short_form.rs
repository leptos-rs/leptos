use leptos::prelude::*;

// Same as test 09 but using the shorthand form for the generic prop.
// Primary error expected:
// - E0277 on the key `generic_fun` (shorthand has no separate value token)
// Secondary error (known limitation):
// - E0599 on the component name `Inner` (from `__check()` / PropsCheck)

#[component]
fn InvalidGenericPropPassed() -> impl IntoView {
    let generic_fun = true;

    view! {
        <div>
            <Inner concrete_i32=42 generic_fun>
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
