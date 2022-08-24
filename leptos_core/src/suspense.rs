use crate as leptos;
use leptos_dom::{Child, Element, IntoChild};
use leptos_macro::Props;
use leptos_reactive::{provide_context, Scope, SuspenseContext};

#[derive(Props)]
pub struct SuspenseProps<F, G>
where
    F: IntoChild + Clone,
    G: Fn() -> Element,
{
    fallback: F,
    children: Vec<G>,
}

#[allow(non_snake_case)]
pub fn Suspense<F, C, G>(cx: Scope, mut props: SuspenseProps<F, G>) -> impl Fn() -> Child
where
    F: IntoChild + Clone,
    G: Fn() -> Element,
{
    let context = SuspenseContext::new(cx);

    // provide this SuspenseContext to any resources below it
    provide_context(cx, context);

    move || {
        if context.ready() || cx.transition_pending() {
            (props.children.iter().map(|child| (child)()))
                .collect::<Vec<_>>()
                .into_child(cx)
        } else {
            props.fallback.clone().into_child(cx)
        }
    }
}
