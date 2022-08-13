use crate as leptos;
use leptos_dom::{Child, IntoChild};
use leptos_macro::Props;
use leptos_reactive::{Scope, SuspenseContext};

#[derive(Props)]
pub struct SuspenseProps<F, C, G>
where
    F: IntoChild + Clone,
    C: IntoChild + Clone,
    G: Fn() -> C,
{
    fallback: F,
    children: G,
}

#[allow(non_snake_case)]
pub fn Suspense<F, C, G>(cx: Scope, props: SuspenseProps<F, C, G>) -> impl Fn() -> Child
where
    F: IntoChild + Clone,
    C: IntoChild + Clone,
    G: Fn() -> C,
{
    let context = SuspenseContext::new(cx);

    // provide this SuspenseContext to any resources below it
    cx.provide_context(context);

    move || {
        if context.ready() || cx.transition_pending() {
            (props.children)().into_child(cx)
        } else {
            props.fallback.clone().into_child(cx)
        }
    }
}
