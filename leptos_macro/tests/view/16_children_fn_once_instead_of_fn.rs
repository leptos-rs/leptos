use leptos::prelude::*;

// Children closure captures a non-Copy variable (`non_copy`), making it FnOnce
// instead of the required Fn. Error expected on the `Inner` component name.
// We would like to "elevate" the "reason" why the children block is only Fn as the main error,
// because this would be of much more interest to the user, but that is not possible.
// The compiler error still notes the usage of `non_copy` as the reason though.

#[component]
fn FnOnceChildrenWhereFnChildrenWereExpected() -> impl IntoView {
    let non_copy = String::from("foo");

    view! {
        <div>
            <Inner concrete=42 fun=|| true>
                "foo"
                { non_copy }
            </Inner>
        </div>
    }
}

#[component]
fn Inner<F>(concrete: i32, fun: F, children: ChildrenFn) -> impl IntoView
where
    F: Fn() -> bool,
{
    let _ = concrete;
    let _ = fun();
    children()
}

fn main() {}
