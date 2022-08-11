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
pub fn Suspense<'a, F, C, G>(cx: Scope, props: SuspenseProps<F, C, G>) -> impl Fn() -> Child
where
    F: IntoChild + Clone,
    C: IntoChild + Clone,
    G: Fn() -> C,
{
    let context = SuspenseContext::new(cx);

    // provide this SuspenseContext to any resources below it
    cx.provide_context(context.clone());

    leptos_dom::log!("point A");
    move || {
        if context.ready() {
            leptos_dom::log!("point B");

            (props.children)().into_child(cx)
        } else {
            leptos_dom::log!("point C");
            props.fallback.clone().into_child(cx)
        }
    }
}
