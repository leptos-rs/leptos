use leptos::prelude::*;

// Wrong type for concrete prop `concrete` in a generic component.
// We expect the error to be on the value `true`.

#[component]
fn InvalidPropPassed() -> impl IntoView {
    view! {
        <div>
            <Inner concrete=true fun=|| true>
                "foo"
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
